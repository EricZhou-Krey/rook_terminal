use bevy_ecs::{
    archetype::{Archetype, ArchetypeEntity, Archetypes},
    component::{ComponentId, Components},
    entity::Entities,
    prelude::*,
    system::{SystemId, SystemParam},
};
use std::{
    collections::{HashMap, HashSet},
    ops::{BitAnd, BitOr, Not},
};

#[derive(Component)]
pub struct VFSName {
    pub name: String,
}

#[derive(Component)]
pub struct VFSParent {
    pub parent: Entity,
}

#[derive(Component, Default)]
pub struct VFSChildren {
    pub children: Vec<Entity>,
}

#[derive(Clone, Debug)]
pub enum DynamicFilter {
    Has(ComponentId),
    And(Vec<DynamicFilter>),
    Or(Vec<DynamicFilter>),
    Not(Box<DynamicFilter>),
    Never,
}

pub trait VFSFilterExtension {
    fn filter<T: Component>(&self) -> DynamicFilter;
}

impl VFSFilterExtension for World {
    fn filter<T: Component>(&self) -> DynamicFilter {
        match self.component_id::<T>() {
            Some(id) => DynamicFilter::Has(id),
            None => DynamicFilter::Never,
        }
    }
}

impl BitAnd for DynamicFilter {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        DynamicFilter::And(vec![self, rhs])
    }
}

impl BitOr for DynamicFilter {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        DynamicFilter::Or(vec![self, rhs])
    }
}

impl Not for DynamicFilter {
    type Output = Self;
    fn not(self) -> Self::Output {
        if let DynamicFilter::Not(inner) = self {
            return *inner;
        }
        DynamicFilter::Not(Box::new(self))
    }
}

impl DynamicFilter {
    pub fn matches_archetype(&self, archetype: &Archetype) -> bool {
        match self {
            DynamicFilter::Has(id) => archetype.contains(*id),
            DynamicFilter::And(filters) => filters
                .iter()
                .all(|filter| filter.matches_archetype(archetype)),
            DynamicFilter::Or(filters) => filters
                .iter()
                .any(|filter| filter.matches_archetype(archetype)),
            DynamicFilter::Not(filter) => !filter.matches_archetype(archetype),
            DynamicFilter::Never => false,
        }
    }
}

#[derive(Component)]
pub struct VFSQueryChildren {
    pub filter: DynamicFilter,
}

#[derive(Component)]
pub struct Text {
    pub text: String,
}

#[derive(Component)]
pub struct Binary {
    pub binary: Vec<u8>,
}

#[derive(SystemParam)]
pub struct ECSFileSystem<'w, 's> {
    pub vfs_names: Query<'w, 's, &'static VFSName>,
    pub vfs_parents: Query<'w, 's, &'static VFSParent>,
    pub vfs_children: Query<'w, 's, &'static VFSChildren>,
    pub vfs_query_children: Query<'w, 's, &'static VFSQueryChildren>,
    pub archetypes: &'w Archetypes,
    pub components: &'w Components,
    pub entities: &'w Entities,
}

impl<'w, 's> ECSFileSystem<'w, 's> {
    pub fn resolve_path(&self, root: Entity, path: &str) -> Option<Entity> {
        let mut current: Entity = root;

        for part in path.split('/') {
            if part.is_empty() || part == "." {
                continue;
            } else if part == ".." {
                if let Some(parent) = self.parent(current) {
                    current = parent;
                }
                continue;
            }

            let children: Vec<Entity> = self.children(current);

            let mut found: bool = false;
            for child in children {
                if let Ok(vfs_name) = self.vfs_names.get(child) &&
                vfs_name.name == *part {
                    current = child;
                    found = true;
                    break;
                }
            }

            if !found {
                return None;
            }
        }

        Some(current)
    }

    pub fn parent(&self, entity: Entity) -> Option<Entity> {
        self.vfs_parents.get(entity).map(|p| p.parent).ok()
    }

    pub fn children(&self, directory: Entity) -> Vec<Entity> {
        let mut children: Vec<Entity> = Vec::new();

        if let Ok(vfs_children) = self.vfs_children.get(directory) {
            children.extend(vfs_children.children.clone());
        }

        if let Ok(vfs_query_children) = self.vfs_query_children.get(directory) {
            let entities: Vec<Entity> = self
                .archetypes
                .iter()
                .filter(|&archetype: &&Archetype| {
                    vfs_query_children.filter.matches_archetype(archetype)
                })
                .flat_map(|archetype: &Archetype| archetype.entities())
                .map(|archetype_entity: &ArchetypeEntity| archetype_entity.id())
                .collect();

            children.extend(entities);
        }

        children
    }

    pub fn compute_size(&self, entity: Entity) -> usize {
        let mut visited: HashSet<Entity> = HashSet::new();
        self.compute_size_inner(entity, &mut visited)
    }

    fn compute_size_inner(&self, entity: Entity, visited: &mut HashSet<Entity>) -> usize {
        if !visited.insert(entity) {
            return 0;
        }

        let mut total_size: usize = 0;
        let children: Vec<Entity> = self.children(entity);

        if !children.is_empty() {
            for child in children {
                total_size += self.compute_size_inner(child, visited);
            }
            return total_size;
        }

        if let Ok(Some(location)) = self.entities.get(entity) &&
        let Some(archetype) = self.archetypes.get(location.archetype_id) {
            for component_id in archetype.components() {
                if let Some(info) = self.components.get_info(*component_id) {
                    total_size += info.layout().size();
                }
            }
        }

        total_size
    }

    pub fn entity_vfs_name(&self, entity: Entity) -> String {
        if let Ok(vfs_name) = self.vfs_names.get(entity) {
            vfs_name.name.clone()
        } else {
            format!("{}v{}", entity.index(), entity.generation())
        }
    }
}

#[derive(Resource, Default)]
pub struct CommandRegistry {
    pub commands: HashMap<String, SystemId<In<Vec<String>>>>,
}

impl CommandRegistry {
    pub fn register(&mut self, name: &str, command_id: SystemId<In<Vec<String>>>) {
        self.commands.insert(name.to_string(), command_id);
    }

    pub fn remove(&mut self, name: &str) {
        self.commands.remove(name);
    }
}
