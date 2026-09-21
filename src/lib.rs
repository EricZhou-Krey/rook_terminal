use bevy_ecs::{prelude::*, system::{Commands, Res, ResMut}, relationship::RelationshipSourceCollection};
use egui::{Color32, TextStyle};
use crate::{
    command::{CatCommand, CdCommand, ClearCommand, CommandResult, HelpCommand, LsCommand, PwdCommand, RookCommand}, file_system::{CommandRegistry, VFSChildren, VFSFilterExtension, VFSName, VFSQueryChildren}, style_sheet::{
        BACKGROUND_COLOR, BACKGROUND_CORNER_RADIUS, PROMPT_TEXT_COLOR, SELECTION_COLOR,
        TEXT_COLOR, TEXT_STYLE,
    },
};

pub mod command;
pub mod file_system;
pub mod style_sheet;

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalStyle {
    pub background_color: Color32,
    pub background_corner_radius: f32,
    pub prompt_text_color: Color32,
    pub selection_color: Color32,
    pub text_color: Color32,
    pub text_style: TextStyle,
}

impl Default for TerminalStyle {
    fn default() -> Self {
        Self {
            background_color: BACKGROUND_COLOR,
            background_corner_radius: BACKGROUND_CORNER_RADIUS,
            prompt_text_color: PROMPT_TEXT_COLOR,
            selection_color: SELECTION_COLOR,
            text_color: TEXT_COLOR,
            text_style: TEXT_STYLE,
        }
    }
}

#[derive(Event)]
pub struct TerminalCommandEvent {
    pub raw_command: String,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct Terminal {
    pub history: Vec<String>,
    pub command_history: Vec<String>,
    pub history_index: usize,
    pub input: String,
    pub current_directory: Vec<String>,
    pub style: TerminalStyle,
    pub root_entity: Entity,
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new(Entity::new())
    }
}

impl Terminal {
    pub fn new(root_entity: Entity) -> Self {
        Self {
            history: Vec::new(),
            command_history: Vec::new(),
            history_index: 0,
            input: String::new(),
            current_directory: Vec::new(),
            style: TerminalStyle::default(),
            root_entity,
        }
    }

