//! Palworld-specific type hints required by legacy GVAS map/set values.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructHint {
    Guid,
    Properties,
}

#[must_use]
pub fn struct_hint(path: &str) -> Option<StructHint> {
    match path {
        ".worldSaveData.CharacterContainerSaveData.Key"
        | ".worldSaveData.CharacterSaveParameterMap.Key"
        | ".worldSaveData.CharacterSaveParameterMap.Value"
        | ".worldSaveData.FoliageGridSaveDataMap.Key"
        | ".worldSaveData.FoliageGridSaveDataMap.Value"
        | ".worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value"
        | ".worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value.InstanceDataMap.Key"
        | ".worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value.InstanceDataMap.Value"
        | ".worldSaveData.ItemContainerSaveData.Key"
        | ".worldSaveData.ItemContainerSaveData.Value"
        | ".worldSaveData.MapObjectSaveData.MapObjectSaveData.ConcreteModel.ModuleMap.Value"
        | ".worldSaveData.MapObjectSaveData.MapObjectSaveData.Model.EffectMap.Value"
        | ".worldSaveData.MapObjectSpawnerInStageSaveData.Key"
        | ".worldSaveData.MapObjectSpawnerInStageSaveData.Value"
        | ".worldSaveData.MapObjectSpawnerInStageSaveData.Value.SpawnerDataMapByLevelObjectInstanceId.Value"
        | ".worldSaveData.MapObjectSpawnerInStageSaveData.Value.SpawnerDataMapByLevelObjectInstanceId.Value.ItemMap.Value"
        | ".worldSaveData.WorkSaveData.WorkSaveData.WorkAssignMap.Value"
        | ".worldSaveData.BaseCampSaveData.Value"
        | ".worldSaveData.BaseCampSaveData.Value.ModuleMap.Value"
        | ".worldSaveData.CharacterContainerSaveData.Value"
        | ".worldSaveData.GroupSaveDataMap.Value"
        | ".worldSaveData.EnemyCampSaveData.EnemyCampStatusMap.Value"
        | ".worldSaveData.DungeonSaveData.DungeonSaveData.MapObjectSaveData.MapObjectSaveData.Model.EffectMap.Value"
        | ".worldSaveData.DungeonSaveData.DungeonSaveData.MapObjectSaveData.MapObjectSaveData.ConcreteModel.ModuleMap.Value"
        | ".worldSaveData.InvaderSaveData.Value"
        | ".worldSaveData.OilrigSaveData.OilrigMap.Value"
        | ".worldSaveData.SupplySaveData.SupplyInfos.Value" => Some(StructHint::Properties),
        ".worldSaveData.MapObjectSpawnerInStageSaveData.Value.SpawnerDataMapByLevelObjectInstanceId.Key"
        | ".worldSaveData.BaseCampSaveData.Key"
        | ".worldSaveData.GroupSaveDataMap.Key"
        | ".worldSaveData.InvaderSaveData.Key"
        | ".worldSaveData.SupplySaveData.SupplyInfos.Key" => Some(StructHint::Guid),
        _ => None,
    }
}

#[must_use]
pub fn requires_custom_codec(path: &str) -> bool {
    matches!(
        path,
        ".worldSaveData.CharacterSaveParameterMap.Value.RawData"
            | ".worldSaveData.ItemContainerSaveData.Value.RawData"
            | ".worldSaveData.ItemContainerSaveData.Value.Slots.Slots.RawData"
            | ".worldSaveData.CharacterContainerSaveData.Value.Slots.Slots.RawData"
            | ".worldSaveData.DynamicItemSaveData.DynamicItemSaveData.RawData"
            | ".worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value.RawData"
            | ".worldSaveData.FoliageGridSaveDataMap.Value.ModelMap.Value.InstanceDataMap.Value.RawData"
            | ".worldSaveData.BaseCampSaveData.Value.RawData"
            | ".worldSaveData.BaseCampSaveData.Value.WorkerDirector.RawData"
            | ".worldSaveData.BaseCampSaveData.Value.WorkCollection.RawData"
            | ".worldSaveData.BaseCampSaveData.Value.ModuleMap"
            | ".worldSaveData.WorkSaveData"
            | ".worldSaveData.MapObjectSaveData"
    )
}

#[must_use]
pub fn is_opaque_custom_property(path: &str) -> bool {
    matches!(
        path,
        ".worldSaveData.BaseCampSaveData.Value.ModuleMap"
            | ".worldSaveData.WorkSaveData"
            | ".worldSaveData.MapObjectSaveData"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_palworld_map_hints() {
        assert_eq!(
            struct_hint(".worldSaveData.GroupSaveDataMap.Key"),
            Some(StructHint::Guid)
        );
        assert_eq!(
            struct_hint(".worldSaveData.GroupSaveDataMap.Value"),
            Some(StructHint::Properties)
        );
        assert_eq!(struct_hint(".unknown.Value"), None);
        assert!(requires_custom_codec(
            ".worldSaveData.CharacterSaveParameterMap.Value.RawData"
        ));
        assert!(!requires_custom_codec(".worldSaveData.PlayerCount"));
        assert!(is_opaque_custom_property(
            ".worldSaveData.MapObjectSaveData"
        ));
    }
}
