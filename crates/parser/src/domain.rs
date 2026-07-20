//! Deterministic Palworld entity model built from decoded property maps.

use crate::raw::decode_group_raw;
use crate::values::{DecodedProperty, DecodedValue, MapEntry};
use palmerge_core::{ErrorCode, PalError};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EntityId(String);

impl EntityId {
    pub fn parse(value: &str) -> Result<Self, PalError> {
        let valid = value.len() == 36
            && value
                .chars()
                .enumerate()
                .all(|(index, character)| match index {
                    8 | 13 | 18 | 23 => character == '-',
                    _ => character.is_ascii_hexdigit(),
                });
        if !valid {
            return Err(invalid(format!("invalid entity GUID: {value}")));
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn is_nil(&self) -> bool {
        self.0 == "00000000-0000-0000-0000-000000000000"
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EntityKind {
    Player,
    Character,
    Guild,
    Base,
    ItemContainer,
    CharacterContainer,
    MapObject,
    DynamicItem,
}

impl EntityKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Character => "character",
            Self::Guild => "guild",
            Self::Base => "base",
            Self::ItemContainer => "item_container",
            Self::CharacterContainer => "character_container",
            Self::MapObject => "map_object",
            Self::DynamicItem => "dynamic_item",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReferenceConfidence {
    Known,
    Observed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityReference {
    pub target: EntityId,
    pub target_kind: Option<EntityKind>,
    pub source_path: String,
    pub confidence: ReferenceConfidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityRecord {
    pub id: EntityId,
    pub kind: EntityKind,
    pub source_path: String,
    pub references: Vec<EntityReference>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EntityIndex {
    pub entities: BTreeMap<EntityId, EntityRecord>,
}

impl EntityIndex {
    pub fn build(properties: &[DecodedProperty]) -> Result<Self, PalError> {
        let mut index = Self::default();
        visit_properties(properties, "", &mut index)?;
        Ok(index)
    }

    pub fn get(&self, id: &EntityId) -> Option<&EntityRecord> {
        self.entities.get(id)
    }
}

fn visit_properties(
    properties: &[DecodedProperty],
    parent_path: &str,
    index: &mut EntityIndex,
) -> Result<(), PalError> {
    for property in properties {
        let path = format!("{parent_path}.{}", property.tag.name);
        if let (Some(kind), DecodedValue::Map(entries)) = (entity_map_kind(&path), &property.value)
        {
            index_map(entries, kind, &path, index)?;
        }
        visit_value(&property.value, &path, index)?;
    }
    Ok(())
}

fn visit_value(value: &DecodedValue, path: &str, index: &mut EntityIndex) -> Result<(), PalError> {
    match value {
        DecodedValue::Struct(properties) => visit_properties(properties, path, index),
        DecodedValue::Array(values) | DecodedValue::Set(values) => {
            for (position, value) in values.iter().enumerate() {
                visit_value(value, &format!("{path}[{position}]"), index)?;
            }
            Ok(())
        }
        DecodedValue::Map(entries) => {
            for (position, entry) in entries.iter().enumerate() {
                visit_value(&entry.key, &format!("{path}.Key[{position}]"), index)?;
                visit_value(&entry.value, &format!("{path}.Value[{position}]"), index)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn index_map(
    entries: &[MapEntry],
    kind: EntityKind,
    path: &str,
    index: &mut EntityIndex,
) -> Result<(), PalError> {
    for (position, entry) in entries.iter().enumerate() {
        let id = entity_key(&entry.key, kind, path, position)?;
        let source_path = format!("{path}.Value[{position}]");
        let mut references = Vec::new();
        collect_references(&entry.value, &source_path, &id, &mut references)?;
        if kind == EntityKind::Character {
            collect_character_key_references(
                &entry.key,
                &format!("{path}.Key[{position}]"),
                &id,
                &mut references,
            )?;
        }
        if kind == EntityKind::Guild {
            if let Some(raw) = decode_group_raw(&entry.value, &id)? {
                references.extend(raw.into_iter().map(|reference| EntityReference {
                    target: reference.target,
                    target_kind: Some(reference.target_kind),
                    source_path: format!("{source_path}{}", reference.source_suffix),
                    confidence: ReferenceConfidence::Known,
                }));
            }
        }
        references.sort_by(|left, right| {
            left.target
                .cmp(&right.target)
                .then_with(|| left.source_path.cmp(&right.source_path))
        });
        references.dedup();
        let record = EntityRecord {
            id: id.clone(),
            kind,
            source_path,
            references,
        };
        if index.entities.insert(id.clone(), record).is_some() {
            return Err(invalid(format!("duplicate entity GUID: {}", id.as_str())));
        }
    }
    Ok(())
}

fn entity_key(
    value: &DecodedValue,
    kind: EntityKind,
    path: &str,
    position: usize,
) -> Result<EntityId, PalError> {
    let value = match (kind, value) {
        (EntityKind::Character, DecodedValue::Struct(properties)) => {
            unique_guid_property(properties, "InstanceId", path)?
        }
        (
            EntityKind::ItemContainer | EntityKind::CharacterContainer,
            DecodedValue::Struct(properties),
        ) => unique_guid_property(properties, "ID", path)?,
        (_, DecodedValue::Guid(value)) => value,
        _ => {
            return Err(invalid(format!(
                "entity map {path} contains an unsupported key at position {position}"
            )))
        }
    };
    let id = EntityId::parse(value)?;
    if id.is_nil() {
        return Err(invalid(format!(
            "entity map {path} contains a nil GUID key at position {position}"
        )));
    }
    Ok(id)
}

fn collect_character_key_references(
    value: &DecodedValue,
    path: &str,
    entity_id: &EntityId,
    output: &mut Vec<EntityReference>,
) -> Result<(), PalError> {
    let properties = match value {
        DecodedValue::Struct(properties) => properties,
        _ => return Err(invalid(format!("character key at {path} is not a struct"))),
    };
    if let Some(value) = optional_unique_guid_property(properties, "PlayerUId", path)? {
        let target = EntityId::parse(value)?;
        if !target.is_nil() && &target != entity_id {
            output.push(EntityReference {
                target,
                target_kind: Some(EntityKind::Player),
                source_path: format!("{path}.PlayerUId"),
                confidence: ReferenceConfidence::Known,
            });
        }
    }
    Ok(())
}

fn unique_guid_property<'a>(
    properties: &'a [DecodedProperty],
    name: &str,
    path: &str,
) -> Result<&'a str, PalError> {
    optional_unique_guid_property(properties, name, path)?
        .ok_or_else(|| invalid(format!("{path} is missing GUID property {name}")))
}

fn optional_unique_guid_property<'a>(
    properties: &'a [DecodedProperty],
    name: &str,
    path: &str,
) -> Result<Option<&'a str>, PalError> {
    let mut matches = properties
        .iter()
        .filter(|property| property.tag.name == name);
    let value = matches.next();
    if matches.next().is_some() {
        return Err(invalid(format!(
            "{path} contains duplicate property {name}"
        )));
    }
    match value {
        Some(DecodedProperty {
            value: DecodedValue::Guid(value),
            ..
        }) => Ok(Some(value)),
        Some(_) => Err(invalid(format!("{path}.{name} is not a GUID"))),
        None => Ok(None),
    }
}

fn collect_references(
    value: &DecodedValue,
    path: &str,
    entity_id: &EntityId,
    output: &mut Vec<EntityReference>,
) -> Result<(), PalError> {
    match value {
        DecodedValue::Guid(value) => {
            let target = EntityId::parse(value)?;
            if &target != entity_id && !target.is_nil() {
                output.push(EntityReference {
                    target,
                    target_kind: None,
                    source_path: path.to_owned(),
                    confidence: ReferenceConfidence::Observed,
                });
            }
        }
        DecodedValue::Struct(properties) => {
            for property in properties {
                collect_references(
                    &property.value,
                    &format!("{path}.{}", property.tag.name),
                    entity_id,
                    output,
                )?;
            }
        }
        DecodedValue::Array(values) | DecodedValue::Set(values) => {
            for (position, value) in values.iter().enumerate() {
                collect_references(value, &format!("{path}[{position}]"), entity_id, output)?;
            }
        }
        DecodedValue::Map(entries) => {
            for (position, entry) in entries.iter().enumerate() {
                collect_references(
                    &entry.key,
                    &format!("{path}.Key[{position}]"),
                    entity_id,
                    output,
                )?;
                collect_references(
                    &entry.value,
                    &format!("{path}.Value[{position}]"),
                    entity_id,
                    output,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn entity_map_kind(path: &str) -> Option<EntityKind> {
    match path {
        ".worldSaveData.CharacterSaveParameterMap" => Some(EntityKind::Character),
        ".worldSaveData.GroupSaveDataMap" => Some(EntityKind::Guild),
        ".worldSaveData.BaseCampSaveData" => Some(EntityKind::Base),
        ".worldSaveData.ItemContainerSaveData" => Some(EntityKind::ItemContainer),
        ".worldSaveData.CharacterContainerSaveData" => Some(EntityKind::CharacterContainer),
        _ => None,
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
                property_type: "StructProperty".to_owned(),
                size: 0,
                array_index: 0,
                metadata: PropertyMetadata::default(),
                property_guid: None,
            },
            value,
        }
    }

    #[test]
    fn indexes_character_struct_keys_by_instance_id() {
        let player = "10000000-0000-0000-0000-000000000001";
        let instance = "20000000-0000-0000-0000-000000000002";
        let root = vec![property(
            "worldSaveData",
            DecodedValue::Struct(vec![property(
                "CharacterSaveParameterMap",
                DecodedValue::Map(vec![MapEntry {
                    key: DecodedValue::Struct(vec![
                        property("PlayerUId", DecodedValue::Guid(player.to_owned())),
                        property("InstanceId", DecodedValue::Guid(instance.to_owned())),
                    ]),
                    value: DecodedValue::Struct(Vec::new()),
                }]),
            )]),
        )];
        let index = EntityIndex::build(&root).unwrap();
        let character = index.get(&EntityId::parse(instance).unwrap()).unwrap();
        assert_eq!(character.kind, EntityKind::Character);
        assert_eq!(character.references[0].target.as_str(), player);
        assert_eq!(
            character.references[0].target_kind,
            Some(EntityKind::Player)
        );
    }

    #[test]
    fn rejects_duplicate_entity_ids_across_maps() {
        let id = "00112233-4455-6677-8899-aabbccddeeff";
        let map = |name: &str| {
            property(
                name,
                DecodedValue::Map(vec![MapEntry {
                    key: DecodedValue::Guid(id.to_owned()),
                    value: DecodedValue::Struct(Vec::new()),
                }]),
            )
        };
        let root = vec![property(
            "worldSaveData",
            DecodedValue::Struct(vec![map("GroupSaveDataMap"), map("BaseCampSaveData")]),
        )];
        assert!(EntityIndex::build(&root).is_err());
    }

    #[test]
    fn indexes_container_struct_keys_by_id() {
        let id = "00112233-4455-6677-8899-aabbccddeeff";
        let root = vec![property(
            "worldSaveData",
            DecodedValue::Struct(vec![property(
                "ItemContainerSaveData",
                DecodedValue::Map(vec![MapEntry {
                    key: DecodedValue::Struct(vec![property(
                        "ID",
                        DecodedValue::Guid(id.to_owned()),
                    )]),
                    value: DecodedValue::Struct(Vec::new()),
                }]),
            )]),
        )];
        let index = EntityIndex::build(&root).unwrap();
        assert_eq!(
            index.get(&EntityId::parse(id).unwrap()).unwrap().kind,
            EntityKind::ItemContainer
        );
    }
}
