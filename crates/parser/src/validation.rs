//! Stable, deterministic validation issues for the decoded entity index.

use crate::domain::{EntityId, EntityIndex, ReferenceConfidence};
use crate::schema::requires_custom_codec;
use crate::values::{DecodedProperty, DecodedValue, OpaqueKind};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ValidationSeverity {
    Warning,
    Error,
}

impl ValidationSeverity {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ValidationCode {
    MissingKnownEntityReference,
    UnresolvedObservedGuid,
    UnsupportedEntityKindReference,
    OpaqueValueNotInspected,
    UnsupportedPalworldRawData,
}

impl ValidationCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingKnownEntityReference => "missing_known_entity_reference",
            Self::UnresolvedObservedGuid => "unresolved_observed_guid",
            Self::UnsupportedEntityKindReference => "unsupported_entity_kind_reference",
            Self::OpaqueValueNotInspected => "opaque_value_not_inspected",
            Self::UnsupportedPalworldRawData => "unsupported_palworld_raw_data",
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ValidationIssue {
    pub code: ValidationCode,
    pub severity: ValidationSeverity,
    pub entity_id: Option<EntityId>,
    pub target_id: Option<EntityId>,
    pub source_path: String,
}

#[must_use]
pub fn validate_index(index: &EntityIndex) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    for (entity_id, entity) in &index.entities {
        for reference in &entity.references {
            if index.entities.contains_key(&reference.target) {
                continue;
            }
            let (code, severity) = match reference.confidence {
                ReferenceConfidence::Known
                    if reference.target_kind.is_some_and(is_indexed_kind) =>
                {
                    (
                        ValidationCode::MissingKnownEntityReference,
                        ValidationSeverity::Error,
                    )
                }
                ReferenceConfidence::Known => (
                    ValidationCode::UnsupportedEntityKindReference,
                    ValidationSeverity::Warning,
                ),
                ReferenceConfidence::Observed => (
                    ValidationCode::UnresolvedObservedGuid,
                    ValidationSeverity::Warning,
                ),
            };
            issues.push(ValidationIssue {
                code,
                severity,
                entity_id: Some(entity_id.clone()),
                target_id: Some(reference.target.clone()),
                source_path: reference.source_path.clone(),
            });
        }
    }
    issues.sort();
    issues.dedup();
    issues
}

fn is_indexed_kind(kind: crate::domain::EntityKind) -> bool {
    matches!(
        kind,
        crate::domain::EntityKind::Character
            | crate::domain::EntityKind::Guild
            | crate::domain::EntityKind::Base
            | crate::domain::EntityKind::ItemContainer
            | crate::domain::EntityKind::CharacterContainer
    )
}

#[must_use]
pub fn validate_decoded(properties: &[DecodedProperty]) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    visit_properties(properties, "", &mut issues);
    issues.sort();
    issues.dedup();
    issues
}

fn visit_properties(
    properties: &[DecodedProperty],
    parent_path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    for property in properties {
        let path = format!("{parent_path}.{}", property.tag.name);
        if requires_custom_codec(&path) {
            issues.push(ValidationIssue {
                code: ValidationCode::UnsupportedPalworldRawData,
                severity: ValidationSeverity::Error,
                entity_id: None,
                target_id: None,
                source_path: path.clone(),
            });
        }
        visit_value(&property.value, &path, issues);
    }
}

fn visit_value(value: &DecodedValue, path: &str, issues: &mut Vec<ValidationIssue>) {
    match value {
        DecodedValue::Opaque { kind, .. } => {
            if *kind == OpaqueKind::UnsupportedProperty {
                issues.push(ValidationIssue {
                    code: ValidationCode::OpaqueValueNotInspected,
                    severity: ValidationSeverity::Warning,
                    entity_id: None,
                    target_id: None,
                    source_path: path.to_owned(),
                });
            }
        }
        DecodedValue::Struct(properties) => visit_properties(properties, path, issues),
        DecodedValue::Array(values) | DecodedValue::Set(values) => {
            for value in values {
                visit_value(value, path, issues);
            }
        }
        DecodedValue::Map(entries) => {
            for entry in entries {
                visit_value(&entry.key, &format!("{path}.Key"), issues);
                visit_value(&entry.value, &format!("{path}.Value"), issues);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EntityKind, EntityRecord, EntityReference};

    fn id(value: &str) -> EntityId {
        EntityId::parse(value).unwrap()
    }

    #[test]
    fn separates_hard_missing_references_from_observations() {
        let source = id("00000000-0000-0000-0000-000000000001");
        let hard_target = id("00000000-0000-0000-0000-000000000002");
        let observed_target = id("00000000-0000-0000-0000-000000000003");
        let mut index = EntityIndex::default();
        index.entities.insert(
            source.clone(),
            EntityRecord {
                id: source,
                kind: EntityKind::Guild,
                source_path: ".test".to_owned(),
                references: vec![
                    EntityReference {
                        target: hard_target,
                        target_kind: Some(EntityKind::Base),
                        source_path: ".known".to_owned(),
                        confidence: ReferenceConfidence::Known,
                    },
                    EntityReference {
                        target: observed_target,
                        target_kind: None,
                        source_path: ".observed".to_owned(),
                        confidence: ReferenceConfidence::Observed,
                    },
                ],
            },
        );

        let issues = validate_index(&index);
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].severity, ValidationSeverity::Error);
        assert_eq!(issues[1].severity, ValidationSeverity::Warning);
        assert_eq!(
            ValidationCode::MissingKnownEntityReference.as_str(),
            "missing_known_entity_reference"
        );
    }

    #[test]
    fn reports_custom_raw_data_as_incomplete_analysis() {
        use crate::properties::{PropertyMetadata, PropertyTag};

        let tagged = |name: &str, property_type: &str, value| DecodedProperty {
            tag: PropertyTag {
                name: name.to_owned(),
                property_type: property_type.to_owned(),
                size: 0,
                array_index: 0,
                metadata: PropertyMetadata::default(),
                property_guid: None,
            },
            value,
        };
        let raw = tagged("RawData", "ArrayProperty", DecodedValue::Array(Vec::new()));
        let group = tagged(
            "worldSaveData",
            "StructProperty",
            DecodedValue::Struct(vec![tagged(
                "CharacterSaveParameterMap",
                "MapProperty",
                DecodedValue::Map(vec![crate::values::MapEntry {
                    key: DecodedValue::Guid("00000000-0000-0000-0000-000000000001".to_owned()),
                    value: DecodedValue::Struct(vec![raw]),
                }]),
            )]),
        );

        let issues = validate_decoded(&[group]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, ValidationSeverity::Error);
        assert_eq!(issues[0].code, ValidationCode::UnsupportedPalworldRawData);
    }
}
