//! Shared, dependency-free primitives for PalMerge.

use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Stable machine-readable error identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    Io,
    InvalidArguments,
    MissingSave,
    UnknownFormat,
}

impl ErrorCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Io => "io_error",
            Self::InvalidArguments => "invalid_arguments",
            Self::MissingSave => "missing_save",
            Self::UnknownFormat => "unknown_format",
        }
    }
}

/// A localized user-facing error with a stable code.
#[derive(Debug)]
pub struct PalError {
    pub code: ErrorCode,
    pub message: String,
}

impl PalError {
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for PalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for PalError {}

/// Supported human-interface languages.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Locale {
    #[default]
    English,
    SimplifiedChinese,
}

impl Locale {
    pub fn parse(value: &str) -> Result<Self, PalError> {
        match value.to_ascii_lowercase().as_str() {
            "en" | "en-us" => Ok(Self::English),
            "zh" | "zh-cn" | "zh-hans" => Ok(Self::SimplifiedChinese),
            _ => Err(PalError::new(
                ErrorCode::InvalidArguments,
                format!("unsupported language: {value}"),
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageKey {
    Usage,
    Inspecting,
    File,
    Size,
    Sha256,
    Modified,
    Format,
    MissingPath,
    ReadFailed,
    UnknownFormat,
}

#[must_use]
pub const fn message(locale: Locale, key: MessageKey) -> &'static str {
    match (locale, key) {
        (Locale::English, MessageKey::Usage) => {
            "Usage: palmerge inspect <save-or-world-path> [--lang en|zh-CN] [--format text|json]"
        }
        (Locale::SimplifiedChinese, MessageKey::Usage) => {
            "用法：palmerge inspect <存档文件或世界目录> [--lang en|zh-CN] [--format text|json]"
        }
        (Locale::English, MessageKey::Inspecting) => "Read-only inspection result",
        (Locale::SimplifiedChinese, MessageKey::Inspecting) => "只读检查结果",
        (Locale::English, MessageKey::File) => "File",
        (Locale::SimplifiedChinese, MessageKey::File) => "文件",
        (Locale::English, MessageKey::Size) => "Size",
        (Locale::SimplifiedChinese, MessageKey::Size) => "大小",
        (Locale::English, MessageKey::Sha256) => "SHA-256",
        (Locale::SimplifiedChinese, MessageKey::Sha256) => "SHA-256",
        (Locale::English, MessageKey::Modified) => "Modified (Unix seconds)",
        (Locale::SimplifiedChinese, MessageKey::Modified) => "修改时间（Unix 秒）",
        (Locale::English, MessageKey::Format) => "Detected format",
        (Locale::SimplifiedChinese, MessageKey::Format) => "检测格式",
        (Locale::English, MessageKey::MissingPath) => "the save path does not exist",
        (Locale::SimplifiedChinese, MessageKey::MissingPath) => "存档路径不存在",
        (Locale::English, MessageKey::ReadFailed) => "could not read the save",
        (Locale::SimplifiedChinese, MessageKey::ReadFailed) => "无法读取存档",
        (Locale::English, MessageKey::UnknownFormat) => "unknown or unsupported save format",
        (Locale::SimplifiedChinese, MessageKey::UnknownFormat) => "未知或不支持的存档格式",
    }
}

/// Immutable file identity recorded during inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fingerprint {
    pub path: PathBuf,
    pub size: u64,
    pub modified_unix_seconds: Option<u64>,
    pub sha256: String,
}

pub fn fingerprint(path: &Path) -> Result<Fingerprint, PalError> {
    let metadata = path
        .metadata()
        .map_err(|error| PalError::new(ErrorCode::Io, format!("{}: {error}", path.display())))?;
    if !metadata.is_file() {
        return Err(PalError::new(
            ErrorCode::InvalidArguments,
            format!("not a file: {}", path.display()),
        ));
    }

    let mut file = File::open(path)
        .map_err(|error| PalError::new(ErrorCode::Io, format!("{}: {error}", path.display())))?;
    let sha256 = sha256_reader(&mut file)
        .map_err(|error| PalError::new(ErrorCode::Io, format!("{}: {error}", path.display())))?;
    let modified_unix_seconds = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_secs());

    Ok(Fingerprint {
        path: path.to_path_buf(),
        size: metadata.len(),
        modified_unix_seconds,
        sha256,
    })
}

fn sha256_reader(reader: &mut impl Read) -> io::Result<String> {
    let mut state = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        state.update(&buffer[..read]);
    }
    Ok(state.finalize_hex())
}

struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    bit_len: u64,
}

