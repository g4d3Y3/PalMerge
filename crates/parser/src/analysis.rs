//! End-to-end read-only analysis pipeline for decoded GVAS saves.

use crate::container::ContainerHeader;
use crate::domain::EntityIndex;
use crate::graph::DependencyGraph;
use crate::gvas::{read_gvas_decoded, GvasHeader};
use crate::validation::{validate_decoded, validate_index, ValidationIssue};
use crate::values::{DecodeLimits, DecodedProperty};
use palmerge_core::PalError;
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct SaveAnalysis {
    pub header: GvasHeader,
    pub properties: Vec<DecodedProperty>,
    pub entities: EntityIndex,
    pub dependencies: DependencyGraph,
    pub issues: Vec<ValidationIssue>,
}

impl SaveAnalysis {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.issues.is_empty()
    }
}

pub fn analyze_gvas(
    path: &Path,
    container: Option<ContainerHeader>,
    limits: DecodeLimits,
) -> Result<SaveAnalysis, PalError> {
    let (header, properties) = read_gvas_decoded(path, container, limits)?;
    analyze_decoded(header, properties)
}

pub fn analyze_decoded(
    header: GvasHeader,
    properties: Vec<DecodedProperty>,
) -> Result<SaveAnalysis, PalError> {
    let entities = EntityIndex::build(&properties)?;
    let dependencies = DependencyGraph::from_index(&entities);
    let mut issues = validate_decoded(&properties);
    issues.extend(validate_index(&entities));
    issues.sort();
    issues.dedup();
    Ok(SaveAnalysis {
        header,
        properties,
        entities,
        dependencies,
        issues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::EntityKind;
    use crate::gvas::{EngineVersion, PackageVersion};
    use crate::properties::{PropertyMetadata, PropertyTag};
    use crate::values::{DecodedValue, MapEntry};

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
    fn connects_decoding_outputs_to_index_graph_and_validation() {
        let guild_id = "00112233-4455-6677-8899-aabbccddeeff";
        let properties = vec![property(
            "worldSaveData",
            DecodedValue::Struct(vec![property(
                "GroupSaveDataMap",
                DecodedValue::Map(vec![MapEntry {
                    key: DecodedValue::Guid(guild_id.to_owned()),
                    value: DecodedValue::Struct(Vec::new()),
                }]),
            )]),
        )];
        let header = GvasHeader {
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
        };

        let analysis = analyze_decoded(header, properties).unwrap();
        assert_eq!(analysis.entities.entities.len(), 1);
        assert_eq!(
            analysis.entities.entities.values().next().unwrap().kind,
            EntityKind::Guild
        );
        assert_eq!(analysis.dependencies.edges.len(), 1);
        assert!(analysis.issues.is_empty());
        assert!(analysis.is_complete());
    }
}
