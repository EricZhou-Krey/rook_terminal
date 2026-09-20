use bevy_ecs::prelude::*;
use egui::{Color32, TextStyle};
use crate::{
    command::{CatCommand, CdCommand, ClearCommand, CommandResult, HelpCommand, LsCommand, PwdCommand, RookCommand}, file_system::{CommandFn, CommandRegistry, VFSChildren, VFSFilterExt, VFSName, VFSQueryChildren}, style_sheet::{
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

#[derive(Debug, Clone, PartialEq)]
pub struct Terminal {
    pub history: Vec<String>,
    pub command_history: Vec<String>,
    pub history_index: usize,
    pub input: String,
    pub current_directory: Vec<String>,
    pub style: TerminalStyle,
    pub root_entity: Entity,
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

    pub fn base_command_registry() -> CommandRegistry {
        let mut registry = CommandRegistry::default();
        registry.register::<ClearCommand>();
        registry.register::<PwdCommand>();
        registry.register::<LsCommand>();
        registry.register::<CdCommand>();
        registry.register::<CatCommand>();
        registry.register::<RookCommand>();
        registry.register::<HelpCommand>();
        registry
    }

    pub fn base_root_entity(world: &mut World) -> Entity {
        let entities_directory: Entity = world
            .spawn((
                VFSName {
                    name: ".entities".to_string(),
                },
                VFSQueryChildren {
                    filter: !file_system::DynamicFilter::Never
                },
            ))
            .id();
        
        let vfs_entities_directory: Entity = world
            .spawn((
                VFSName {
                    name: ".vfs_entities".to_string(),
                },
                VFSQueryChildren {
                    filter: world.filter::<VFSName>() | world.filter::<CommandRegistry>(),
                }
            ))
            .id();
        

        let non_vfs_entities_directory: Entity = world
            .spawn((
                VFSName {
                    name: ".app_entities".to_string(),
                },
                VFSQueryChildren {
                    filter: !(world.filter::<VFSName>() | world.filter::<CommandRegistry>()),
                }
            ))
            .id();

        world.spawn((
            VFSName { name: "root".to_string() },
            VFSChildren { children: vec![entities_directory, vfs_entities_directory, non_vfs_entities_directory] },
        )).id()
    } 

    pub fn execute_command(&mut self, world: &mut World, raw_command: &str) -> CommandResult {
        let prompt: String = format!(
            "{}$ {}",
            self.current_directory.join("/"),
            raw_command
        );
        self.history.push(prompt);

        let parts: Vec<&str> = raw_command.split_whitespace().collect();

        if parts.is_empty() {
            return CommandResult::Handled;
        }

        let command: &str = parts[0];
        let args: &[&str] = &parts[1..];

        let command_function_option: Option<CommandFn> = world
            .get_resource::<CommandRegistry>()
            .and_then(|registry| registry.commands.get(command).copied());

        let Some(command_function): Option<CommandFn> = command_function_option else {
            let unhandled_arguments: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            return CommandResult::Unhandled(command.to_string(), unhandled_arguments);
        };

        command_function(self, world, args)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, world: &mut World) -> Option<CommandResult> {
        let mut final_command_result: Option<CommandResult> = None;

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
                    if input.pointer.primary_clicked() && let Some(pos) = input.pointer.interact_pos() {
                        terminal_clicked = terminal_rect.contains(pos);
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
                        let mut move_cursor_to_end = false;

                        horizontal_ui.input(|input| {
                            if input.key_pressed(egui::Key::ArrowUp) && self.history_index > 0 {
                                self.history_index -= 1;
                                self.input = self.command_history[self.history_index].clone();
                                move_cursor_to_end = true;
                            }
                            if input.key_pressed(egui::Key::ArrowDown) && self.history_index < self.command_history.len() {
                                self.history_index += 1;
                                if self.history_index < self.command_history.len() {
                                    self.input = self.command_history[self.history_index].clone();
                                    move_cursor_to_end = true;
                                } else {
                                    self.input.clear();
                                }
                            }
                        });

                        if move_cursor_to_end && let Some(mut state) = egui::TextEdit::load_state(horizontal_ui.ctx(), response.id) {
                            let ccursor = egui::text::CCursor::new(self.input.chars().count());
                            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                            state.store(horizontal_ui.ctx(), response.id);
                        }
                    }

                    if response.lost_focus() && horizontal_ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        let command: String = self.input.clone();
                        self.input.clear();

                        if !command.trim().is_empty() {
                            self.command_history.push(command.clone());
                            
                            let exec_result = self.execute_command(world, &command);

                            if let CommandResult::Unhandled(..) = exec_result {
                                final_command_result = Some(exec_result);
                            }
                        }
                        
                        self.history_index = self.command_history.len();
                        response.request_focus();
                    }
                });
            });

        final_command_result
    }
}
