//! Bounded recursive decoding for legacy GVAS property values.

use crate::gvas::{format_guid, read_exact, read_fstring, read_u32, read_u8, GvasHeader};
use crate::properties::{read_property_tag, PropertyTag};
use crate::schema::{is_opaque_custom_property, struct_hint, StructHint};
use palmerge_core::{ErrorCode, PalError};
use std::io::Read;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeLimits {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_collection_entries: usize,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_nodes: 1_000_000,
            max_collection_entries: 250_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecodedProperty {
    pub tag: PropertyTag,
    pub value: DecodedValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapEntry {
    pub key: DecodedValue,
    pub value: DecodedValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DecodedValue {
    Bool(bool),
    Signed(i64),
    Unsigned(u64),
    Float32(u32),
    Float64(u64),
    String(String),
    Guid(String),
    Struct(Vec<DecodedProperty>),
    Array(Vec<DecodedValue>),
    Set(Vec<DecodedValue>),
    Map(Vec<MapEntry>),
    Opaque { bytes: u32, kind: OpaqueKind },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpaqueKind {
    NativeStruct,
    UnsupportedProperty,
    PalworldCustomProperty,
}

struct Budget {
    remaining_nodes: usize,
    limits: DecodeLimits,
}

pub fn decode_properties(
    reader: &mut impl Read,
    header: &GvasHeader,
    limits: DecodeLimits,
) -> Result<Vec<DecodedProperty>, PalError> {
    if (header.engine_version.major, header.engine_version.minor) >= (5, 4) {
        return Err(invalid(
            "UE 5.4 complete property tags are not supported yet",
        ));
    }
    let mut budget = Budget {
        remaining_nodes: limits.max_nodes,
        limits,
    };
    decode_property_list(reader, header, "", 0, &mut budget)
}

fn decode_property_list(
    reader: &mut dyn Read,
    header: &GvasHeader,
    path: &str,
    depth: usize,
    budget: &mut Budget,
) -> Result<Vec<DecodedProperty>, PalError> {
    check_depth(depth, budget)?;
    let mut properties = Vec::new();
    loop {
        let tag = match read_property_tag(reader, header)? {
            Some(tag) => tag,
            None => return Ok(properties),
        };
        take_node(budget)?;
        let property_path = format!("{path}.{}", tag.name);
        let mut value_reader = (&mut *reader).take(u64::from(tag.size));
        let value = decode_tag_value(
            &mut value_reader,
            header,
            &property_path,
            &tag,
            depth + 1,
            budget,
        )?;
        if value_reader.limit() != 0 {
            return Err(invalid(format!(
                "property {} left {} undecoded bytes",
                tag.name,
                value_reader.limit()
            )));
        }
        properties.push(DecodedProperty { tag, value });
    }
}

fn decode_tag_value(
    reader: &mut dyn Read,
    header: &GvasHeader,
    path: &str,
    tag: &PropertyTag,
    depth: usize,
    budget: &mut Budget,
) -> Result<DecodedValue, PalError> {
    if is_opaque_custom_property(path) {
        return drain_opaque(reader, tag.size, OpaqueKind::PalworldCustomProperty);
    }
    match tag.property_type.as_str() {
        "BoolProperty" => Ok(DecodedValue::Bool(
            tag.metadata
                .bool_value
                .ok_or_else(|| invalid("bool tag is missing its value"))?,
        )),
        "Int8Property" => Ok(DecodedValue::Signed(i64::from(read_i8(reader, path)?))),
        "Int16Property" => Ok(DecodedValue::Signed(i64::from(read_i16(reader, path)?))),
        "IntProperty" => Ok(DecodedValue::Signed(i64::from(read_i32(reader, path)?))),
        "Int64Property" => Ok(DecodedValue::Signed(read_i64(reader, path)?)),
        "UInt8Property" => Ok(DecodedValue::Unsigned(u64::from(read_u8(reader, path)?))),
        "UInt16Property" => Ok(DecodedValue::Unsigned(u64::from(read_u16(reader, path)?))),
        "UInt32Property" => Ok(DecodedValue::Unsigned(u64::from(read_u32(reader, path)?))),
        "UInt64Property" => Ok(DecodedValue::Unsigned(read_u64(reader, path)?)),
        "FloatProperty" => Ok(DecodedValue::Float32(read_u32(reader, path)?)),
        "FixedPoint64Property" => Ok(DecodedValue::Signed(i64::from(read_i32(reader, path)?))),
        "DoubleProperty" => Ok(DecodedValue::Float64(read_u64(reader, path)?)),
        "StrProperty" | "NameProperty" | "ObjectProperty" | "InterfaceProperty" => {
            Ok(DecodedValue::String(read_fstring(reader, path)?))
        }
        "ByteProperty" => {
            if tag.metadata.enum_type.is_some() {
                Ok(DecodedValue::String(read_fstring(reader, path)?))
            } else {
                Ok(DecodedValue::Unsigned(u64::from(read_u8(reader, path)?)))
            }
        }
        "EnumProperty" => Ok(DecodedValue::String(read_fstring(reader, path)?)),
        "StructProperty" => decode_struct(
            reader,
            header,
            path,
            tag.metadata.struct_type.as_deref(),
            depth,
            budget,
        ),
        "ArrayProperty" => decode_array(reader, header, path, tag, depth, budget),
        "SetProperty" => decode_set(reader, header, path, tag, depth, budget),
        "MapProperty" => decode_map(reader, header, path, tag, depth, budget),
        _ => drain_opaque(reader, tag.size, OpaqueKind::UnsupportedProperty),
    }
}

fn decode_struct(
    reader: &mut dyn Read,
    header: &GvasHeader,
    path: &str,
    struct_type: Option<&str>,
    depth: usize,
    budget: &mut Budget,
) -> Result<DecodedValue, PalError> {
    match struct_type {
        Some("Guid") => Ok(DecodedValue::Guid(read_guid(reader, path)?)),
        Some(value) if is_native_struct(value) => {
            drain_opaque_unknown(reader, OpaqueKind::NativeStruct)
        }
        Some(_) | None => Ok(DecodedValue::Struct(decode_property_list(
            reader, header, path, depth, budget,
        )?)),
    }
}

fn decode_array(
    reader: &mut dyn Read,
    header: &GvasHeader,
    path: &str,
    tag: &PropertyTag,
    depth: usize,
    budget: &mut Budget,
) -> Result<DecodedValue, PalError> {
    let count = checked_count(read_u32(reader, path)?, budget)?;
    let inner = tag
        .metadata
        .inner_type
        .as_deref()
        .ok_or_else(|| invalid(format!("array {path} has no inner type")))?;
    let struct_type = if inner == "StructProperty" {
        let inner_tag = read_property_tag(reader, header)?
            .ok_or_else(|| invalid(format!("array {path} is missing its inner struct tag")))?;
        if inner_tag.property_type != "StructProperty" {
            return Err(invalid(format!(
                "array {path} inner tag is {} instead of StructProperty",
                inner_tag.property_type
            )));
        }
        inner_tag.metadata.struct_type
    } else {
        None
    };

    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        take_node(budget)?;
        values.push(decode_typed_value(
            reader,
            header,
            path,
            inner,
            struct_type.as_deref(),
            depth + 1,
            budget,
        )?);
    }
    Ok(DecodedValue::Array(values))
}

fn decode_set(
    reader: &mut dyn Read,
    header: &GvasHeader,
    path: &str,
    tag: &PropertyTag,
    depth: usize,
    budget: &mut Budget,
) -> Result<DecodedValue, PalError> {
    let removed = read_u32(reader, path)?;
    if removed != 0 {
        return Err(invalid(format!(
            "set {path} contains unsupported removed-template entries"
        )));
    }
    let count = checked_count(read_u32(reader, path)?, budget)?;
    let value_type = tag
        .metadata
        .key_type
        .as_deref()
        .ok_or_else(|| invalid(format!("set {path} has no value type")))?;
    let struct_type = collection_struct_type(path, value_type)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        take_node(budget)?;
        values.push(decode_typed_value(
            reader,
            header,
            path,
            value_type,
            struct_type,
            depth + 1,
            budget,
        )?);
    }
    Ok(DecodedValue::Set(values))
}

fn decode_map(
    reader: &mut dyn Read,
    header: &GvasHeader,
    path: &str,
    tag: &PropertyTag,
    depth: usize,
    budget: &mut Budget,
) -> Result<DecodedValue, PalError> {
    let key_type = tag
        .metadata
        .key_type
        .as_deref()
        .ok_or_else(|| invalid(format!("map {path} has no key type")))?;
    let value_type = tag
        .metadata
        .value_type
        .as_deref()
        .ok_or_else(|| invalid(format!("map {path} has no value type")))?;
    let key_path = format!("{path}.Key");
    let value_path = format!("{path}.Value");
    let key_struct = collection_struct_type(&key_path, key_type)?;
    let value_struct = collection_struct_type(&value_path, value_type)?;

    let removed = checked_count(read_u32(reader, path)?, budget)?;
    for _ in 0..removed {
        take_node(budget)?;
        let _ = decode_typed_value(
            reader,
            header,
            &key_path,
            key_type,
            key_struct,
            depth + 1,
            budget,
        )?;
    }

    let count = checked_count(read_u32(reader, path)?, budget)?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        take_node(budget)?;
        let key = decode_typed_value(
            reader,
            header,
            &key_path,
            key_type,
            key_struct,
            depth + 1,
            budget,
        )?;
        let value = decode_typed_value(
            reader,
            header,
            &value_path,
            value_type,
            value_struct,
            depth + 1,
            budget,
        )?;
        entries.push(MapEntry { key, value });
    }
    Ok(DecodedValue::Map(entries))
}

fn decode_typed_value(
    reader: &mut dyn Read,
    header: &GvasHeader,
    path: &str,
    property_type: &str,
    struct_type: Option<&str>,
    depth: usize,
    budget: &mut Budget,
) -> Result<DecodedValue, PalError> {
    check_depth(depth, budget)?;
    match property_type {
        "StructProperty" => decode_struct(reader, header, path, struct_type, depth, budget),
        "Int8Property" => Ok(DecodedValue::Signed(i64::from(read_i8(reader, path)?))),
        "Int16Property" => Ok(DecodedValue::Signed(i64::from(read_i16(reader, path)?))),
        "IntProperty" => Ok(DecodedValue::Signed(i64::from(read_i32(reader, path)?))),
        "Int64Property" => Ok(DecodedValue::Signed(read_i64(reader, path)?)),
        "UInt8Property" | "ByteProperty" => {
            Ok(DecodedValue::Unsigned(u64::from(read_u8(reader, path)?)))
        }
        "UInt16Property" => Ok(DecodedValue::Unsigned(u64::from(read_u16(reader, path)?))),
        "UInt32Property" => Ok(DecodedValue::Unsigned(u64::from(read_u32(reader, path)?))),
        "UInt64Property" => Ok(DecodedValue::Unsigned(read_u64(reader, path)?)),
        "FloatProperty" => Ok(DecodedValue::Float32(read_u32(reader, path)?)),
        "FixedPoint64Property" => Ok(DecodedValue::Signed(i64::from(read_i32(reader, path)?))),
        "DoubleProperty" => Ok(DecodedValue::Float64(read_u64(reader, path)?)),
        "BoolProperty" => Ok(DecodedValue::Bool(read_u8(reader, path)? != 0)),
        "StrProperty" | "NameProperty" | "ObjectProperty" | "InterfaceProperty"
        | "EnumProperty" => Ok(DecodedValue::String(read_fstring(reader, path)?)),
        other => Err(invalid(format!(
            "unsupported collection value type {other} at {path}"
        ))),
    }
}

fn collection_struct_type(
    path: &str,
    property_type: &str,
) -> Result<Option<&'static str>, PalError> {
    if property_type != "StructProperty" {
        return Ok(None);
    }
    match struct_hint(path) {
        Some(StructHint::Guid) => Ok(Some("Guid")),
        Some(StructHint::Properties) => Ok(None),
        None => Err(invalid(format!(
            "missing Palworld struct type hint for {path}"
        ))),
    }
}

