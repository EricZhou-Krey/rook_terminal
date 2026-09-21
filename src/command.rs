use crate::file_system::{Binary, CommandRegistry, ECSFileSystem, Text};
use crate::Terminal;
use bevy_ecs::prelude::*;
use bevy_ecs::system::In;
use std::collections::{HashSet, VecDeque};
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    Handled(Option<String>),
    Unhandled(String, Vec<String>),
}

pub struct ClearCommand;
impl ClearCommand {
    pub fn name() -> &'static str {
        "clear"
    }
    pub fn execute(_args: In<Vec<String>>, mut terminal: ResMut<Terminal>) {
        terminal.history.clear();
        terminal.push_command_result(&CommandResult::Handled(None));
    }
}

pub struct PwdCommand;
impl PwdCommand {
    pub fn name() -> &'static str {
        "pwd"
    }
    pub fn execute(_args: In<Vec<String>>, mut terminal: ResMut<Terminal>) {
        let path: String = format!("/{}", terminal.current_directory.join("/"));
        terminal.push_command_result(&CommandResult::Handled(Some(path)));
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
    pub fn from_char(c: &char) -> Result<Self, &'static str> {
        match c {
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

    fn parse_flags(args: &[&str]) -> HashSet<LsFlags> {
        args.iter()
            .filter_map(|&arg: &&str| {
                let mut arg_chars: Chars = arg.chars();
                let character: char = arg_chars.next()?;
                if character != '-' {
                    return None;
                }

                Some(
                    arg_chars
                        .filter_map(|c: char| LsFlags::from_char(&c).ok())
                        .collect::<HashSet<LsFlags>>(),
                )
            })
            .flatten()
            .collect()
    }

    fn parse_paths<'a>(args: &[&'a str]) -> Vec<&'a str> {
        let explict_paths: Vec<&str> = args
            .iter()
            .filter_map(|&arg: &&str| {
                let mut arg_chars: Chars = arg.chars();
                let character: char = arg_chars.next()?;
                if character == '-' {
                    return None;
                }
                Some(arg)
            })
            .collect();

        if explict_paths.is_empty() {
            vec!["."]
        } else {
            explict_paths
        }
    }

    fn format_size(size: usize, human_readable: bool) -> String {
        if !human_readable {
            return format!("{size}");
        }

        let units: [&str; 4] = ["B", "K", "M", "G"];
        let mut float_size: f64 = size as f64;
        let mut unit_index: usize = 0;

        while float_size >= 1024.0 && unit_index < units.len() - 1 {
            float_size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{size}B")
        } else {
            format!("{:.1}{}", float_size, units[unit_index])
        }
    }

    fn gather_entities_to_list(
        path: &str,
        target_entity: Entity,
        flags: &HashSet<LsFlags>,
        file_system: &ECSFileSystem,
        queue: &mut VecDeque<(String, Entity)>,
    ) -> Vec<(String, Entity)> {
        if flags.contains(&LsFlags::Directory) {
            return vec![(path.to_string(), target_entity)];
        }

        let mut list: Vec<(String, Entity)> = Vec::new();
        if flags.contains(&LsFlags::ShowAll) {
            list.push((".".to_string(), target_entity));

            let parent: Entity = file_system.parent(target_entity).unwrap_or(target_entity);
            list.push(("..".to_string(), parent));
        }

        let children: Vec<Entity> = file_system.children(target_entity);
        for child in children {
            let name: String = file_system.entity_vfs_name(child);

            if flags.contains(&LsFlags::ShowAll) || !name.starts_with('.') {
                list.push((name.clone(), child));

                if flags.contains(&LsFlags::Recursive) {
                    let child_children: Vec<Entity> = file_system.children(child);
                    if !child_children.is_empty() {
                        let new_path: String = if path == "." {
                            name
                        } else {
                            format!("{}/{}", path, name)
                        };
                        queue.push_back((new_path, child));
                    }
                }
            }
        }
        list
    }

    fn format_single_entity(
        mut name: String,
        entity: Entity,
        flags: &HashSet<LsFlags>,
        file_system: &ECSFileSystem,
    ) -> String {
        let entity_children: Vec<Entity> = file_system.children(entity);

        if flags.contains(&LsFlags::Classify) && !entity_children.is_empty() {
            name.push('/');
        }

        let mut parts: Vec<String> = Vec::new();

        if flags.contains(&LsFlags::EntityId) {
            parts.push(format!("{:>5}v{:<2}", entity.index(), entity.generation()));
        }

        let size_in_bytes: usize = file_system.compute_size(entity);
        let mut component_names: Vec<String> = Vec::new();

        if flags.contains(&LsFlags::ComponentView) {
            if let Ok(Some(location)) = file_system.entities.get(entity) &&
            let Some(archetype) = file_system.archetypes.get(location.archetype_id) {
                for component_id in archetype.components() {
                    if let Some(info) = file_system.components.get_info(*component_id) {
                        let full_name = info.name();
                        let short_name = full_name.split("::").last().unwrap_or(&full_name);
                        component_names.push(short_name.to_string());
                    }
                }
            }
            component_names.sort();
            parts.push(format!("[{}]", component_names.join(", ")));
        }

        if flags.contains(&LsFlags::ByteSize) || flags.contains(&LsFlags::LongList) {
            let formatted_size: String =
                Self::format_size(size_in_bytes, flags.contains(&LsFlags::HumanReadable));

            if flags.contains(&LsFlags::LongList) {
                let type_char: char = if !entity_children.is_empty() {
                    'd'
                } else {
                    '-'
                };
                parts.push(format!("{} {:>5}", type_char, formatted_size));
            } else if flags.contains(&LsFlags::ByteSize) {
                parts.push(format!("{:>5}", formatted_size));
            }
        }

        parts.push(name);
        parts.join(" ")
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

        let formatted_output: String =
            if flags.contains(&LsFlags::OnePerLine) || flags.contains(&LsFlags::LongList) {
                output_lines.join("\n")
            } else {
                output_lines.join("  ")
            };

        let mut block_output: String = String::new();
        if !flags.contains(&LsFlags::Labbeled) {
            block_output.push_str(&format!("{}:\n", path));
        }

        block_output.push_str(&formatted_output);
        block_output
    }

    pub fn execute(
        args: In<Vec<String>>,
        mut terminal: ResMut<Terminal>,
        file_system: ECSFileSystem,
    ) {
        let args_vec: Vec<String> = args.0;
        let args_refs: Vec<&str> = args_vec.iter().map(String::as_str).collect();
        let args: &[&str] = &args_refs;

        let root_entity: Entity = terminal.root_entity;

        let mut flags: HashSet<LsFlags> = Self::parse_flags(args);
        let directory_paths: Vec<&str> = Self::parse_paths(args);

        if directory_paths.len() == 1 && !flags.contains(&LsFlags::Recursive) {
            flags.insert(LsFlags::Labbeled);
        }

        let mut queue: VecDeque<(String, Entity)> = VecDeque::new();
        let mut visited: HashSet<Entity> = HashSet::new();

        let mut all_directory_outputs: Vec<String> = Vec::new();
        let mut final_messages: Vec<String> = Vec::new();

        for path in directory_paths {
            if let Some(target_entity) = file_system.resolve_path(root_entity, path) {
                queue.push_back((path.to_string(), target_entity));
            } else {
                final_messages.push(format!(
                    "ls: cannot access '{}': No such file or directory",
                    path
                ));
            }
        }

        while let Some((path, target_entity)) = queue.pop_front() {
            if !visited.insert(target_entity) {
                continue;
            }

            let entities_to_list: Vec<(String, Entity)> = Self::gather_entities_to_list(
                &path,
                target_entity,
                &flags,
                &file_system,
                &mut queue,
            );

            let output_lines: Vec<String> = entities_to_list
                .into_iter()
                .map(|(name, entity)| {
                    Self::format_single_entity(name, entity, &flags, &file_system)
                })
                .collect();

            let block_output: String = Self::format_directory_block(&path, output_lines, &flags);

            if !block_output.is_empty() {
                all_directory_outputs.push(block_output);
            }
        }

        if !all_directory_outputs.is_empty() {
            final_messages.push(all_directory_outputs.join("\n\n"));
        }

        let result: CommandResult = if !final_messages.is_empty() {
            CommandResult::Handled(Some(final_messages.join("\n")))
        } else {
            CommandResult::Handled(None)
        };
        terminal.push_command_result(&result);
    }
}