impl Sha256 {
    const fn new() -> Self {
        Self {
            state: [
                0x6a09_e667,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            buffer: [0; 64],
            buffer_len: 0,
            bit_len: 0,
        }
    }

    fn update(&mut self, mut input: &[u8]) {
        self.bit_len = self
            .bit_len
            .wrapping_add((input.len() as u64).wrapping_mul(8));
        if self.buffer_len > 0 {
            let needed = 64 - self.buffer_len;
            let take = needed.min(input.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&input[..take]);
            self.buffer_len += take;
            input = &input[take..];
            if self.buffer_len == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffer_len = 0;
            }
        }
        while input.len() >= 64 {
            let (block, rest) = input.split_at(64);
            self.compress(block.try_into().expect("64-byte block"));
            input = rest;
        }
        self.buffer[..input.len()].copy_from_slice(input);
        self.buffer_len = input.len();
    }

    fn finalize_hex(mut self) -> String {
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;
        if self.buffer_len > 56 {
            self.buffer[self.buffer_len..].fill(0);
            let block = self.buffer;
            self.compress(&block);
            self.buffer = [0; 64];
        } else {
            self.buffer[self.buffer_len..56].fill(0);
        }
        self.buffer[56..64].copy_from_slice(&self.bit_len.to_be_bytes());
        let block = self.buffer;
        self.compress(&block);
        self.state
            .iter()
            .map(|word| format!("{word:08x}"))
            .collect()
    }

    fn compress(&mut self, block: &[u8; 64]) {
        const K: [u32; 64] = [
            0x428a_2f98,
            0x7137_4491,
            0xb5c0_fbcf,
            0xe9b5_dba5,
            0x3956_c25b,
            0x59f1_11f1,
            0x923f_82a4,
            0xab1c_5ed5,
            0xd807_aa98,
            0x1283_5b01,
            0x2431_85be,
            0x550c_7dc3,
            0x72be_5d74,
            0x80de_b1fe,
            0x9bdc_06a7,
            0xc19b_f174,
            0xe49b_69c1,
            0xefbe_4786,
            0x0fc1_9dc6,
            0x240c_a1cc,
            0x2de9_2c6f,
            0x4a74_84aa,
            0x5cb0_a9dc,
            0x76f9_88da,
            0x983e_5152,
            0xa831_c66d,
            0xb003_27c8,
            0xbf59_7fc7,
            0xc6e0_0bf3,
            0xd5a7_9147,
            0x06ca_6351,
            0x1429_2967,
            0x27b7_0a85,
            0x2e1b_2138,
            0x4d2c_6dfc,
            0x5338_0d13,
            0x650a_7354,
            0x766a_0abb,
            0x81c2_c92e,
            0x9272_2c85,
            0xa2bf_e8a1,
            0xa81a_664b,
            0xc24b_8b70,
            0xc76c_51a3,
            0xd192_e819,
            0xd699_0624,
            0xf40e_3585,
            0x106a_a070,
            0x19a4_c116,
            0x1e37_6c08,
            0x2748_774c,
            0x34b0_bcb5,
            0x391c_0cb3,
            0x4ed8_aa4a,
            0x5b9c_ca4f,
            0x682e_6ff3,
            0x748f_82ee,
            0x78a5_636f,
            0x84c8_7814,
            0x8cc7_0208,
            0x90be_fffa,
            0xa450_6ceb,
            0xbef9_a3f7,
            0xc671_78f2,
        ];
        let mut w = [0_u32; 64];
        for (index, chunk) in block.chunks_exact(4).enumerate() {
            w[index] = u32::from_be_bytes(chunk.try_into().expect("four bytes"));
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_standard_vectors() {
        assert_eq!(
            sha256_reader(&mut &b""[..]).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_reader(&mut &b"abc"[..]).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn important_messages_are_localized() {
        assert_eq!(
            message(Locale::English, MessageKey::UnknownFormat),
            "unknown or unsupported save format"
        );
        assert_eq!(
            message(Locale::SimplifiedChinese, MessageKey::UnknownFormat),
            "未知或不支持的存档格式"
        );
    }
}