    pub fn push_command_result(&mut self, command_result: &CommandResult) {
        match command_result {
            CommandResult::Handled(Some(output)) => {
                self.history.push(output.clone());
            }
            CommandResult::Handled(None) => {}
            CommandResult::Unhandled(command_name, _) => {
                self.history.push(format!("command not found: {}", command_name));
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<TerminalCommandEvent> {
        let mut input_command: Option<TerminalCommandEvent> = None;

        let mut style: egui::Style = (**ui.style()).clone();
        style.visuals.override_text_color = Some(self.style.text_color);
        style.visuals.selection.bg_fill = self.style.selection_color;
        ui.set_style(style);

        let terminal_rect: egui::Rect = ui.available_rect_before_wrap();

        ui.painter().rect_filled(
            terminal_rect,
            self.style.background_corner_radius,
            self.style.background_color,
        );

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui: &mut egui::Ui| {
                for line in &self.history {
                    if line.contains("\x1b[") {
                        let mut layout_job: egui::text::LayoutJob = egui::text::LayoutJob::default();
                        let mut current_text_format: egui::text::TextFormat = egui::text::TextFormat {
                            font_id: TEXT_STYLE.resolve(ui.style()),
                            color: self.style.text_color,
                            background: egui::Color32::TRANSPARENT,
                            italics: false,
                            underline: egui::Stroke::NONE,
                            strikethrough: egui::Stroke::NONE,
                            ..Default::default()
                        };
                        
                        let mut remaining: &str = line.as_str();

                        while let Some(escape_sequence_start_index) = remaining.find("\x1b[") {
                            if escape_sequence_start_index > 0 {
                                layout_job.append(
                                    &remaining[..escape_sequence_start_index],
                                    0.0,
                                    current_text_format.clone(),
                                );
                            }

                            let string_after_escape: &str = &remaining[escape_sequence_start_index + 2..];
                            let Some(letter_m_index): Option<usize> = string_after_escape.find('m') else { break; };
                            
                            let codes: &str = &string_after_escape[..letter_m_index];
                            for code_part in codes.split(';') {
                                crate::style_sheet::apply_ansi_code(
                                    code_part,
                                    &mut current_text_format,
                                    self.style.text_color,
                                    egui::Color32::TRANSPARENT,
                                );
                            }

                            remaining = &string_after_escape[letter_m_index + 1..];
                        }

                        if !remaining.is_empty() {
                            layout_job.append(remaining, 0.0, current_text_format.clone());
                        }

                        ui.add(egui::Label::new(layout_job));
                    } else {
                        ui.add(egui::Label::new(
                            egui::RichText::new(line).text_style(TEXT_STYLE),
                        ));
                    }
                }

                let mut terminal_clicked: bool = false;
                ui.input(|input: &egui::InputState| {
                    if input.pointer.primary_clicked() && let Some(position) = input.pointer.interact_pos() {
                        terminal_clicked = terminal_rect.contains(position);
                    }
                });

                ui.horizontal(|horizontal_ui: &mut egui::Ui| {
                    let prompt: String = format!("{}$", self.current_directory.join("/"));

                    horizontal_ui.add(egui::Label::new(
                        egui::RichText::new(&prompt)
                            .text_style(TEXT_STYLE)
                            .color(self.style.prompt_text_color),
                    ));

                    let response: egui::Response = horizontal_ui.add(
                        egui::TextEdit::singleline(&mut self.input)
                            .id_source("terminal_input_id")
                            .font(TEXT_STYLE)
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .lock_focus(true),
                    );

                    if terminal_clicked {
                        response.request_focus();
                    }

                    if response.has_focus() {
                        let mut move_cursor_to_end: bool = false;

                        horizontal_ui.input(|input_state: &egui::InputState| {
                            if input_state.key_pressed(egui::Key::ArrowUp) && self.history_index > 0 {
                                self.history_index -= 1;
                                self.input = self.command_history[self.history_index].clone();
                                move_cursor_to_end = true;
                            }
                            if input_state.key_pressed(egui::Key::ArrowDown) && self.history_index < self.command_history.len() {
                                self.history_index += 1;
                                if self.history_index < self.command_history.len() {
                                    self.input = self.command_history[self.history_index].clone();
                                    move_cursor_to_end = true;
                                } else {
                                    self.input.clear();
                                }
                            }
                        });

                        if move_cursor_to_end && let Some(mut text_edit_state) = egui::TextEdit::load_state(horizontal_ui.ctx(), response.id) {
                            let character_cursor: egui::text::CCursor = egui::text::CCursor::new(self.input.chars().count());
                            text_edit_state.cursor.set_char_range(Some(egui::text::CCursorRange::one(character_cursor)));
                            text_edit_state.store(horizontal_ui.ctx(), response.id);
                        }
                    }

                    if response.lost_focus() && horizontal_ui.input(|input_state: &egui::InputState| input_state.key_pressed(egui::Key::Enter)) {
                        let command: String = self.input.clone();
                        self.input.clear();

                        if !command.trim().is_empty() {
                            self.command_history.push(command.clone());
                            input_command = Some(TerminalCommandEvent { raw_command: command });
                        }
                        
                        self.history_index = self.command_history.len();
                        response.request_focus();
                    }
                });
            });

        input_command
    }
}

pub fn execute_command(
    terminal_command_event: On<TerminalCommandEvent>,
    mut terminal: ResMut<Terminal>,
    registry: Res<CommandRegistry>,
    mut commands: Commands,
) {
    let prompt: String = format!("{}$ {}", terminal.current_directory.join("/"), terminal_command_event.raw_command);
    terminal.history.push(prompt);

    let parts: Vec<&str> = terminal_command_event.raw_command.split_whitespace().collect();
    if parts.is_empty() { return; }

    let command_name: &str = parts[0];
    let arguments: Vec<String> = parts[1..].iter().map(|arg: &&str| arg.to_string()).collect();

    if let Some(&system_id) = registry.commands.get(command_name) {
        commands.run_system_with(system_id, arguments);
    } else {
        let unhandled_result: CommandResult = CommandResult::Unhandled(command_name.to_string(), arguments);
        terminal.push_command_result(&unhandled_result);
    }
}

pub trait TerminalWorldExtension {
    fn setup_terminal(&mut self) -> &mut Self;
}

impl TerminalWorldExtension for World {
    fn setup_terminal(&mut self) -> &mut Self {
        self.register_component::<VFSName>();
        self.register_component::<CommandRegistry>();
        self.register_component::<Terminal>();


        let mut command_registry: CommandRegistry = CommandRegistry::default();
        command_registry.register(ClearCommand::name(), self.register_system(ClearCommand::execute));
        command_registry.register(PwdCommand::name(), self.register_system(PwdCommand::execute));
        command_registry.register(LsCommand::name(), self.register_system(LsCommand::execute));
        command_registry.register(CdCommand::name(), self.register_system(CdCommand::execute));
        command_registry.register(CatCommand::name(), self.register_system(CatCommand::execute));
        command_registry.register(RookCommand::name(), self.register_system(RookCommand::execute));
        command_registry.register(HelpCommand::name(), self.register_system(HelpCommand::execute));
        
        self.insert_resource(command_registry);
        self.add_observer(execute_command);

        let entities_directory: Entity = self
            .spawn((
                VFSName {
                    name: ".entities".to_string(),
                },
                VFSQueryChildren {
                    filter: !file_system::DynamicFilter::Never
                },
            ))
            .id();
        
        let vfs_entities_directory: Entity = self
            .spawn((
                VFSName {
                    name: ".vfs_entities".to_string(),
                },
                VFSQueryChildren {
                    filter: self.filter::<VFSName>() | self.filter::<CommandRegistry>() | self.filter::<Terminal>(),
                }
            ))
            .id();
        
        let non_vfs_entities_directory: Entity = self
            .spawn((
                VFSName {
                    name: ".app_entities".to_string(),
                },
                VFSQueryChildren {
                    filter: !(self.filter::<VFSName>() | self.filter::<CommandRegistry>() | self.filter::<Terminal>()),
                }
            ))
            .id();

        let root_entity: Entity = self.spawn((
            VFSName { name: "root".to_string() },
            VFSChildren { children: vec![entities_directory, vfs_entities_directory, non_vfs_entities_directory] },
        )).id();

        let terminal: Terminal = Terminal::new(root_entity);
        self.insert_resource(terminal);

        self
    }
}
