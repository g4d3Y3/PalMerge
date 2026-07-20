//! Read-only discovery and format probing.

mod container;

pub use container::{
    parse_header, read_header, validate_plz, CompressionKind, ContainerHeader, ContainerKind,
    DecodeSummary, DEFAULT_MAX_DECOMPRESSED_SIZE,
};
use palmerge_core::{fingerprint, ErrorCode, Fingerprint, PalError};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_DISCOVERED_FILES: usize = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveFormat {
    Gvas,
    PalworldPlz,
    PalworldPlm,
    Unknown,
}

impl SaveFormat {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gvas => "gvas",
            Self::PalworldPlz => "palworld_plz",
            Self::PalworldPlm => "palworld_plm",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Inspection {
    pub fingerprint: Fingerprint,
    pub format: SaveFormat,
    pub container: Option<ContainerHeader>,
    pub decoded: Option<DecodeSummary>,
}

/// Finds the known save files in a world directory without following links or
/// descending into arbitrary subdirectories.
pub fn discover(path: &Path) -> Result<Vec<PathBuf>, PalError> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if !path.is_dir() {
        return Err(PalError::new(
            ErrorCode::MissingSave,
            format!("path does not exist: {}", path.display()),
        ));
    }

    let mut files = Vec::new();
    for name in [
        "Level.sav",
        "LevelMeta.sav",
        "LocalData.sav",
        "WorldOption.sav",
    ] {
        push_regular_file(&mut files, path.join(name));
    }

    let players = path.join("Players");
    if players.is_dir() {
        for entry in fs::read_dir(&players).map_err(|error| {
            PalError::new(ErrorCode::Io, format!("{}: {error}", players.display()))
        })? {
            let entry = entry.map_err(|error| PalError::new(ErrorCode::Io, error.to_string()))?;
            if files.len() >= MAX_DISCOVERED_FILES {
                return Err(PalError::new(
                    ErrorCode::InvalidArguments,
                    format!("world contains more than {MAX_DISCOVERED_FILES} save files"),
                ));
            }
            let candidate = entry.path();
            if candidate
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("sav"))
            {
                push_regular_file(&mut files, candidate);
            }
        }
    }

    files.sort();
    if files.is_empty() {
        return Err(PalError::new(
            ErrorCode::MissingSave,
            format!("no recognized save files in {}", path.display()),
        ));
    }
    Ok(files)
}

fn push_regular_file(files: &mut Vec<PathBuf>, path: PathBuf) {
    if path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_file())
    {
        files.push(path);
    }
}

pub fn inspect(path: &Path) -> Result<Inspection, PalError> {
    let mut file = File::open(path)
        .map_err(|error| PalError::new(ErrorCode::Io, format!("{}: {error}", path.display())))?;
    let mut magic = [0_u8; 4];
    let count = file
        .read(&mut magic)
        .map_err(|error| PalError::new(ErrorCode::Io, format!("{}: {error}", path.display())))?;
    let container = read_header(path)?;
    let (format, decoded) = if count == magic.len() && &magic == b"GVAS" {
        (SaveFormat::Gvas, None)
    } else if let Some(header) = container {
        match header.kind {
            ContainerKind::Plz => (
                SaveFormat::PalworldPlz,
                Some(validate_plz(path, header, DEFAULT_MAX_DECOMPRESSED_SIZE)?),
            ),
            ContainerKind::Plm => (SaveFormat::PalworldPlm, None),
        }
    } else {
        (SaveFormat::Unknown, None)
    };
    Ok(Inspection {
        fingerprint: fingerprint(path)?,
        format,
        container,
        decoded,
    })
}

pub fn inspect_all(path: &Path) -> Result<Vec<Inspection>, PalError> {
    discover(path)?.iter().map(|file| inspect(file)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("palmerge-parser-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn discovers_world_files_deterministically() {
        let root = unique_dir();
        fs::create_dir_all(root.join("Players")).unwrap();
        fs::write(root.join("Level.sav"), b"GVASworld").unwrap();
        fs::write(root.join("Players/B.sav"), b"GVASb").unwrap();
        fs::write(root.join("Players/A.sav"), b"GVASa").unwrap();
        fs::write(root.join("Players/ignore.txt"), b"ignored").unwrap();

        let files = discover(&root).unwrap();
        assert_eq!(
            files,
            vec![
                root.join("Level.sav"),
                root.join("Players/A.sav"),
                root.join("Players/B.sav")
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recognizes_gvas_without_writing() {
        let root = unique_dir();
        fs::create_dir_all(&root).unwrap();
        let path = root.join("Level.sav");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"GVASpayload").unwrap();
        drop(file);

        let result = inspect(&path).unwrap();
        assert_eq!(result.format, SaveFormat::Gvas);
        assert!(result.container.is_none());
        assert_eq!(fs::read(&path).unwrap(), b"GVASpayload");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unknown_data_is_not_guessed() {
        let root = unique_dir();
        fs::create_dir_all(&root).unwrap();
        let path = root.join("Level.sav");
        fs::write(&path, b"not-a-save").unwrap();
        assert_eq!(inspect(&path).unwrap().format, SaveFormat::Unknown);
        fs::remove_dir_all(root).unwrap();
    }
}
