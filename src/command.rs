use crate::file_system::{
    ECSFileSystem, FileName, VFSChildren, VFSDirectory, VFSEntityLink, VFSQueryDirectory,
    VFSTextFile,
};
use crate::Terminal;
use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct CommandRegistry {
    pub commands: HashMap<String, fn(&mut Terminal, &mut World, &[&str]) -> CommandResult>,
}

impl CommandRegistry {
    pub fn register<C: Command>(&mut self) {
        self.commands.insert(C::name().to_string(), C::execute);
    }

    pub fn remove(&mut self, name: &str) {
        self.commands.remove(name);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    Handled,
    Unhandled(String, Vec<String>),
}

pub trait Command {
    fn name() -> &'static str;
    fn execute(terminal: &mut Terminal, world: &mut World, args: &[&str]) -> CommandResult;
}

pub struct ClearCommand;
impl Command for ClearCommand {
    fn name() -> &'static str {
        "clear"
    }
    fn execute(terminal: &mut Terminal, _world: &mut World, _args: &[&str]) -> CommandResult {
        terminal.history.clear();
        CommandResult::Handled
    }
}

pub struct PwdCommand;
impl Command for PwdCommand {
    fn name() -> &'static str {
        "pwd"
    }
    fn execute(terminal: &mut Terminal, _world: &mut World, _args: &[&str]) -> CommandResult {
        let path = format!("/{}", terminal.current_directory.join("/"));
        terminal.history.push(path);
        CommandResult::Handled
    }
}

pub struct LsCommand;
impl Command for LsCommand {
    fn name() -> &'static str {
        "ls"
    }
    fn execute(terminal: &mut Terminal, world: &mut World, args: &[&str]) -> CommandResult {
        let show_all = args.contains(&"-a");
        let path_refs: Vec<&str> = terminal
            .current_directory
            .iter()
            .map(|s| s.as_str())
            .collect();

        let Some(target_entity) =
            ECSFileSystem::resolve_path(terminal.root_entity, world, &path_refs)
        else {
            terminal.history.push(format!(
                "ls: cannot access '{}': No such file or directory",
                terminal.current_directory.join("/")
            ));
            return CommandResult::Handled;
        };

        let mut output_files = Vec::new();
        if show_all {
            output_files.push(".".to_string());
            output_files.push("..".to_string());
        }

        if let Some(children) = world.get::<VFSChildren>(target_entity) {
            output_files.extend(
                children
                    .0
                    .iter()
                    .filter_map(|&child| world.get::<FileName>(child))
                    .filter(|name| show_all || !name.0.starts_with('.'))
                    .map(|name| name.0.clone()),
            );
        }

        if let Some(query_dir) = world.get::<VFSQueryDirectory>(target_entity) {
            if let Some(app_registry) = world.get_resource::<AppTypeRegistry>() {
                let registry = app_registry.read();

                for archetype in world.archetypes().iter() {
                    if archetype.is_empty()
                        || archetype.id() == bevy_ecs::archetype::ArchetypeId::EMPTY
                    {
                        continue;
                    }

                    let matches = query_dir.required_components.iter().all(|req_comp| {
                        archetype.components().iter().any(|comp_id| {
                            world
                                .components()
                                .get_info(*comp_id)
                                .and_then(|info| info.type_id())
                                .and_then(|t_id| registry.get(t_id))
                                .is_some_and(|reg| {
                                    reg.type_info().type_path_table().short_path() == req_comp
                                })
                        })
                    });

                    if matches {
                        output_files.extend(
                            archetype
                                .entities()
                                .iter()
                                .map(|e| format!("{}v{}", e.id().index(), e.id().generation())),
                        );
                    }
                }
            }
        }

        let link_target = world
            .get::<VFSEntityLink>(target_entity)
            .map(|l| l.0)
            .or_else(|| {
                ECSFileSystem::parse_entity_string(
                    terminal
                        .current_directory
                        .last()
                        .map(|s| s.as_str())
                        .unwrap_or(""),
                )
            });

        if let Some(ecs_entity) = link_target {
            if let Ok(linked_entity_ref) = world.get_entity(ecs_entity) {
                if let Some(app_registry) = world.get_resource::<AppTypeRegistry>() {
                    let registry = app_registry.read();

                    output_files.extend(
                        linked_entity_ref
                            .archetype()
                            .components()
                            .into_iter()
                            .filter_map(|comp_id| world.components().get_info(*comp_id))
                            .filter_map(|info| info.type_id())
                            .filter_map(|t_id| registry.get(t_id))
                            .map(|reg| reg.type_info().type_path_table().short_path().to_string()),
                    );
                }
            }
        }

        output_files.sort();
        terminal.history.push(output_files.join("  "));
        CommandResult::Handled
    }
}

