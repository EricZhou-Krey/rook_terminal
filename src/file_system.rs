use bevy_ecs::{entity::{EntityGeneration, EntityIndex}, prelude::*};

#[derive(Component)]
pub struct FileName(pub String);

#[derive(Component)]
pub struct VFSParent(pub Entity);

#[derive(Component, Default)]
pub struct VFSChildren(pub Vec<Entity>);

#[derive(Component)]
pub struct VFSDirectory;

#[derive(Component)]
pub struct VFSTextFile(pub String);

#[derive(Component)]
pub struct VFSEntityLink(pub Entity);

#[derive(Component)]
pub struct VFSQueryDirectory {
    pub required_components: Vec<String>,
}

pub struct ECSFileSystem;
impl ECSFileSystem {
    pub fn resolve_path(root: Entity, world: &World, path: &[&str]) -> Option<Entity> {
        let mut current: Entity = root;

        for part in path {
            if part.is_empty() || *part == "." { continue; }

            let children: &VFSChildren = world.get::<VFSChildren>(current)?;

            let mut found: bool = false;
            for &child in &children.0 {
                if let Some(name) = world.get::<FileName>(child) && name.0 == *part {
                    current = child;
                    found = true;
                    break;
                }
            }

            if !found { return None; }
        }

        Some(current)
    }

    pub fn parse_entity_string(s: &str) -> Option<Entity> {
        let parts: Vec<&str> = s.split('v').collect();
        if parts.len() == 2 {
            let index: u32 = parts[0].parse::<u32>().ok()?;
            let generation: u32 = parts[1].parse::<u32>().ok()?;
            let entity_index: EntityIndex = EntityIndex::from_raw_u32(index)?;
            let entity_generation: EntityGeneration = EntityGeneration::from_bits(generation);

            Some(Entity::from_index_and_generation(entity_index, entity_generation))
        } else {
            None
        }
    }
}