fn is_native_struct(value: &str) -> bool {
    matches!(
        value,
        "DateTime"
            | "Timespan"
            | "Vector2D"
            | "Vector"
            | "Vector4"
            | "IntVector"
            | "Box"
            | "Box2D"
            | "IntPoint"
            | "Quat"
            | "LinearColor"
            | "Color"
            | "Rotator"
            | "SoftObjectPath"
            | "SoftClassPath"
            | "GameplayTagContainer"
            | "UniqueNetIdRepl"
    )
}

fn checked_count(value: u32, budget: &Budget) -> Result<usize, PalError> {
    let value = value as usize;
    if value > budget.limits.max_collection_entries {
        return Err(invalid(format!(
            "collection count {value} exceeds limit {}",
            budget.limits.max_collection_entries
        )));
    }
    Ok(value)
}

fn take_node(budget: &mut Budget) -> Result<(), PalError> {
    budget.remaining_nodes = budget
        .remaining_nodes
        .checked_sub(1)
        .ok_or_else(|| invalid("decoded node limit exceeded"))?;
    Ok(())
}

fn check_depth(depth: usize, budget: &Budget) -> Result<(), PalError> {
    if depth > budget.limits.max_depth {
        return Err(invalid(format!(
            "property nesting depth {depth} exceeds limit {}",
            budget.limits.max_depth
        )));
    }
    Ok(())
}

