use crate::file_system::{Binary, CommandRegistry, ECSFileSystem, Text};
use crate::Terminal;
use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use std::collections::{HashSet, VecDeque};
use std::str::Chars;

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
        let path: String = format!("/{}", terminal.current_directory.join("/"));
        terminal.history.push(path);
        CommandResult::Handled
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
    fn parse_flags(args: &[&str]) -> HashSet<LsFlags> {
        args.iter()
            .filter_map(|&arg: &&str| {
                let mut arg_chars: Chars = arg.chars();
                let c = arg_chars.next()?;
                if c != '-' {
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
                let c = arg_chars.next()?;
                if c == '-' {
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
        let mut s: f64 = size as f64;
        let mut u: usize = 0;

        while s >= 1024.0 && u < units.len() - 1 {
            s /= 1024.0;
            u += 1;
        }

        if u == 0 {
            format!("{size}B")
        } else {
            format!("{:.1}{}", s, units[u])
        }
    }

    fn gather_entities_to_list(
        path: &str,
        target_entity: Entity,
        flags: &HashSet<LsFlags>,
        world: &World,
        queue: &mut VecDeque<(String, Entity)>,
    ) -> Vec<(String, Entity)> {
        if flags.contains(&LsFlags::Directory) {
            return vec![(path.to_string(), target_entity)];
        }

        let mut list: Vec<(String, Entity)> = Vec::new();
        if flags.contains(&LsFlags::ShowAll) {
            list.push((".".to_string(), target_entity));

            let parent: Entity =
                ECSFileSystem::parent(target_entity, world).unwrap_or(target_entity);
            list.push(("..".to_string(), parent));
        }

        let children: Vec<Entity> = ECSFileSystem::children(target_entity, world);
        for child in children {
            let name: String = ECSFileSystem::entity_vfs_name(child, world);

            if flags.contains(&LsFlags::ShowAll) || !name.starts_with('.') {
                list.push((name.clone(), child));

                if flags.contains(&LsFlags::Recursive) {
                    let child_children: Vec<Entity> = ECSFileSystem::children(child, world);
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
        world: &World,
    ) -> String {
        let entity_children: Vec<Entity> = ECSFileSystem::children(entity, world);

        if flags.contains(&LsFlags::Classify) && !entity_children.is_empty() {
            name.push('/');
        }

        let mut parts: Vec<String> = Vec::new();

        if flags.contains(&LsFlags::EntityId) {
            parts.push(format!("{:>5}v{:<2}", entity.index(), entity.generation()));
        }

        let size_in_bytes: usize = ECSFileSystem::compute_size(entity, world);
        let mut component_names: Vec<String> = Vec::new();

        if flags.contains(&LsFlags::ComponentView) {
            if let Ok(entity_ref) = world.get_entity(entity) {
                for comp_id in entity_ref.archetype().components() {
                    if let Some(info) = world.components().get_info(*comp_id) {
                        let full_name: &String = &info.name().as_string();
                        let short_name: &str = full_name.split("::").last().unwrap_or(full_name);
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
                let type_char = if !entity_children.is_empty() {
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

        let formatted_output =
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
}

impl Command for LsCommand {
    fn name() -> &'static str {
        "ls"
    }

    fn execute(terminal: &mut Terminal, world: &mut World, args: &[&str]) -> CommandResult {
        let mut flags: HashSet<LsFlags> = Self::parse_flags(args);
        let directory_paths: Vec<&str> = Self::parse_paths(args);

        if directory_paths.len() == 1 && !flags.contains(&LsFlags::Recursive) {
            flags.insert(LsFlags::Labbeled);
        }

        let mut queue: VecDeque<(String, Entity)> = VecDeque::new();
        let mut visited: HashSet<Entity> = HashSet::new();
        let mut all_directory_outputs: Vec<String> = Vec::new();

        for path in directory_paths {
            if let Some(target_entity) =
                ECSFileSystem::resolve_path(terminal.root_entity, world, path)
            {
                queue.push_back((path.to_string(), target_entity));
            } else {
                terminal.history.push(format!(
                    "ls: cannot access '{}': No such file or directory",
                    path
                ));
            }
        }

        while let Some((path, target_entity)) = queue.pop_front() {
            if !visited.insert(target_entity) {
                continue;
            }

            let entities_to_list: Vec<(String, Entity)> =
                Self::gather_entities_to_list(&path, target_entity, &flags, world, &mut queue);

            let output_lines: Vec<String> = entities_to_list
                .into_iter()
                .map(|(name, entity)| Self::format_single_entity(name, entity, &flags, world))
                .collect();

            let block_output: String = Self::format_directory_block(&path, output_lines, &flags);

            if !block_output.is_empty() {
                all_directory_outputs.push(block_output);
            }
        }

        if !all_directory_outputs.is_empty() {
            terminal.history.push(all_directory_outputs.join("\n\n"));
        }

        CommandResult::Handled
    }
}

pub struct CdCommand;

impl CdCommand {
    fn parse_target_path<'a>(args: &[&'a str]) -> Option<&'a str> {
        args.iter().find(|&&arg| !arg.starts_with('-')).copied()
    }

    fn compute_new_path(current_dir: &[String], target: &str) -> Vec<String> {
        let mut new_path = if target.starts_with('/') {
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
}

impl Command for CdCommand {
    fn name() -> &'static str {
        "cd"
    }

    fn execute(terminal: &mut Terminal, world: &mut World, args: &[&str]) -> CommandResult {
        let target: &str = Self::parse_target_path(args).unwrap_or("/");

        let new_path: Vec<String> = Self::compute_new_path(&terminal.current_directory, target);
        if let Some(target_entity) =
            ECSFileSystem::resolve_path(terminal.root_entity, world, &new_path.join("/"))
        {
            let has_children = !ECSFileSystem::children(target_entity, world).is_empty();

            if !has_children {
                terminal
                    .history
                    .push(format!("cd: {}: Not a directory", target));
            } else {
                terminal.current_directory = new_path;
            }
        } else {
            terminal
                .history
                .push(format!("cd: {}: No such file or directory", target));
        }

        CommandResult::Handled
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
    fn parse_flags(args: &[&str]) -> HashSet<CatFlags> {
        args.iter()
            .filter_map(|&arg: &&str| {
                let mut arg_chars: Chars = arg.chars();
                let c = arg_chars.next()?;
                if c != '-' {
                    return None;
                }

                Some(
                    arg_chars
                        .filter_map(|c: char| CatFlags::from_char(&c).ok())
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
        for (i, line) in content.lines().enumerate() {
            let mut formatted_line: String = String::new();

            if flags.contains(&CatFlags::NumberLines) {
                formatted_line.push_str(&format!("{:>6}  ", i + 1));
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
}

impl Command for CatCommand {
    fn name() -> &'static str {
        "cat"
    }

    fn execute(terminal: &mut Terminal, world: &mut World, args: &[&str]) -> CommandResult {
        let flags: HashSet<CatFlags> = Self::parse_flags(args);
        let paths: Vec<&str> = Self::parse_paths(args);

        if paths.is_empty() {
            terminal.history.push("cat: missing operand".to_string());
            return CommandResult::Handled;
        }

        let mut output_blocks: Vec<String> = Vec::new();

        for path in paths {
            let absolute_path: String =
                Self::build_absolute_path(&terminal.current_directory, path);

            if let Some(target_entity) =
                ECSFileSystem::resolve_path(terminal.root_entity, world, &absolute_path)
            {
                if let Some(text_comp) = world.get::<Text>(target_entity) {
                    output_blocks.push(Self::format_content(&text_comp.text, &flags));
                } else if world.get::<Binary>(target_entity).is_some() {
                    output_blocks.push(format!("cat: {}: cannot display binary file", path));
                } else {
                    let children = ECSFileSystem::children(target_entity, world);
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

        if !output_blocks.is_empty() {
            terminal.history.push(output_blocks.join("\n"));
        }

        CommandResult::Handled
    }
}

pub struct HelpCommand;
impl Command for HelpCommand {
    fn name() -> &'static str {
        "help"
    }
    fn execute(terminal: &mut Terminal, world: &mut World, _args: &[&str]) -> CommandResult {
        let Some(registry): Option<&CommandRegistry> = world.get_resource::<CommandRegistry>()
        else {
            terminal.history.push(
                "Error: CommandRegistry missing from World, ensure this is the cpu world"
                    .to_string(),
            );
            return CommandResult::Handled;
        };

        let mut command_names: Vec<String> = registry.commands.keys().cloned().collect();
        command_names.sort();

        terminal.history.push(format!(
            "Available built-in commands: {}",
            command_names.join(", ")
        ));

        CommandResult::Handled
    }
}

pub struct RookCommand;
impl Command for RookCommand {
    fn name() -> &'static str {
        "rook"
    }

    fn execute(terminal: &mut Terminal, _world: &mut World, _args: &[&str]) -> CommandResult {
        let colored_icon: String = format!("\x1b[96m{}\x1b[0m", crate::style_sheet::ICON);
        terminal.history.push(colored_icon);
        CommandResult::Handled
    }
}
