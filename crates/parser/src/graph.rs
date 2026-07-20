//! Deterministic, cycle-safe dependency graph over decoded entities.

use crate::domain::{EntityId, EntityIndex, EntityKind, ReferenceConfidence};
use palmerge_core::{ErrorCode, PalError};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyScope {
    KnownOnly,
    IncludeObserved,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DependencyEdge {
    pub target: EntityId,
    pub target_kind: Option<EntityKind>,
    pub source_path: String,
    pub confidence: ReferenceConfidence,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DependencyGraph {
    pub edges: BTreeMap<EntityId, Vec<DependencyEdge>>,
}

impl DependencyGraph {
    #[must_use]
    pub fn from_index(index: &EntityIndex) -> Self {
        let mut edges = BTreeMap::new();
        for (id, entity) in &index.entities {
            let mut outgoing = entity
                .references
                .iter()
                .filter(|reference| index.entities.contains_key(&reference.target))
                .map(|reference| DependencyEdge {
                    target: reference.target.clone(),
                    target_kind: reference.target_kind,
                    source_path: reference.source_path.clone(),
                    confidence: reference.confidence,
                })
                .collect::<Vec<_>>();
            outgoing.sort();
            outgoing.dedup();
            edges.insert(id.clone(), outgoing);
        }
        Self { edges }
    }

    pub fn closure(
        &self,
        root: &EntityId,
        scope: DependencyScope,
    ) -> Result<BTreeSet<EntityId>, PalError> {
        if !self.edges.contains_key(root) {
            return Err(invalid(format!(
                "dependency root is not indexed: {}",
                root.as_str()
            )));
        }

        let mut visited = BTreeSet::new();
        let mut pending = VecDeque::from([root.clone()]);
        while let Some(current) = pending.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            for edge in self.edges.get(&current).into_iter().flatten() {
                if scope == DependencyScope::KnownOnly
                    && edge.confidence != ReferenceConfidence::Known
                {
                    continue;
                }
                if !visited.contains(&edge.target) {
                    pending.push_back(edge.target.clone());
                }
            }
        }
        Ok(visited)
    }
}

fn invalid(message: impl Into<String>) -> PalError {
    PalError::new(ErrorCode::UnknownFormat, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{EntityKind, EntityRecord, EntityReference};

    fn id(value: &str) -> EntityId {
        EntityId::parse(value).unwrap()
    }

    fn record(id: EntityId, references: Vec<EntityReference>) -> EntityRecord {
        EntityRecord {
            id,
            kind: EntityKind::Guild,
            source_path: ".test".to_owned(),
            references,
        }
    }

    #[test]
    fn computes_cycle_safe_closure_by_confidence() {
        let a = id("00000000-0000-0000-0000-000000000001");
        let b = id("00000000-0000-0000-0000-000000000002");
        let c = id("00000000-0000-0000-0000-000000000003");
        let reference = |target: &EntityId, confidence| EntityReference {
            target: target.clone(),
            target_kind: None,
            source_path: ".ref".to_owned(),
            confidence,
        };
        let mut index = EntityIndex::default();
        index.entities.insert(
            a.clone(),
            record(a.clone(), vec![reference(&b, ReferenceConfidence::Known)]),
        );
        index.entities.insert(
            b.clone(),
            record(
                b.clone(),
                vec![
                    reference(&a, ReferenceConfidence::Known),
                    reference(&c, ReferenceConfidence::Observed),
                ],
            ),
        );
        index
            .entities
            .insert(c.clone(), record(c.clone(), Vec::new()));

        let graph = DependencyGraph::from_index(&index);
        assert_eq!(
            graph.closure(&a, DependencyScope::KnownOnly).unwrap(),
            BTreeSet::from([a.clone(), b.clone()])
        );
        assert_eq!(
            graph.closure(&a, DependencyScope::IncludeObserved).unwrap(),
            BTreeSet::from([a, b, c])
        );
    }
}
