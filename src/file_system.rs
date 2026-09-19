use std::collections::{HashMap, HashSet};
use bevy_ecs::{archetype::{Archetype, ArchetypeEntity}, component::ComponentId, prelude::*};
use crate::{Terminal, command::{CommandResult, Command}};

#[derive(Component)]
pub struct VFSName { pub name: String }

#[derive(Component)]
pub struct VFSParent { pub parent: Entity }

#[derive(Component, Default)]
pub struct VFSChildren { pub children: Vec<Entity> }

#[derive(Component)]
pub struct VFSQueryChildren { pub required_components: Vec<ComponentId> }

#[derive(Component)]
pub struct Text { pub text: String }

#[derive(Component)]
pub struct Binary { pub binary: Vec<u8> }

pub struct ECSFileSystem;
impl ECSFileSystem {
    pub fn resolve_path(root: Entity, world: &World, path: &str) -> Option<Entity> {
        let mut current: Entity = root;

        for part in path.split('/') {
            if part.is_empty() || part == "." { continue; }
            else if part == ".." {
                if let Some(parent) = ECSFileSystem::parent(current, world) {
                    current = parent;
                }
                continue;
            }

            let children: Vec<Entity> = ECSFileSystem::children(current, world);

            let mut found: bool = false;
            for child in children {
                if let Some(vfs_name) = world.get::<VFSName>(child) && vfs_name.name == *part {
                    current = child;
                    found = true;
                    break;
                }
            }

            if !found { return None; }
        }

        Some(current)
    }

    pub fn parent(entity: Entity, world: &World) -> Option<Entity> {
        Some(world.get::<VFSParent>(entity)?.parent)
    }

    pub fn children(directory: Entity, world: &World) -> Vec<Entity> {
        let mut children: Vec<Entity> = Vec::new();
        
        if let Some(vfs_children) = world.get::<VFSChildren>(directory) {
            children.extend(vfs_children.children.clone());
        }
        
        if let Some(vfs_query_children) = world.get::<VFSQueryChildren>(directory) {
            let entities: Vec<Entity> = world.archetypes().iter()
                .filter(|&archtype: &&Archetype|
                    vfs_query_children.required_components.iter().all(|&id: &ComponentId| archtype.contains(id))
                )
                .flat_map(|archetype: &Archetype| archetype.entities())
                .map(|archetype_entity: &ArchetypeEntity| archetype_entity.id())
                .collect();

            children.extend(entities);
        }

        children
    }

    pub fn compute_size(entity: Entity, world: &World) -> usize {
        let mut visited = HashSet::new();
        Self::compute_size_inner(entity, world, &mut visited)
    }

    fn compute_size_inner(entity: Entity, world: &World, visited: &mut HashSet<Entity>) -> usize {
        if !visited.insert(entity) {
            return 0; 
        }

        let mut total_size: usize = 0;
        let children: Vec<Entity> = Self::children(entity, world);

        if !children.is_empty() {
            for child in children {
                total_size += Self::compute_size_inner(child, world, visited);
            }
            return total_size;
        }

        if let Some(text) = world.get::<Text>(entity) {
            total_size += text.text.len();
        } else if let Some(binary) = world.get::<Binary>(entity) {
            total_size += binary.binary.len();
        } else {
            if let Ok(entity_ref) = world.get_entity(entity) {
                for comp_id in entity_ref.archetype().components() {
                    if let Some(info) = world.components().get_info(*comp_id) {
                        total_size += info.layout().size();
                    }
                }
            }
        }

        total_size
    }
    pub fn entity_vfs_name(entity: Entity, world: &World) -> String {
        if let Some(vfs_name) = world.get::<VFSName>(entity) {
            vfs_name.name.clone()
        } else {
            format!("{}v{}", entity.index(), entity.generation())
        }
    }
}

pub type CommandFn = fn(&mut Terminal, &mut World, &[&str]) -> CommandResult;

#[derive(Resource, Default)]
pub struct CommandRegistry {
    pub commands: HashMap<String, CommandFn>,
}

impl CommandRegistry {
    pub fn register<C: Command>(&mut self) {
        self.commands.insert(C::name().to_string(), C::execute);
    }

    pub fn remove(&mut self, name: &str) {
        self.commands.remove(name);
    }
}
