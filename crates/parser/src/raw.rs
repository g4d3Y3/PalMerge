//! Bounded decoders for Palworld-specific relationship data stored in byte arrays.

use crate::domain::{EntityId, EntityKind};
use crate::gvas::format_guid;
use crate::values::{DecodedProperty, DecodedValue};
use palmerge_core::{ErrorCode, PalError};

const MAX_RAW_COLLECTION_ENTRIES: usize = 250_000;
const MAX_RAW_STRING_UNITS: usize = 64 * 1024;

pub(crate) struct RawReference {
    pub target: EntityId,
    pub target_kind: EntityKind,
    pub source_suffix: String,
}

pub(crate) fn decode_group_raw(
    value: &DecodedValue,
    map_id: &EntityId,
) -> Result<Option<Vec<RawReference>>, PalError> {
    let properties = match value {
        DecodedValue::Struct(properties) => properties,
        _ => return Ok(None),
    };
    let raw = match property_value(properties, "RawData")? {
        Some(raw) => raw,
        None => return Ok(None),
    };
    let group_type = match property_value(properties, "GroupType")? {
        Some(DecodedValue::String(value)) => value.as_str(),
        _ => return Err(invalid("group RawData is missing GroupType")),
    };
    let bytes = match raw {
        DecodedValue::Array(bytes) => bytes,
        _ => return Err(invalid("group RawData is not a byte array")),
    };

    let mut reader = RawReader::new(bytes);
    let raw_id = reader.guid("group_id")?;
    if &raw_id != map_id {
        return Err(invalid(format!(
            "group map key {} does not match RawData group_id {}",
            map_id.as_str(),
            raw_id.as_str()
        )));
    }
    let _group_name = reader.fstring("group_name")?;
    let mut references = Vec::new();

    let handle_count = reader.count("individual_character_handle_ids")?;
    for position in 0..handle_count {
        push_reference(
            &mut references,
            reader.guid("character handle guid")?,
            EntityKind::Player,
            format!(".RawData.individual_character_handle_ids[{position}].guid"),
            map_id,
        );
        push_reference(
            &mut references,
            reader.guid("character handle instance_id")?,
            EntityKind::Character,
            format!(".RawData.individual_character_handle_ids[{position}].instance_id"),
            map_id,
        );
    }

    let organization = matches!(
        group_type,
        "EPalGroupType::Guild" | "EPalGroupType::IndependentGuild" | "EPalGroupType::Organization"
    );
    let guild = matches!(
        group_type,
        "EPalGroupType::Guild" | "EPalGroupType::IndependentGuild"
    );
    if organization {
        let _organization_type = reader.byte("organization type")?;
        let base_count = reader.count("base_ids")?;
        for position in 0..base_count {
            push_reference(
                &mut references,
                reader.guid("base id")?,
                EntityKind::Base,
                format!(".RawData.base_ids[{position}]"),
                map_id,
            );
        }
    }
    if guild {
        let _base_camp_level = reader.i32("base camp level")?;
        let map_object_count = reader.count("base camp map object ids")?;
        for position in 0..map_object_count {
            push_reference(
                &mut references,
                reader.guid("base camp map object id")?,
                EntityKind::MapObject,
                format!(".RawData.map_object_instance_ids_base_camp_points[{position}]"),
                map_id,
            );
        }
        let _guild_name = reader.fstring("guild name")?;
    }
    if group_type == "EPalGroupType::IndependentGuild" {
        push_reference(
            &mut references,
            reader.guid("independent guild player uid")?,
            EntityKind::Player,
            ".RawData.player_uid".to_owned(),
            map_id,
        );
        let _guild_name = reader.fstring("independent guild name")?;
        let _last_online = reader.i64("last online time")?;
        let _player_name = reader.fstring("player name")?;
    }
    if group_type == "EPalGroupType::Guild" {
        push_reference(
            &mut references,
            reader.guid("admin player uid")?,
            EntityKind::Player,
            ".RawData.admin_player_uid".to_owned(),
            map_id,
        );
        let player_count = reader.signed_count("guild players")?;
        for position in 0..player_count {
            push_reference(
                &mut references,
                reader.guid("guild player uid")?,
                EntityKind::Player,
                format!(".RawData.players[{position}].player_uid"),
                map_id,
            );
            let _last_online = reader.i64("last online time")?;
            let _player_name = reader.fstring("player name")?;
        }
    }
    if !reader.is_empty() {
        return Err(invalid(format!(
            "unsupported or trailing group RawData for {group_type}"
        )));
    }
    references.sort_by(|left, right| {
        left.target
            .cmp(&right.target)
            .then_with(|| left.source_suffix.cmp(&right.source_suffix))
    });
    references.dedup_by(|left, right| {
        left.target == right.target
            && left.target_kind == right.target_kind
            && left.source_suffix == right.source_suffix
    });
    Ok(Some(references))
}

fn property_value<'a>(
    properties: &'a [DecodedProperty],
    name: &str,
) -> Result<Option<&'a DecodedValue>, PalError> {
    let mut matches = properties
        .iter()
        .filter(|property| property.tag.name == name);
    let value = matches.next().map(|property| &property.value);
    if matches.next().is_some() {
        return Err(invalid(format!(
            "group data contains duplicate {name} properties"
        )));
    }
    Ok(value)
}