pub struct CdCommand;
impl CdCommand {
    pub fn name() -> &'static str {
        "cd"
    }

    fn parse_target_path<'a>(args: &[&'a str]) -> Option<&'a str> {
        args.iter().find(|&&arg| !arg.starts_with('-')).copied()
    }

    fn compute_new_path(current_dir: &[String], target: &str) -> Vec<String> {
        let mut new_path: Vec<String> = if target.starts_with('/') {
            Vec::new()
        } else {
            current_dir.to_vec()
        };

        for part in target.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    new_path.pop();
                }
                _ => {
                    new_path.push(part.to_string());
                }
            }
        }
        new_path
    }

    pub fn execute(
        args: In<Vec<String>>,
        mut terminal: ResMut<Terminal>,
        file_system: ECSFileSystem,
    ) {
        let args_vec: Vec<String> = args.0;
        let args_refs: Vec<&str> = args_vec.iter().map(String::as_str).collect();
        let args: &[&str] = &args_refs;

        let root_entity: Entity = terminal.root_entity;
        let current_directory: Vec<String> = terminal.current_directory.clone();

        let target: &str = Self::parse_target_path(args).unwrap_or("/");
        let new_path: Vec<String> = Self::compute_new_path(&current_directory, target);

        let result: CommandResult = if let Some(target_entity) =
            file_system.resolve_path(root_entity, &new_path.join("/"))
        {
            let has_children: bool = !file_system.children(target_entity).is_empty();

            if !has_children {
                CommandResult::Handled(Some(format!("cd: {}: Not a directory", target)))
            } else {
                terminal.current_directory = new_path;
                CommandResult::Handled(None)
            }
        } else {
            CommandResult::Handled(Some(format!("cd: {}: No such file or directory", target)))
        };

        terminal.push_command_result(&result);
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Debug, Eq)]
pub enum CatFlags {
    NumberLines,
    ShowEnds,
}

