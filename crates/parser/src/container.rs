//! Conservative parsing and read-only validation of Palworld save containers.

use flate2::read::ZlibDecoder;
use palmerge_core::{ErrorCode, PalError};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub const DEFAULT_MAX_DECOMPRESSED_SIZE: u64 = 2 * 1024 * 1024 * 1024;
const STANDARD_HEADER_SIZE: u64 = 12;
const CHUNKED_HEADER_SIZE: u64 = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContainerKind {
    Plz,
    Plm,
}

impl ContainerKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plz => "plz",
            Self::Plm => "plm",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionKind {
    SingleZlib,
    DoubleZlib,
    Oodle,
    Unsupported,
}

impl CompressionKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SingleZlib => "single_zlib",
            Self::DoubleZlib => "double_zlib",
            Self::Oodle => "oodle",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContainerHeader {
    pub kind: ContainerKind,
    pub compression: CompressionKind,
    pub save_type: u8,
    pub uncompressed_len: u64,
    pub compressed_len: u64,
    pub payload_offset: u64,
    pub chunk_wrapped: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeSummary {
    pub decoded_len: u64,
    pub embedded_gvas: bool,
}

/// Parses only recognized Palworld headers. Unknown bytes remain unknown.
pub fn parse_header(prefix: &[u8]) -> Result<Option<ContainerHeader>, PalError> {
    if prefix.len() < STANDARD_HEADER_SIZE as usize {
        return Ok(None);
    }
    let mut offset = 0_usize;
    let mut chunk_wrapped = false;
    if &prefix[8..11] == b"CNK" {
        if prefix.len() < CHUNKED_HEADER_SIZE as usize {
            return Err(invalid("truncated CNK container header"));
        }
        offset = 12;
        chunk_wrapped = true;
    }

    let magic = &prefix[offset + 8..offset + 11];
    let kind = match magic {
        b"PlZ" => ContainerKind::Plz,
        b"PlM" => ContainerKind::Plm,
        _ => return Ok(None),
    };
    let save_type = prefix[offset + 11];
    let compression = match (kind, save_type) {
        (ContainerKind::Plz, 0x31) => CompressionKind::SingleZlib,
        (ContainerKind::Plz, 0x32) => CompressionKind::DoubleZlib,
        (ContainerKind::Plm, 0x31) => CompressionKind::Oodle,
        _ => CompressionKind::Unsupported,
    };
    Ok(Some(ContainerHeader {
        kind,
        compression,
        save_type,
        uncompressed_len: u32::from_le_bytes(
            prefix[offset..offset + 4].try_into().expect("four bytes"),
        )
        .into(),
        compressed_len: u32::from_le_bytes(
            prefix[offset + 4..offset + 8]
                .try_into()
                .expect("four bytes"),
        )
        .into(),
        payload_offset: if chunk_wrapped {
            CHUNKED_HEADER_SIZE
        } else {
            STANDARD_HEADER_SIZE
        },
        chunk_wrapped,
    }))
}

pub fn read_header(path: &Path) -> Result<Option<ContainerHeader>, PalError> {
    let mut file = File::open(path)
        .map_err(|error| PalError::new(ErrorCode::Io, format!("{}: {error}", path.display())))?;
    let mut prefix = [0_u8; CHUNKED_HEADER_SIZE as usize];
    let count = file
        .read(&mut prefix)
        .map_err(|error| PalError::new(ErrorCode::Io, format!("{}: {error}", path.display())))?;
    parse_header(&prefix[..count])
}

/// Streams decompression and validation without storing the decoded save in memory.
pub fn validate_plz(
    path: &Path,
    header: ContainerHeader,
    max_decompressed_size: u64,
) -> Result<DecodeSummary, PalError> {
    if header.kind != ContainerKind::Plz {
        return Err(PalError::new(
            ErrorCode::UnknownFormat,
            "PlM/Oodle decompression is not supported",
        ));
    }
    if header.uncompressed_len > max_decompressed_size {
        return Err(PalError::new(
            ErrorCode::InvalidArguments,
            format!(
                "declared output size {} exceeds limit {max_decompressed_size}",
                header.uncompressed_len
            ),
        ));
    }

    let mut file = File::open(path)
        .map_err(|error| PalError::new(ErrorCode::Io, format!("{}: {error}", path.display())))?;
    let file_len = file
        .metadata()
        .map_err(|error| PalError::new(ErrorCode::Io, error.to_string()))?
        .len();
    if file_len < header.payload_offset {
        return Err(invalid("container payload is missing"));
    }
    let payload_len = file_len - header.payload_offset;
    file.seek(SeekFrom::Start(header.payload_offset))
        .map_err(|error| PalError::new(ErrorCode::Io, error.to_string()))?;

    let summary = match header.compression {
        CompressionKind::SingleZlib => {
            if payload_len != header.compressed_len {
                return Err(invalid("compressed length does not match payload"));
            }
            let mut decoder = ZlibDecoder::new(file.take(payload_len));
            consume(&mut decoder, max_decompressed_size)?
        }
        CompressionKind::DoubleZlib => {
            let outer = ZlibDecoder::new(file.take(payload_len));
            let mut inner = ZlibDecoder::new(outer);
            let summary = consume(&mut inner, max_decompressed_size)?;
            if inner.get_ref().total_out() != header.compressed_len {
                return Err(invalid("inner compressed length does not match header"));
            }
            summary
        }
        CompressionKind::Oodle | CompressionKind::Unsupported => {
            return Err(PalError::new(
                ErrorCode::UnknownFormat,
                format!("unsupported compression type 0x{:02x}", header.save_type),
            ));
        }
    };
    if summary.decoded_len != header.uncompressed_len {
        return Err(invalid("uncompressed length does not match header"));
    }
    if !summary.embedded_gvas {
        return Err(invalid("decoded payload does not begin with GVAS"));
    }
    Ok(summary)
}

fn consume(reader: &mut impl Read, limit: u64) -> Result<DecodeSummary, PalError> {
    let mut buffer = [0_u8; 64 * 1024];
    let mut prefix = [0_u8; 4];
    let mut prefix_len = 0_usize;
    let mut decoded_len = 0_u64;
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| invalid(format!("zlib decompression failed: {error}")))?;
        if count == 0 {
            break;
        }
        decoded_len = decoded_len
            .checked_add(count as u64)
            .ok_or_else(|| invalid("decoded size overflow"))?;
        if decoded_len > limit {
            return Err(PalError::new(
                ErrorCode::InvalidArguments,
                format!("decoded data exceeds limit {limit}"),
            ));
        }
        if prefix_len < prefix.len() {
            let take = (prefix.len() - prefix_len).min(count);
            prefix[prefix_len..prefix_len + take].copy_from_slice(&buffer[..take]);
            prefix_len += take;
        }
    }
    Ok(DecodeSummary {
        decoded_len,
        embedded_gvas: prefix_len == prefix.len() && &prefix == b"GVAS",
    })
}