fn push_reference(
    output: &mut Vec<RawReference>,
    target: EntityId,
    target_kind: EntityKind,
    source_suffix: String,
    source_id: &EntityId,
) {
    if &target != source_id && !target.is_nil() {
        output.push(RawReference {
            target,
            target_kind,
            source_suffix,
        });
    }
}

struct RawReader<'a> {
    values: &'a [DecodedValue],
    position: usize,
}

impl<'a> RawReader<'a> {
    fn new(values: &'a [DecodedValue]) -> Self {
        Self {
            values,
            position: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.position == self.values.len()
    }

    fn byte(&mut self, field: &str) -> Result<u8, PalError> {
        let value = self
            .values
            .get(self.position)
            .ok_or_else(|| invalid(format!("group RawData is truncated at {field}")))?;
        self.position += 1;
        match value {
            DecodedValue::Unsigned(value) => u8::try_from(*value)
                .map_err(|_| invalid(format!("group RawData {field} is not a byte"))),
            _ => Err(invalid(format!(
                "group RawData {field} contains a non-byte value"
            ))),
        }
    }

    fn bytes<const N: usize>(&mut self, field: &str) -> Result<[u8; N], PalError> {
        let mut output = [0_u8; N];
        for byte in &mut output {
            *byte = self.byte(field)?;
        }
        Ok(output)
    }

    fn i32(&mut self, field: &str) -> Result<i32, PalError> {
        Ok(i32::from_le_bytes(self.bytes(field)?))
    }

    fn u32(&mut self, field: &str) -> Result<u32, PalError> {
        Ok(u32::from_le_bytes(self.bytes(field)?))
    }

    fn i64(&mut self, field: &str) -> Result<i64, PalError> {
        Ok(i64::from_le_bytes(self.bytes(field)?))
    }

    fn guid(&mut self, field: &str) -> Result<EntityId, PalError> {
        EntityId::parse(&format_guid(
            self.u32(field)?,
            self.u32(field)?,
            self.u32(field)?,
            self.u32(field)?,
        ))
    }

    fn count(&mut self, field: &str) -> Result<usize, PalError> {
        let value = self.u32(field)?;
        self.checked_count(u64::from(value), field)
    }

    fn signed_count(&mut self, field: &str) -> Result<usize, PalError> {
        let value = self.i32(field)?;
        if value < 0 {
            return Err(invalid(format!("group RawData {field} count is negative")));
        }
        self.checked_count(value as u64, field)
    }

    fn checked_count(&self, value: u64, field: &str) -> Result<usize, PalError> {
        let value = usize::try_from(value)
            .map_err(|_| invalid(format!("group RawData {field} count overflows")))?;
        if value > MAX_RAW_COLLECTION_ENTRIES {
            return Err(invalid(format!(
                "group RawData {field} count {value} exceeds limit {MAX_RAW_COLLECTION_ENTRIES}"
            )));
        }
        Ok(value)
    }

    fn fstring(&mut self, field: &str) -> Result<String, PalError> {
        let length = self.i32(field)?;
        if length == 0 {
            return Ok(String::new());
        }
        if length == i32::MIN {
            return Err(invalid(format!("invalid group RawData {field} length")));
        }
        let units = length.unsigned_abs() as usize;
        if units > MAX_RAW_STRING_UNITS {
            return Err(invalid(format!(
                "group RawData {field} length {units} exceeds limit {MAX_RAW_STRING_UNITS}"
            )));
        }
        if length > 0 {
            let mut bytes = Vec::with_capacity(units.saturating_sub(1));
            for position in 0..units {
                let byte = self.byte(field)?;
                if position + 1 == units {
                    if byte != 0 {
                        return Err(invalid(format!(
                            "group RawData {field} is not null-terminated"
                        )));
                    }
                } else {
                    bytes.push(byte);
                }
            }
            String::from_utf8(bytes)
                .map_err(|_| invalid(format!("group RawData {field} is not valid UTF-8")))
        } else {
            let mut chars = Vec::with_capacity(units.saturating_sub(1));
            for position in 0..units {
                let value = u16::from_le_bytes(self.bytes(field)?);
                if position + 1 == units {
                    if value != 0 {
                        return Err(invalid(format!(
                            "group RawData {field} is not null-terminated"
                        )));
                    }
                } else {
                    chars.push(value);
                }
            }
            String::from_utf16(&chars)
                .map_err(|_| invalid(format!("group RawData {field} is not valid UTF-16")))
        }
    }
}

fn invalid(message: impl Into<String>) -> PalError {
    PalError::new(ErrorCode::UnknownFormat, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::{PropertyMetadata, PropertyTag};

    fn property(name: &str, value: DecodedValue) -> DecodedProperty {
        DecodedProperty {
            tag: PropertyTag {
                name: name.to_owned(),
                property_type: "ArrayProperty".to_owned(),
                size: 0,
                array_index: 0,
                metadata: PropertyMetadata::default(),
                property_guid: None,
            },
            value,
        }
    }

    #[test]
    fn rejects_truncated_group_raw_data() {
        let value = DecodedValue::Struct(vec![
            property(
                "GroupType",
                DecodedValue::String("EPalGroupType::Guild".to_owned()),
            ),
            property("RawData", DecodedValue::Array(Vec::new())),
        ]);
        let id = EntityId::parse("00112233-4455-6677-8899-aabbccddeeff").unwrap();
        assert!(decode_group_raw(&value, &id).is_err());
    }
}
