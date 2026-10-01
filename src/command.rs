use crate::{
    event::{
        ChangeDirectoryRequested,
        ClearTerminalHistory,
        OutputLevel,
        TerminalOutput,
    },
    file_system::{Binary, CommandRegistry, ECSFileSystem, Text},
    TerminalSession,
};

use bevy_ecs::prelude::*;
use std::collections::{HashSet, VecDeque};

pub struct ClearCommand;

impl ClearCommand {
    pub fn name() -> &'static str {
        "clear"
    }

    pub fn execute(
        _arguments: In<Vec<String>>,
        mut commands: Commands,
    ) {
        commands.trigger(ClearTerminalHistory);
    }
}

pub struct PwdCommand;

impl PwdCommand {
    pub fn name() -> &'static str {
        "pwd"
    }

    pub fn execute(
        _arguments: In<Vec<String>>,
        session: Res<TerminalSession>,
        mut commands: Commands,
    ) {
        let current_path: String = format!("/{}", session.current_directory.join("/"));

        commands.trigger(TerminalOutput {
            content: current_path,
            level: OutputLevel::Info,
        });
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Debug, Eq)]
pub enum LsFlags {
    ShowAll,
    Classify,
    LongList,
    Recursive,
    ByteSize,
    EntityId,
    ComponentView,
    OnePerLine,
    Directory,
    Reverse,
    HumanReadable,
    Labbeled,
}

impl LsFlags {
    pub fn from_char(character: &char) -> Result<Self, &'static str> {
        match character {
            'a' => Ok(Self::ShowAll),
            'l' => Ok(Self::LongList),
            'R' => Ok(Self::Recursive),
            's' => Ok(Self::ByteSize),
            'F' => Ok(Self::Classify),
            'i' => Ok(Self::EntityId),
            'c' => Ok(Self::ComponentView),
            '1' => Ok(Self::OnePerLine),
            'd' => Ok(Self::Directory),
            'r' => Ok(Self::Reverse),
            'h' => Ok(Self::HumanReadable),
            _ => Err("Invalid flag for ls"),
        }
    }
}

pub struct LsCommand;