pub struct CdCommand;
impl Command for CdCommand {
    fn name() -> &'static str {
        "cd"
    }
    fn execute(terminal: &mut Terminal, world: &mut World, args: &[&str]) -> CommandResult {
        let target_directory = args.first().copied().unwrap_or("/");

        let mut new_path = if target_directory.starts_with('/') {
            Vec::new()
        } else {
            terminal.current_directory.clone()
        };

        for path_part in target_directory.split('/') {
            match path_part {
                "" | "." => {}
                ".." => {
                    new_path.pop();
                }
                _ => new_path.push(path_part.to_string()),
            }
        }

        let path_refs: Vec<&str> = new_path.iter().map(|s| s.as_str()).collect();

        if let Some(node_entity) =
            ECSFileSystem::resolve_path(terminal.root_entity, world, &path_refs)
        {
            let is_dir = world.get::<VFSDirectory>(node_entity).is_some()
                || world.get::<VFSQueryDirectory>(node_entity).is_some()
                || world.get::<VFSEntityLink>(node_entity).is_some();

            if is_dir {
                terminal.current_directory = new_path;
            } else {
                terminal
                    .history
                    .push(format!("cd: {}: Not a directory", target_directory));
            }
        } else if let Some(last_part) = new_path.last() {
            if ECSFileSystem::parse_entity_string(last_part).is_some() {
                let mut parent_path = new_path.clone();
                parent_path.pop();
                let parent_refs: Vec<&str> = parent_path.iter().map(|s| s.as_str()).collect();

                if let Some(parent_node) =
                    ECSFileSystem::resolve_path(terminal.root_entity, world, &parent_refs)
                {
                    if world.get::<VFSQueryDirectory>(parent_node).is_some() {
                        terminal.current_directory = new_path;
                        return CommandResult::Handled;
                    }
                }
            }
            terminal.history.push(format!(
                "cd: {}: No such file or directory",
                target_directory
            ));
        }

        CommandResult::Handled
    }
}

pub struct CatCommand;
impl Command for CatCommand {
    fn name() -> &'static str {
        "cat"
    }
    fn execute(terminal: &mut Terminal, world: &mut World, args: &[&str]) -> CommandResult {
        let Some(&target_file) = args.first() else {
            terminal
                .history
                .push("cat: missing file operand".to_string());
            return CommandResult::Handled;
        };

        let mut file_path = terminal.current_directory.clone();
        for part in target_file.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    file_path.pop();
                }
                _ => file_path.push(part.to_string()),
            }
        }

        if file_path.is_empty() {
            terminal
                .history
                .push(format!("cat: {}: Is a directory", target_file));
            return CommandResult::Handled;
        }

        let path_refs: Vec<&str> = file_path.iter().map(|s| s.as_str()).collect();

        if let Some(node_entity) =
            ECSFileSystem::resolve_path(terminal.root_entity, world, &path_refs)
        {
            if let Some(text_file) = world.get::<VFSTextFile>(node_entity) {
                terminal.history.push(text_file.0.clone());
            } else {
                terminal.history.push(format!(
                    "cat: {}: Is a directory or unreadable",
                    target_file
                ));
            }
            return CommandResult::Handled;
        }

        let component_name = file_path.pop().unwrap();
        let parent_refs: Vec<&str> = file_path.iter().map(|s| s.as_str()).collect();
        let mut target_ecs_entity = None;

        if let Some(parent_node) =
            ECSFileSystem::resolve_path(terminal.root_entity, world, &parent_refs)
        {
            if let Some(link) = world.get::<VFSEntityLink>(parent_node) {
                target_ecs_entity = Some(link.0);
            }
        } else if let Some(last_part) = file_path.last() {
            target_ecs_entity = ECSFileSystem::parse_entity_string(last_part);
        }

        let Some(ecs_entity) = target_ecs_entity else {
            terminal
                .history
                .push(format!("cat: {}: No such file or directory", target_file));
            return CommandResult::Handled;
        };

        let Some(app_registry) = world.get_resource::<AppTypeRegistry>() else {
            terminal
                .history
                .push("cat: AppTypeRegistry resource missing".to_string());
            return CommandResult::Handled;
        };

        let registry = app_registry.read();

        if let Ok(linked_entity) = world.get_entity(ecs_entity) {
            let reflected_string =
                linked_entity
                    .archetype()
                    .components()
                    .iter()
                    .find_map(|comp_id| {
                        let type_id = world.components().get_info(*comp_id)?.type_id()?;
                        let reg = registry.get(type_id)?;

                        if reg.type_info().type_path_table().short_path() == component_name {
                            let reflect_comp = reg.data::<ReflectComponent>()?;
                            let reflected_data = reflect_comp.reflect(linked_entity)?;
                            Some(format!("{:#?}", reflected_data))
                        } else {
                            None
                        }
                    });

            if let Some(data) = reflected_string {
                terminal.history.push(data);
            } else {
                terminal
                    .history
                    .push(format!("cat: error reading component {}", component_name));
            }
            return CommandResult::Handled;
        }

        terminal
            .history
            .push(format!("cat: {}: No such file or directory", target_file));
        CommandResult::Handled
    }
}

pub struct HelpCommand;
impl Command for HelpCommand {
    fn name() -> &'static str {
        "help"
    }
    fn execute(terminal: &mut Terminal, world: &mut World, _args: &[&str]) -> CommandResult {
        if let Some(registry) = world.get_resource::<CommandRegistry>() {
            let mut command_strings: Vec<String> = registry.commands.keys().cloned().collect();
            command_strings.sort();

            terminal.history.push(format!(
                "Available built-in commands: {}",
                command_strings.join(", ")
            ));
        } else {
            terminal
                .history
                .push("Error: CommandRegistry missing from World.".to_string());
        }
        CommandResult::Handled
    }
}