impl CatFlags {
    pub fn from_char(c: &char) -> Result<Self, &'static str> {
        match c {
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

    fn parse_flags(args: &[&str]) -> HashSet<CatFlags> {
        args.iter()
            .filter_map(|&arg: &&str| {
                let mut arg_chars: Chars = arg.chars();
                let character: char = arg_chars.next()?;
                if character != '-' {
                    return None;
                }

                Some(
                    arg_chars
                        .filter_map(|char_item: char| CatFlags::from_char(&char_item).ok())
                        .collect::<HashSet<CatFlags>>(),
                )
            })
            .flatten()
            .collect()
    }

    fn parse_paths<'a>(args: &[&'a str]) -> Vec<&'a str> {
        args.iter()
            .filter(|&&arg| !arg.starts_with('-'))
            .copied()
            .collect()
    }

    fn format_content(content: &str, flags: &HashSet<CatFlags>) -> String {
        if flags.is_empty() {
            return content.to_string();
        }

        let mut output: Vec<String> = Vec::new();
        for (index, line) in content.lines().enumerate() {
            let mut formatted_line: String = String::new();

            if flags.contains(&CatFlags::NumberLines) {
                formatted_line.push_str(&format!("{:>6}  ", index + 1));
            }

            formatted_line.push_str(line);

            if flags.contains(&CatFlags::ShowEnds) {
                formatted_line.push('$');
            }

            output.push(formatted_line);
        }
        output.join("\n")
    }

    fn build_absolute_path(current_dir: &[String], target: &str) -> String {
        let mut parts: Vec<String> = if target.starts_with('/') {
            Vec::new()
        } else {
            current_dir.to_vec()
        };

        for part in target.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                _ => {
                    parts.push(part.to_string());
                }
            }
        }
        parts.join("/")
    }

    pub fn execute(
        args: In<Vec<String>>,
        mut terminal: ResMut<Terminal>,
        file_system: ECSFileSystem,
        texts: Query<&Text>,
        binaries: Query<&Binary>,
    ) {
        let args_vec: Vec<String> = args.0;
        let args_refs: Vec<&str> = args_vec.iter().map(String::as_str).collect();
        let args: &[&str] = &args_refs;

        let root_entity: Entity = terminal.root_entity;
        let current_directory: Vec<String> = terminal.current_directory.clone();

        let flags: HashSet<CatFlags> = Self::parse_flags(args);
        let paths: Vec<&str> = Self::parse_paths(args);

        if paths.is_empty() {
            let result: CommandResult =
                CommandResult::Handled(Some("cat: missing operand".to_string()));
            terminal.push_command_result(&result);
            return;
        }

        let mut output_blocks: Vec<String> = Vec::new();

        for path in paths {
            let absolute_path: String = Self::build_absolute_path(&current_directory, path);

            if let Some(target_entity) = file_system.resolve_path(root_entity, &absolute_path) {
                if let Ok(text_comp) = texts.get(target_entity) {
                    output_blocks.push(Self::format_content(&text_comp.text, &flags));
                } else if binaries.get(target_entity).is_ok() {
                    output_blocks.push(format!("cat: {}: cannot display binary file", path));
                } else {
                    let children: Vec<Entity> = file_system.children(target_entity);
                    if !children.is_empty() {
                        output_blocks.push(format!("cat: {}: Is a directory", path));
                    } else {
                        output_blocks.push(format!("cat: {}: No text data", path));
                    }
                }
            } else {
                output_blocks.push(format!("cat: {}: No such file or directory", path));
            }
        }

        let result: CommandResult = if !output_blocks.is_empty() {
            CommandResult::Handled(Some(output_blocks.join("\n")))
        } else {
            CommandResult::Handled(None)
        };
        terminal.push_command_result(&result);
    }
}

pub struct HelpCommand;
impl HelpCommand {
    pub fn name() -> &'static str {
        "help"
    }

    pub fn execute(
        _args: In<Vec<String>>,
        mut terminal: ResMut<Terminal>,
        registry: Res<CommandRegistry>,
    ) {
        let mut command_names: Vec<String> = registry.commands.keys().cloned().collect();
        command_names.sort();

        let result: CommandResult = CommandResult::Handled(Some(format!(
            "Available built-in commands: {}",
            command_names.join(", ")
        )));
        terminal.push_command_result(&result);
    }
}

pub struct RookCommand;
impl RookCommand {
    pub fn name() -> &'static str {
        "rook"
    }

    pub fn execute(_args: In<Vec<String>>, mut terminal: ResMut<Terminal>) {
        let colored_icon: String = format!("\x1b[96m{}\x1b[0m", crate::style_sheet::ICON);
        let result: CommandResult = CommandResult::Handled(Some(colored_icon));
        terminal.push_command_result(&result);
    }
}