impl LsCommand {
    pub fn name() -> &'static str {
        "ls"
    }

    fn parse_flags(arguments: &[&str]) -> HashSet<LsFlags> {
        arguments
            .iter()
            .filter_map(|argument: &&str| {
                let mut argument_characters = argument.chars();

                if argument_characters.next()? != '-' {
                    return None;
                }

                Some(
                    argument_characters
                        .filter_map(|character: char| LsFlags::from_char(&character).ok())
                        .collect::<HashSet<LsFlags>>(),
                )
            })
            .flatten()
            .collect()
    }

    fn parse_paths<'a>(arguments: &[&'a str]) -> Vec<&'a str> {
        let explicit_paths: Vec<&str> = arguments
            .iter()
            .filter(|argument: &&&'a str| !argument.starts_with('-'))
            .copied()
            .collect();

        if explicit_paths.is_empty() {
            vec!["."]
        } else {
            explicit_paths
        }
    }

    fn format_size(size: usize, human_readable: bool) -> String {
        if !human_readable {
            return size.to_string();
        }

        let units: [&str; 4] = ["B", "K", "M", "G"];
        let mut converted_size: f64 = size as f64;
        let mut unit_index: usize = 0;

        while converted_size >= 1024.0 && unit_index < units.len() - 1 {
            converted_size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{size}B")
        } else {
            format!("{converted_size:.1}{}", units[unit_index])
        }
    }

    fn gather_entities_to_list(
        path: &str,
        target_entity: Entity,
        flags: &HashSet<LsFlags>,
        file_system: &ECSFileSystem,
        traversal_queue: &mut VecDeque<(String, Entity)>,
    ) -> Vec<(String, Entity)> {
        if flags.contains(&LsFlags::Directory) {
            return vec![(path.to_string(), target_entity)];
        }

        let mut entities: Vec<(String, Entity)> = Vec::new();

        if flags.contains(&LsFlags::ShowAll) {
            entities.push((".".to_string(), target_entity));

            let parent_entity: Entity = file_system
                .parent(target_entity)
                .unwrap_or(target_entity);

            entities.push(("..".to_string(), parent_entity));
        }

        for child_entity in file_system.children(target_entity) {
            let child_name: String = file_system.entity_vfs_name(child_entity);

            if flags.contains(&LsFlags::ShowAll) || !child_name.starts_with('.') {
                entities.push((child_name.clone(), child_entity));

                if flags.contains(&LsFlags::Recursive) {
                    let child_children: Vec<Entity> = file_system.children(child_entity);

                    if !child_children.is_empty() {
                        let child_path: String = if path == "." {
                            child_name
                        } else {
                            format!("{path}/{child_name}")
                        };

                        traversal_queue.push_back((child_path, child_entity));
                    }
                }
            }
        }

        entities
    }

    fn format_single_entity(
        mut name: String,
        entity: Entity,
        flags: &HashSet<LsFlags>,
        file_system: &ECSFileSystem,
    ) -> String {
        let child_entities: Vec<Entity> = file_system.children(entity);

        if flags.contains(&LsFlags::Classify) && !child_entities.is_empty() {
            name.push('/');
        }

        let mut output_parts: Vec<String> = Vec::new();

        if flags.contains(&LsFlags::EntityId) {
            output_parts.push(format!(
                "{:>5}v{:<2}",
                entity.index(),
                entity.generation()
            ));
        }

        if flags.contains(&LsFlags::ComponentView) {
            let mut component_names: Vec<String> = Vec::new();

            if let Ok(Some(entity_location)) = file_system.entities.get(entity)
                && let Some(archetype) =
                    file_system.archetypes.get(entity_location.archetype_id)
            {
                for component_id in archetype.components() {
                    if let Some(component_info) =
                        file_system.components.get_info(*component_id)
                    {
                        let full_name: &str = &component_info.name();
                        let short_name: &str = full_name
                            .split("::")
                            .last()
                            .unwrap_or(full_name);

                        component_names.push(short_name.to_string());
                    }
                }
            }

            component_names.sort();
            output_parts.push(format!("[{}]", component_names.join(", ")));
        }

        let entity_size: usize = file_system.compute_size(entity);

        if flags.contains(&LsFlags::ByteSize)
            || flags.contains(&LsFlags::LongList)
        {
            let formatted_size: String = Self::format_size(
                entity_size,
                flags.contains(&LsFlags::HumanReadable),
            );

            if flags.contains(&LsFlags::LongList) {
                let entity_type: char = if child_entities.is_empty() {
                    '-'
                } else {
                    'd'
                };

                output_parts.push(format!("{entity_type} {formatted_size:>5}"));
            } else {
                output_parts.push(format!("{formatted_size:>5}"));
            }
        }

        output_parts.push(name);
        output_parts.join(" ")
    }

    fn format_directory_block(
        path: &str,
        mut output_lines: Vec<String>,
        flags: &HashSet<LsFlags>,
    ) -> String {
        output_lines.sort();

        if flags.contains(&LsFlags::Reverse) {
            output_lines.reverse();
        }

        let formatted_lines: String = if flags.contains(&LsFlags::OnePerLine)
            || flags.contains(&LsFlags::LongList)
        {
            output_lines.join("\n")
        } else {
            output_lines.join("  ")
        };

        if flags.contains(&LsFlags::Labbeled) {
            formatted_lines
        } else {
            format!("{path}:\n{formatted_lines}")
        }
    }

    pub fn execute(
        arguments: In<Vec<String>>,
        session: Res<TerminalSession>,
        file_system: ECSFileSystem,
        mut commands: Commands,
    ) {
        let argument_strings: Vec<String> = arguments.0;
        let argument_references: Vec<&str> = argument_strings
            .iter()
            .map(String::as_str)
            .collect();

        let flags: HashSet<LsFlags> = Self::parse_flags(&argument_references);
        let directory_paths: Vec<&str> = Self::parse_paths(&argument_references);

        let mut effective_flags: HashSet<LsFlags> = flags;

        if directory_paths.len() == 1
            && !effective_flags.contains(&LsFlags::Recursive)
        {
            effective_flags.insert(LsFlags::Labbeled);
        }

        let mut traversal_queue: VecDeque<(String, Entity)> = VecDeque::new();
        let mut visited_entities: HashSet<Entity> = HashSet::new();
        let mut directory_outputs: Vec<String> = Vec::new();
        let mut error_outputs: Vec<String> = Vec::new();

        for directory_path in directory_paths {
            if let Some(target_entity) = file_system.resolve_path(
                session.root_entity,
                directory_path,
            ) {
                traversal_queue.push_back((
                    directory_path.to_string(),
                    target_entity,
                ));
            } else {
                error_outputs.push(format!(
                    "ls: cannot access '{}': No such file or directory",
                    directory_path
                ));
            }
        }

        while let Some((path, target_entity)) = traversal_queue.pop_front() {
            if !visited_entities.insert(target_entity) {
                continue;
            }

            let entities_to_display: Vec<(String, Entity)> = Self::gather_entities_to_list(
                &path,
                target_entity,
                &effective_flags,
                &file_system,
                &mut traversal_queue,
            );

            let output_lines: Vec<String> = entities_to_display
                .into_iter()
                .map(|(name, entity): (String, Entity)| {
                    Self::format_single_entity(
                        name,
                        entity,
                        &effective_flags,
                        &file_system,
                    )
                })
                .collect();

            let directory_output: String = Self::format_directory_block(
                &path,
                output_lines,
                &effective_flags,
            );

            if !directory_output.is_empty() {
                directory_outputs.push(directory_output);
            }
        }

        let mut final_output: Vec<String> = error_outputs;

        if !directory_outputs.is_empty() {
            final_output.push(directory_outputs.join("\n\n"));
        }

        if !final_output.is_empty() {
            commands.trigger(TerminalOutput {
                content: final_output.join("\n"),
                level: OutputLevel::Info,
            });
        }
    }
}