fn drain_opaque(
    reader: &mut dyn Read,
    bytes: u32,
    kind: OpaqueKind,
) -> Result<DecodedValue, PalError> {
    drain(reader)?;
    Ok(DecodedValue::Opaque { bytes, kind })
}

fn drain_opaque_unknown(reader: &mut dyn Read, kind: OpaqueKind) -> Result<DecodedValue, PalError> {
    let bytes = drain(reader)?;
    Ok(DecodedValue::Opaque {
        bytes: u32::try_from(bytes).map_err(|_| invalid("opaque value size overflow"))?,
        kind,
    })
}

fn drain(reader: &mut dyn Read) -> Result<u64, PalError> {
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| invalid(format!("could not skip opaque value: {error}")))?;
        if count == 0 {
            return Ok(total);
        }
        total = total
            .checked_add(count as u64)
            .ok_or_else(|| invalid("opaque value size overflow"))?;
    }
}

fn read_guid(reader: &mut dyn Read, field: &str) -> Result<String, PalError> {
    Ok(format_guid(
        read_u32(reader, field)?,
        read_u32(reader, field)?,
        read_u32(reader, field)?,
        read_u32(reader, field)?,
    ))
}

fn read_i8(reader: &mut dyn Read, field: &str) -> Result<i8, PalError> {
    Ok(read_u8(reader, field)? as i8)
}

