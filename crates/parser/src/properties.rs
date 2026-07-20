//! Bounded, read-only inventory of legacy Unreal Engine property tags.

use crate::gvas::{format_guid, read_fstring, read_u32, read_u8, GvasHeader};
use palmerge_core::{ErrorCode, PalError};
use std::io::Read;

const MAX_TOP_LEVEL_PROPERTIES: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyInventory {
    pub properties: Vec<PropertyTag>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyTag {
    pub name: String,
    pub property_type: String,
    pub size: u32,
    pub array_index: u32,
    pub metadata: PropertyMetadata,
    pub property_guid: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PropertyMetadata {
    pub bool_value: Option<bool>,
    pub enum_type: Option<String>,
    pub inner_type: Option<String>,
    pub key_type: Option<String>,
    pub value_type: Option<String>,
    pub struct_type: Option<String>,
    pub struct_guid: Option<String>,
}

pub(crate) fn parse_property_inventory(
    reader: &mut impl Read,
    header: &GvasHeader,
) -> Result<PropertyInventory, PalError> {
    if (header.engine_version.major, header.engine_version.minor) >= (5, 4) {
        return Err(invalid(
            "UE 5.4 complete property tags are not supported yet",
        ));
    }

    let mut properties = Vec::new();
    loop {
        let name = read_fstring(reader, "property name")?;
        if name == "None" {
            return Ok(PropertyInventory { properties });
        }
        if properties.len() >= MAX_TOP_LEVEL_PROPERTIES {
            return Err(invalid(format!(
                "top-level property count exceeds limit {MAX_TOP_LEVEL_PROPERTIES}"
            )));
        }

        let property_type = read_fstring(reader, "property type")?;
        let size = read_u32(reader, "property size")?;
        let array_index = read_u32(reader, "property array index")?;
        let metadata = read_metadata(reader, &property_type)?;
        let property_guid = if (header.engine_version.major, header.engine_version.minor) >= (4, 12)
        {
            read_optional_guid(reader, "property GUID")?
        } else {
            None
        };
        discard_exact(reader, u64::from(size), &name)?;
        properties.push(PropertyTag {
            name,
            property_type,
            size,
            array_index,
            metadata,
            property_guid,
        });
    }
}

fn read_metadata(
    reader: &mut impl Read,
    property_type: &str,
) -> Result<PropertyMetadata, PalError> {
    let mut metadata = PropertyMetadata::default();
    match property_type {
        "BoolProperty" => metadata.bool_value = Some(read_u8(reader, "bool property value")? != 0),
        "ByteProperty" => {
            let value = read_fstring(reader, "byte enum type")?;
            metadata.enum_type = (value != "None").then_some(value);
        }
        "EnumProperty" => metadata.enum_type = Some(read_fstring(reader, "enum type")?),
        "ArrayProperty" => metadata.inner_type = Some(read_fstring(reader, "array inner type")?),
        "SetProperty" => metadata.key_type = Some(read_fstring(reader, "set key type")?),
        "MapProperty" => {
            metadata.key_type = Some(read_fstring(reader, "map key type")?);
            metadata.value_type = Some(read_fstring(reader, "map value type")?);
        }
        "StructProperty" => {
            metadata.struct_type = Some(read_fstring(reader, "struct type")?);
            metadata.struct_guid = Some(read_guid(reader, "struct GUID")?);
        }
        "IntProperty"
        | "Int8Property"
        | "Int16Property"
        | "Int64Property"
        | "UInt8Property"
        | "UInt16Property"
        | "UInt32Property"
        | "UInt64Property"
        | "FloatProperty"
        | "DoubleProperty"
        | "StrProperty"
        | "ObjectProperty"
        | "InterfaceProperty"
        | "FieldPathProperty"
        | "SoftObjectProperty"
        | "NameProperty"
        | "TextProperty"
        | "DelegateProperty"
        | "MulticastDelegateProperty"
        | "MulticastInlineDelegateProperty"
        | "MulticastSparseDelegateProperty" => {}
        other => return Err(invalid(format!("unsupported property tag type: {other}"))),
    }
    Ok(metadata)
}

fn read_optional_guid(reader: &mut impl Read, field: &str) -> Result<Option<String>, PalError> {
    match read_u8(reader, field)? {
        0 => Ok(None),
        1 => Ok(Some(read_guid(reader, field)?)),
        value => Err(invalid(format!("invalid {field} presence flag {value}"))),
    }
}

fn read_guid(reader: &mut impl Read, field: &str) -> Result<String, PalError> {
    let a = read_u32(reader, field)?;
    let b = read_u32(reader, field)?;
    let c = read_u32(reader, field)?;
    let d = read_u32(reader, field)?;
    Ok(format_guid(a, b, c, d))
}

fn discard_exact(reader: &mut impl Read, mut remaining: u64, name: &str) -> Result<(), PalError> {
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        let wanted = remaining.min(buffer.len() as u64) as usize;
        let count = reader
            .read(&mut buffer[..wanted])
            .map_err(|error| invalid(format!("could not read property {name}: {error}")))?;
        if count == 0 {
            return Err(invalid(format!("property {name} data is truncated")));
        }
        remaining -= count as u64;
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> PalError {
    PalError::new(ErrorCode::UnknownFormat, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gvas::{EngineVersion, PackageVersion};
    use std::io::Cursor;

    fn fstring(value: &str) -> Vec<u8> {
        let mut bytes = ((value.len() + 1) as i32).to_le_bytes().to_vec();
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
        bytes
    }

    fn header() -> GvasHeader {
        GvasHeader {
            save_game_version: 3,
            package_version: PackageVersion {
                ue4: 522,
                ue5: Some(1009),
            },
            engine_version: EngineVersion {
                major: 5,
                minor: 1,
                patch: 1,
                build: 0,
                branch: String::new(),
            },
            custom_format_version: Some(3),
            custom_versions: Vec::new(),
            save_game_class: "/Script/Pal.PalWorldSaveGame".to_owned(),
        }
    }

    fn property(name: &str, property_type: &str, metadata: &[u8], value: &[u8]) -> Vec<u8> {
        let mut bytes = fstring(name);
        bytes.extend_from_slice(&fstring(property_type));
        bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(metadata);
        bytes.push(0);
        bytes.extend_from_slice(value);
        bytes
    }

    #[test]
    fn inventories_legacy_property_tags_and_skips_values() {
        let mut bytes = property("Score", "IntProperty", &[], &42_i32.to_le_bytes());
        bytes.extend_from_slice(&property(
            "Items",
            "ArrayProperty",
            &fstring("StructProperty"),
            &[1, 2, 3, 4],
        ));
        bytes.extend_from_slice(&fstring("None"));

        let inventory = parse_property_inventory(&mut Cursor::new(bytes), &header()).unwrap();
        assert_eq!(inventory.properties.len(), 2);
        assert_eq!(inventory.properties[0].name, "Score");
        assert_eq!(
            inventory.properties[1].metadata.inner_type.as_deref(),
            Some("StructProperty")
        );
    }

    #[test]
    fn rejects_unknown_tags_and_truncated_values() {
        let mut unknown = property("Mystery", "FutureProperty", &[], &[]);
        unknown.extend_from_slice(&fstring("None"));
        assert!(parse_property_inventory(&mut Cursor::new(unknown), &header()).is_err());

        let truncated = property("Score", "IntProperty", &[], &[1, 2]);
        let mut declared_larger = truncated;
        let size_offset = fstring("Score").len() + fstring("IntProperty").len();
        declared_larger[size_offset..size_offset + 4].copy_from_slice(&4_u32.to_le_bytes());
        assert!(parse_property_inventory(&mut Cursor::new(declared_larger), &header()).is_err());
    }

    #[test]
    fn rejects_complete_property_tags_until_supported() {
        let mut newer = header();
        newer.engine_version.minor = 4;
        assert!(parse_property_inventory(&mut Cursor::new(Vec::new()), &newer).is_err());
    }
}