pub struct CdCommand;

impl CdCommand {
    pub fn name() -> &'static str {
        "cd"
    }

    fn parse_target_path<'a>(arguments: &[&'a str]) -> Option<&'a str> {
        arguments
            .iter()
            .find(|argument: &&&'a str| !argument.starts_with('-'))
            .copied()
    }

    fn compute_new_path(current_directory: &[String], target: &str) -> Vec<String> {
        let mut new_path: Vec<String> = if target.starts_with('/') {
            Vec::new()
        } else {
            current_directory.to_vec()
        };

        for path_part in target.split('/') {
            match path_part {
                "" | "." => {}
                ".." => {
                    new_path.pop();
                }
                path_part => {
                    new_path.push(path_part.to_string());
                }
            }
        }

        new_path
    }

    pub fn execute(
        arguments: In<Vec<String>>,
        session: Res<TerminalSession>,
        mut commands: Commands,
    ) {
        let argument_strings: Vec<String> = arguments.0;
        let argument_references: Vec<&str> = argument_strings
            .iter()
            .map(String::as_str)
            .collect();

        let target_path: &str = Self::parse_target_path(&argument_references)
            .unwrap_or("/");

        let new_path: Vec<String> = Self::compute_new_path(
            &session.current_directory,
            target_path,
        );

        commands.trigger(ChangeDirectoryRequested {
            path: new_path,
        });
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Debug, Eq)]
pub enum CatFlags {
    NumberLines,
    ShowEnds,
}

impl CatFlags {
    pub fn from_char(character: &char) -> Result<Self, &'static str> {
        match character {
            'n' => Ok(Self::NumberLines),
            'E' => Ok(Self::ShowEnds),
            _ => Err("Invalid flag for cat"),
        }
    }
}

pub struct CatCommand;