fn read_u16(reader: &mut dyn Read, field: &str) -> Result<u16, PalError> {
    let mut bytes = [0_u8; 2];
    read_exact(reader, &mut bytes, field)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_i16(reader: &mut dyn Read, field: &str) -> Result<i16, PalError> {
    let mut bytes = [0_u8; 2];
    read_exact(reader, &mut bytes, field)?;
    Ok(i16::from_le_bytes(bytes))
}

fn read_i32(reader: &mut dyn Read, field: &str) -> Result<i32, PalError> {
    let mut bytes = [0_u8; 4];
    read_exact(reader, &mut bytes, field)?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_u64(reader: &mut dyn Read, field: &str) -> Result<u64, PalError> {
    let mut bytes = [0_u8; 8];
    read_exact(reader, &mut bytes, field)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_i64(reader: &mut dyn Read, field: &str) -> Result<i64, PalError> {
    let mut bytes = [0_u8; 8];
    read_exact(reader, &mut bytes, field)?;
    Ok(i64::from_le_bytes(bytes))
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

    fn tag(name: &str, property_type: &str, metadata: &[u8], value: &[u8]) -> Vec<u8> {
        let mut bytes = fstring(name);
        bytes.extend_from_slice(&fstring(property_type));
        bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(metadata);
        bytes.push(0);
        bytes.extend_from_slice(value);
        bytes
    }

    fn struct_metadata(struct_type: &str) -> Vec<u8> {
        let mut bytes = fstring(struct_type);
        bytes.extend_from_slice(&[0_u8; 16]);
        bytes
    }

    #[test]
    fn decodes_nested_properties_without_raw_buffers() {
        let mut world = tag("PlayerCount", "IntProperty", &[], &3_i32.to_le_bytes());
        world.extend_from_slice(&fstring("None"));
        let mut root = tag(
            "worldSaveData",
            "StructProperty",
            &struct_metadata("PalWorldSaveData"),
            &world,
        );
        root.extend_from_slice(&fstring("None"));
        let decoded =
            decode_properties(&mut Cursor::new(root), &header(), DecodeLimits::default()).unwrap();
        let world = match &decoded[0].value {
            DecodedValue::Struct(world) => world,
            _ => panic!("expected world struct"),
        };
        assert_eq!(world[0].value, DecodedValue::Signed(3));
    }

    #[test]
    fn uses_palworld_hints_for_guid_to_struct_maps() {
        let mut guild = tag("Name", "StrProperty", &[], &fstring("Builders"));
        guild.extend_from_slice(&fstring("None"));
        let mut map_value = 0_u32.to_le_bytes().to_vec();
        map_value.extend_from_slice(&1_u32.to_le_bytes());
        map_value.extend_from_slice(&[0_u8; 16]);
        map_value.extend_from_slice(&guild);
        let mut map_metadata = fstring("StructProperty");
        map_metadata.extend_from_slice(&fstring("StructProperty"));
        let mut world = tag("GroupSaveDataMap", "MapProperty", &map_metadata, &map_value);
        world.extend_from_slice(&fstring("None"));
        let mut root = tag(
            "worldSaveData",
            "StructProperty",
            &struct_metadata("PalWorldSaveData"),
            &world,
        );
        root.extend_from_slice(&fstring("None"));
        let decoded =
            decode_properties(&mut Cursor::new(root), &header(), DecodeLimits::default()).unwrap();
        let world = match &decoded[0].value {
            DecodedValue::Struct(world) => world,
            _ => panic!("expected world struct"),
        };
        let entries = match &world[0].value {
            DecodedValue::Map(entries) => entries,
            _ => panic!("expected guild map"),
        };
        assert!(matches!(entries[0].key, DecodedValue::Guid(_)));
        assert!(matches!(entries[0].value, DecodedValue::Struct(_)));
    }

    #[test]
    fn enforces_global_node_budget() {
        let mut bytes = tag("A", "IntProperty", &[], &1_i32.to_le_bytes());
        bytes.extend_from_slice(&tag("B", "IntProperty", &[], &2_i32.to_le_bytes()));
        bytes.extend_from_slice(&fstring("None"));
        let limits = DecodeLimits {
            max_nodes: 1,
            ..DecodeLimits::default()
        };
        assert!(decode_properties(&mut Cursor::new(bytes), &header(), limits).is_err());
    }

    #[test]
    fn enforces_nesting_depth_after_erasing_recursive_reader_types() {
        let mut nested = tag("Leaf", "IntProperty", &[], &1_i32.to_le_bytes());
        nested.extend_from_slice(&fstring("None"));
        let mut world = tag(
            "Nested",
            "StructProperty",
            &struct_metadata("NestedData"),
            &nested,
        );
        world.extend_from_slice(&fstring("None"));
        let mut root = tag(
            "worldSaveData",
            "StructProperty",
            &struct_metadata("PalWorldSaveData"),
            &world,
        );
        root.extend_from_slice(&fstring("None"));
        let limits = DecodeLimits {
            max_depth: 1,
            ..DecodeLimits::default()
        };

        assert!(decode_properties(&mut Cursor::new(root), &header(), limits).is_err());
    }
}
