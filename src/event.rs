use bevy_ecs::prelude::*;

use crate::file_system::{CommandRegistry, ECSFileSystem};
use crate::style::TerminalStyle;
use crate::{TerminalInputState, TerminalSession, TerminalViewState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Event, Debug, Clone)]
pub struct TerminalCommandRequested {
    pub raw_command: String,
}

#[derive(Event, Debug, Clone)]
pub struct TerminalOutput {
    pub content: String,
    pub level: OutputLevel,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct ClearTerminalHistory;

#[derive(Event, Debug, Clone)]
pub struct ChangeDirectoryRequested {
    pub path: Vec<String>,
}

#[derive(Event, Debug, Clone)]
pub struct DirectoryChanged {
    pub path: Vec<String>,
}

#[derive(Event, Debug, Clone)]
pub struct TerminalStyleChangeRequested {
    pub style: TerminalStyle,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct TerminalFocusRequested;

#[derive(Event, Debug, Clone, Copy)]
pub struct TerminalInputCleared;

pub fn execute_command(
    command_event: On<TerminalCommandRequested>,
    session: Res<TerminalSession>,
    registry: Res<CommandRegistry>,
    mut commands: Commands,
) {
    let raw_command: &str = command_event.raw_command.trim();

    if raw_command.is_empty() {
        return;
    }

    let prompt: String = format!("{}$ {}", session.current_directory.join("/"), raw_command);

    commands.trigger(TerminalOutput {
        content: prompt,
        level: OutputLevel::Info,
    });

    let command_parts: Vec<&str> = raw_command.split_whitespace().collect();

    let Some(command_name) = command_parts.first() else {
        return;
    };

    let command_arguments: Vec<String> = command_parts[1..]
        .iter()
        .map(|argument: &&str| (*argument).to_string())
        .collect();

    if let Some(&system_id) = registry.commands.get(*command_name) {
        commands.run_system_with(system_id, command_arguments);
    } else {
        commands.trigger(TerminalOutput {
            content: format!("unknown command: {command_name}"),
            level: OutputLevel::Error,
        });
    }
}

pub fn append_terminal_output(
    output_event: On<TerminalOutput>,
    mut session: ResMut<TerminalSession>,
) {
    session.history.push(output_event.content.clone());
}

pub fn clear_terminal_history(
    _clear_reader: On<ClearTerminalHistory>,
    mut session: ResMut<TerminalSession>,
) {
    session.history.clear();
}

pub fn change_terminal_directory(
    directory_event: On<ChangeDirectoryRequested>,
    mut session: ResMut<TerminalSession>,
    file_system: ECSFileSystem,
    mut commands: Commands,
) {
    let requested_path: &Vec<String> = &directory_event.path;

    let Some(target_entity): Option<Entity> =
        file_system.resolve_path(session.root_entity, &requested_path.join("/"))
    else {
        commands.trigger(TerminalOutput {
            content: format!(
                "cd: {}: No such file or directory",
                requested_path.join("/")
            ),
            level: OutputLevel::Error,
        });

        return;
    };

    let target_children: Vec<Entity> = file_system.children(target_entity);

    if target_children.is_empty() {
        commands.trigger(TerminalOutput {
            content: format!("cd: {}: Not a directory", requested_path.join("/")),
            level: OutputLevel::Error,
        });

        return;
    }

    session.current_directory = requested_path.clone();

    commands.trigger(DirectoryChanged {
        path: requested_path.clone(),
    });
}

pub fn update_terminal_style(
    style_event: On<TerminalStyleChangeRequested>,
    mut view_state: ResMut<TerminalViewState>,
) {
    view_state.style = style_event.style.clone();
}

pub fn request_terminal_focus(
    _focus_event: On<TerminalFocusRequested>,
    mut input_state: ResMut<TerminalInputState>,
) {
    input_state.focus_requested = true;
}

pub fn clear_terminal_input(
    _clear_reader: On<TerminalInputCleared>,
    mut input_state: ResMut<TerminalInputState>,
) {
    input_state.input.clear();
}