impl CatCommand {
    pub fn name() -> &'static str {
        "cat"
    }

    fn parse_flags(arguments: &[&str]) -> HashSet<CatFlags> {
        arguments
            .iter()
            .filter_map(|argument: &&str| {
                let mut argument_characters = argument.chars();

                if argument_characters.next()? != '-' {
                    return None;
                }

                Some(
                    argument_characters
                        .filter_map(|character: char| {
                            CatFlags::from_char(&character).ok()
                        })
                        .collect::<HashSet<CatFlags>>(),
                )
            })
            .flatten()
            .collect()
    }

    fn parse_paths<'a>(arguments: &[&'a str]) -> Vec<&'a str> {
        arguments
            .iter()
            .filter(|argument: &&&'a str| !argument.starts_with('-'))
            .copied()
            .collect()
    }

    fn format_content(
        content: &str,
        flags: &HashSet<CatFlags>,
    ) -> String {
        if flags.is_empty() {
            return content.to_string();
        }

        content
            .lines()
            .enumerate()
            .map(|(line_index, line): (usize, &str)| {
                let mut formatted_line: String = String::new();

                if flags.contains(&CatFlags::NumberLines) {
                    formatted_line.push_str(&format!("{:>6}  ", line_index + 1));
                }

                formatted_line.push_str(line);

                if flags.contains(&CatFlags::ShowEnds) {
                    formatted_line.push('$');
                }

                formatted_line
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn build_absolute_path(
        current_directory: &[String],
        target: &str,
    ) -> String {
        let mut path_parts: Vec<String> = if target.starts_with('/') {
            Vec::new()
        } else {
            current_directory.to_vec()
        };

        for path_part in target.split('/') {
            match path_part {
                "" | "." => {}
                ".." => {
                    path_parts.pop();
                }
                path_part => {
                    path_parts.push(path_part.to_string());
                }
            }
        }

        path_parts.join("/")
    }

    pub fn execute(
        arguments: In<Vec<String>>,
        session: Res<TerminalSession>,
        file_system: ECSFileSystem,
        text_query: Query<&Text>,
        binary_query: Query<&Binary>,
        mut commands: Commands,
    ) {
        let argument_strings: Vec<String> = arguments.0;
        let argument_references: Vec<&str> = argument_strings
            .iter()
            .map(String::as_str)
            .collect();

        let flags: HashSet<CatFlags> = Self::parse_flags(&argument_references);
        let paths: Vec<&str> = Self::parse_paths(&argument_references);

        if paths.is_empty() {
            commands.trigger(TerminalOutput {
                content: "cat: missing operand".to_string(),
                level: OutputLevel::Error,
            });
            return;
        }

        let mut output_blocks: Vec<String> = Vec::new();

        for path in paths {
            let absolute_path: String = Self::build_absolute_path(
                &session.current_directory,
                path,
            );

            let Some(target_entity) = file_system.resolve_path(
                session.root_entity,
                &absolute_path,
            ) else {
                output_blocks.push(format!(
                    "cat: {path}: No such file or directory"
                ));
                continue;
            };

            if let Ok(text_component) = text_query.get(target_entity) {
                output_blocks.push(Self::format_content(
                    &text_component.text,
                    &flags,
                ));
            } else if binary_query.get(target_entity).is_ok() {
                output_blocks.push(format!(
                    "cat: {path}: cannot display binary file"
                ));
            } else if !file_system.children(target_entity).is_empty() {
                output_blocks.push(format!("cat: {path}: Is a directory"));
            } else {
                output_blocks.push(format!("cat: {path}: No text data"));
            }
        }

        commands.trigger(TerminalOutput {
            content: output_blocks.join("\n"),
            level: OutputLevel::Info,
        });
    }
}

pub struct HelpCommand;

impl HelpCommand {
    pub fn name() -> &'static str {
        "help"
    }

    pub fn execute(
        _arguments: In<Vec<String>>,
        registry: Res<CommandRegistry>,
        mut commands: Commands,
    ) {
        let mut command_names: Vec<String> =
            registry.commands.keys().cloned().collect();

        command_names.sort();

        commands.trigger(TerminalOutput {
            content: format!(
                "Available built-in commands: {}",
                command_names.join(", ")
            ),
            level: OutputLevel::Info,
        });
    }
}

pub struct RookCommand;

impl RookCommand {
    pub fn name() -> &'static str {
        "rook"
    }

    pub fn execute(
        _arguments: In<Vec<String>>,
        mut commands: Commands,
    ) {
        let colored_icon: String = format!(
            "\x1b[96m{}\x1b[0m",
            crate::style_sheet::ICON
        );

        commands.trigger(TerminalOutput {
            content: colored_icon,
            level: OutputLevel::Success,
        });
    }
}
