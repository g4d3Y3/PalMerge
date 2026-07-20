//! Bounded parsing of the metadata prefix shared by Unreal Engine GVAS saves.

use crate::container::{
    open_plz_stream, validate_plz, ContainerHeader, DecodeSummary, DEFAULT_MAX_DECOMPRESSED_SIZE,
};
use crate::properties::{parse_property_inventory, PropertyInventory};
use palmerge_core::{ErrorCode, PalError};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAX_STRING_CODE_UNITS: usize = 64 * 1024;
const MAX_CUSTOM_VERSIONS: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageVersion {
    pub ue4: u32,
    pub ue5: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
    pub build: u32,
    pub branch: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomVersion {
    pub guid: String,
    pub value: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GvasHeader {
    pub save_game_version: u32,
    pub package_version: PackageVersion,
    pub engine_version: EngineVersion,
    pub custom_format_version: Option<u32>,
    pub custom_versions: Vec<CustomVersion>,
    pub save_game_class: String,
}

pub fn read_gvas_header(
    path: &Path,
    container: Option<ContainerHeader>,
) -> Result<GvasHeader, PalError> {
    match container {
        Some(header) => {
            validate_plz(path, header, DEFAULT_MAX_DECOMPRESSED_SIZE)?;
            let mut reader = open_plz_stream(path, header)?;
            parse_gvas_header(&mut reader)
        }
        None => {
            let mut file = File::open(path).map_err(|error| {
                PalError::new(ErrorCode::Io, format!("{}: {error}", path.display()))
            })?;
            parse_gvas_header(&mut file)
        }
    }
}

pub fn read_gvas_inventory(
    path: &Path,
    container: Option<ContainerHeader>,
) -> Result<(GvasHeader, PropertyInventory), PalError> {
    match container {
        Some(header) => {
            validate_plz(path, header, DEFAULT_MAX_DECOMPRESSED_SIZE)?;
            let mut reader = open_plz_stream(path, header)?;
            parse_gvas_inventory(&mut reader)
        }
        None => {
            let mut file = File::open(path).map_err(|error| {
                PalError::new(ErrorCode::Io, format!("{}: {error}", path.display()))
            })?;
            parse_gvas_inventory(&mut file)
        }
    }
}

pub(crate) fn inspect_plz_gvas(
    path: &Path,
    header: ContainerHeader,
) -> Result<(DecodeSummary, GvasHeader, PropertyInventory), PalError> {
    let decoded = validate_plz(path, header, DEFAULT_MAX_DECOMPRESSED_SIZE)?;
    let mut reader = open_plz_stream(path, header)?;
    let (gvas, properties) = parse_gvas_inventory(&mut reader)?;
    Ok((decoded, gvas, properties))
}

fn parse_gvas_inventory(
    reader: &mut impl Read,
) -> Result<(GvasHeader, PropertyInventory), PalError> {
    let header = parse_gvas_header(reader)?;
    let inventory = parse_property_inventory(reader, &header)?;
    Ok((header, inventory))
}

pub fn parse_gvas_header(reader: &mut impl Read) -> Result<GvasHeader, PalError> {
    let mut magic = [0_u8; 4];
    read_exact(reader, &mut magic, "GVAS magic")?;
    if &magic != b"GVAS" {
        return Err(invalid("decoded payload does not begin with GVAS"));
    }

    let save_game_version = read_u32(reader, "save-game version")?;
    let package_version = PackageVersion {
        ue4: read_u32(reader, "UE4 package version")?,
        ue5: if save_game_version >= 3 && save_game_version != 34 {
            Some(read_u32(reader, "UE5 package version")?)
        } else {
            None
        },
    };
    let engine_version = EngineVersion {
        major: read_u16(reader, "engine major version")?,
        minor: read_u16(reader, "engine minor version")?,
        patch: read_u16(reader, "engine patch version")?,
        build: read_u32(reader, "engine build")?,
        branch: read_fstring(reader, "engine branch")?,
    };

    let (custom_format_version, custom_versions) =
        if (engine_version.major, engine_version.minor) >= (4, 12) {
            let format = read_u32(reader, "custom-version format")?;
            let count = read_u32(reader, "custom-version count")? as usize;
            if count > MAX_CUSTOM_VERSIONS {
                return Err(invalid(format!(
                    "custom-version count {count} exceeds limit {MAX_CUSTOM_VERSIONS}"
                )));
            }
            let mut versions = Vec::with_capacity(count);
            for _ in 0..count {
                let a = read_u32(reader, "custom-version GUID")?;
                let b = read_u32(reader, "custom-version GUID")?;
                let c = read_u32(reader, "custom-version GUID")?;
                let d = read_u32(reader, "custom-version GUID")?;
                versions.push(CustomVersion {
                    guid: format_guid(a, b, c, d),
                    value: read_i32(reader, "custom-version value")?,
                });
            }
            (Some(format), versions)
        } else {
            (None, Vec::new())
        };

    Ok(GvasHeader {
        save_game_version,
        package_version,
        engine_version,
        custom_format_version,
        custom_versions,
        save_game_class: read_fstring(reader, "save-game class")?,
    })
}

pub(crate) fn read_fstring(reader: &mut impl Read, field: &str) -> Result<String, PalError> {
    let length = read_i32(reader, field)?;
    if length == 0 {
        return Ok(String::new());
    }
    if length == i32::MIN {
        return Err(invalid(format!("invalid {field} length")));
    }
    let units = length.unsigned_abs() as usize;
    if units > MAX_STRING_CODE_UNITS {
        return Err(invalid(format!(
            "{field} length {units} exceeds limit {MAX_STRING_CODE_UNITS}"
        )));
    }

    if length > 0 {
        let mut bytes = vec![0_u8; units];
        read_exact(reader, &mut bytes, field)?;
        if bytes.pop() != Some(0) {
            return Err(invalid(format!("{field} is not null-terminated")));
        }
        String::from_utf8(bytes).map_err(|_| invalid(format!("{field} is not valid UTF-8")))
    } else {
        let mut chars = Vec::with_capacity(units.saturating_sub(1));
        for index in 0..units {
            let value = read_u16(reader, field)?;
            if index + 1 == units {
                if value != 0 {
                    return Err(invalid(format!("{field} is not null-terminated")));
                }
            } else {
                chars.push(value);
            }
        }
        String::from_utf16(&chars).map_err(|_| invalid(format!("{field} is not valid UTF-16")))
    }
}

fn read_u16(reader: &mut impl Read, field: &str) -> Result<u16, PalError> {
    let mut bytes = [0_u8; 2];
    read_exact(reader, &mut bytes, field)?;
    Ok(u16::from_le_bytes(bytes))
}

pub(crate) fn read_u32(reader: &mut impl Read, field: &str) -> Result<u32, PalError> {
    let mut bytes = [0_u8; 4];
    read_exact(reader, &mut bytes, field)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_i32(reader: &mut impl Read, field: &str) -> Result<i32, PalError> {
    let mut bytes = [0_u8; 4];
    read_exact(reader, &mut bytes, field)?;
    Ok(i32::from_le_bytes(bytes))
}

pub(crate) fn read_u8(reader: &mut impl Read, field: &str) -> Result<u8, PalError> {
    let mut byte = [0_u8; 1];
    read_exact(reader, &mut byte, field)?;
    Ok(byte[0])
}

pub(crate) fn read_exact(
    reader: &mut impl Read,
    bytes: &mut [u8],
    field: &str,
) -> Result<(), PalError> {
    reader
        .read_exact(bytes)
        .map_err(|error| invalid(format!("could not read {field}: {error}")))
}

pub(crate) fn format_guid(a: u32, b: u32, c: u32, d: u32) -> String {
    let b = b.to_le_bytes();
    let c = c.to_le_bytes();
    format!(
        "{a:08x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{d:08x}",
        b[3], b[2], b[1], b[0], c[3], c[2], c[1], c[0]
    )
}

fn invalid(message: impl Into<String>) -> PalError {
    PalError::new(ErrorCode::UnknownFormat, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn fstring(value: &str) -> Vec<u8> {
        let mut bytes = ((value.len() + 1) as i32).to_le_bytes().to_vec();
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
        bytes
    }

    fn fixture() -> Vec<u8> {
        let mut bytes = b"GVAS".to_vec();
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(&522_u32.to_le_bytes());
        bytes.extend_from_slice(&1009_u32.to_le_bytes());
        bytes.extend_from_slice(&5_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&123_u32.to_le_bytes());
        bytes.extend_from_slice(&fstring("++UE5+Release-5.1"));
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        for value in [0x12f88b9f_u32, 0x88754afc, 0xa67cd90c, 0x383abd29] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&7_i32.to_le_bytes());
        bytes.extend_from_slice(&fstring("/Script/Pal.PalWorldSaveGame"));
        bytes
    }

    #[test]
    fn parses_ue5_header_and_custom_versions() {
        let header = parse_gvas_header(&mut Cursor::new(fixture())).unwrap();
        assert_eq!(header.save_game_version, 3);
        assert_eq!(header.package_version.ue5, Some(1009));
        assert_eq!(header.engine_version.major, 5);
        assert_eq!(header.custom_versions.len(), 1);
        assert_eq!(header.custom_versions[0].value, 7);
        assert_eq!(header.save_game_class, "/Script/Pal.PalWorldSaveGame");
    }

    #[test]
    fn rejects_unbounded_and_truncated_strings() {
        let mut oversized = fixture();
        oversized[26..30].copy_from_slice(&((MAX_STRING_CODE_UNITS + 1) as i32).to_le_bytes());
        assert!(parse_gvas_header(&mut Cursor::new(oversized)).is_err());

        let mut truncated = fixture();
        truncated.truncate(32);
        assert!(parse_gvas_header(&mut Cursor::new(truncated)).is_err());
    }

    #[test]
    fn rejects_excessive_custom_version_count() {
        let mut bytes = fixture();
        let count_offset = 30 + "++UE5+Release-5.1".len() + 1 + 4;
        bytes[count_offset..count_offset + 4]
            .copy_from_slice(&((MAX_CUSTOM_VERSIONS + 1) as u32).to_le_bytes());
        assert!(parse_gvas_header(&mut Cursor::new(bytes)).is_err());
    }
}