fn invalid(message: impl Into<String>) -> PalError {
    PalError::new(ErrorCode::UnknownFormat, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn zlib(data: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    fn save(data: &[u8], save_type: u8, chunked: bool) -> (PathBuf, ContainerHeader) {
        let inner = zlib(data);
        let payload = if save_type == 0x32 {
            zlib(&inner)
        } else {
            inner.clone()
        };
        let mut bytes = Vec::new();
        if chunked {
            bytes.extend_from_slice(&0_u32.to_le_bytes());
            bytes.extend_from_slice(&0_u32.to_le_bytes());
            bytes.extend_from_slice(b"CNK\0");
        }
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(
            &((if save_type == 0x32 {
                inner.len()
            } else {
                payload.len()
            }) as u32)
                .to_le_bytes(),
        );
        bytes.extend_from_slice(b"PlZ");
        bytes.push(save_type);
        bytes.extend_from_slice(&payload);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "palmerge-container-{}-{nonce}.sav",
            std::process::id()
        ));
        fs::write(&path, bytes).unwrap();
        let header = read_header(&path).unwrap().unwrap();
        (path, header)
    }

    #[test]
    fn validates_single_and_double_zlib() {
        for save_type in [0x31, 0x32] {
            let (path, header) = save(b"GVASpayload", save_type, false);
            let summary = validate_plz(&path, header, 1024).unwrap();
            assert_eq!(summary.decoded_len, 11);
            assert!(summary.embedded_gvas);
            fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn parses_cnk_wrapper() {
        let (path, header) = save(b"GVASchunked", 0x31, true);
        assert!(header.chunk_wrapped);
        assert_eq!(header.payload_offset, 24);
        validate_plz(&path, header, 1024).unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn recognizes_plm_without_treating_it_as_zlib() {
        let mut prefix = [0_u8; 12];
        prefix[8..11].copy_from_slice(b"PlM");
        prefix[11] = 0x31;
        let header = parse_header(&prefix).unwrap().unwrap();
        assert_eq!(header.kind, ContainerKind::Plm);
        assert_eq!(header.compression, CompressionKind::Oodle);
    }

    #[test]
    fn enforces_declared_output_limit_before_decompression() {
        let (path, mut header) = save(b"GVASpayload", 0x31, false);
        header.uncompressed_len = 2048;
        let error = validate_plz(&path, header, 1024).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidArguments);
        fs::remove_file(path).unwrap();
    }
}
