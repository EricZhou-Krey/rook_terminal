use bevy_ecs::{
    entity_disabling::DefaultQueryFilters,
    prelude::*,
    system::SystemIdMarker,
};

use crate::{
    command::{
        CatCommand,
        CdCommand,
        ClearCommand,
        HelpCommand,
        LsCommand,
        PwdCommand,
        RookCommand,
    }, event::{
        TerminalCommandRequested, append_terminal_output, change_terminal_directory, clear_terminal_history, clear_terminal_input, execute_command, request_terminal_focus, update_terminal_style,
    }, file_system::{
        CommandRegistry,
        VFSChildren,
        VFSFilterExtension,
        VFSName,
        VFSQueryChildren,
    }, style::TerminalStyle,
};

pub mod command;
pub mod event;
pub mod file_system;
pub mod style;
pub mod style_sheet;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct TerminalSession {
    pub history: Vec<String>,
    pub command_history: Vec<String>,
    pub history_index: usize,
    pub current_directory: Vec<String>,
    pub root_entity: Entity,
}

impl TerminalSession {
    pub fn new(root_entity: Entity) -> Self {
        Self {
            history: Vec::new(),
            command_history: Vec::new(),
            history_index: 0,
            current_directory: Vec::new(),
            root_entity,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Resource, Default)]
pub struct TerminalInputState {
    pub input: String,
    pub focus_requested: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Resource)]
pub struct TerminalViewState {
    pub style: TerminalStyle,
}


impl TerminalSession {
    pub fn ui(
        &mut self,
        input_state: &mut TerminalInputState,
        view_state: &TerminalViewState,
        ui: &mut egui::Ui,
    ) -> Option<TerminalCommandRequested> {
        let mut pending_command: Option<TerminalCommandRequested> = None;

        let mut egui_style: egui::Style = (**ui.style()).clone();

        egui_style.visuals.override_text_color =
            Some(view_state.style.text_color);

        egui_style.visuals.selection.bg_fill =
            view_state.style.selection_color;

        ui.set_style(egui_style);

        let terminal_rect: egui::Rect = ui.available_rect_before_wrap();

        ui.painter().rect_filled(
            terminal_rect,
            view_state.style.background_corner_radius,
            view_state.style.background_color,
        );

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui: &mut egui::Ui| {
                for history_line in &self.history {
                    if history_line.contains("\x1b[") {
                        let mut layout_job: egui::text::LayoutJob =
                            egui::text::LayoutJob::default();

                        let mut text_format: egui::text::TextFormat = egui::text::TextFormat {
                            font_id: view_state
                                .style
                                .text_style
                                .resolve(ui.style()),
                            color: view_state.style.text_color,
                            background: egui::Color32::TRANSPARENT,
                            italics: false,
                            underline: egui::Stroke::NONE,
                            strikethrough: egui::Stroke::NONE,
                            ..Default::default()
                        };

                        let mut remaining_text: &str = history_line.as_str();

                        while let Some(escape_start) =
                            remaining_text.find("\x1b[")
                        {
                            if escape_start > 0 {
                                layout_job.append(
                                    &remaining_text[..escape_start],
                                    0.0,
                                    text_format.clone(),
                                );
                            }

                            let text_after_escape: &str =
                                &remaining_text[escape_start + 2..];

                            let Some(letter_m_index) =
                                text_after_escape.find('m')
                            else {
                                break;
                            };

                            let ansi_codes: &str =
                                &text_after_escape[..letter_m_index];

                            for ansi_code in ansi_codes.split(';') {
                                crate::style_sheet::apply_ansi_code(
                                    ansi_code,
                                    &mut text_format,
                                    view_state.style.text_color,
                                    egui::Color32::TRANSPARENT,
                                );
                            }

                            remaining_text =
                                &text_after_escape[letter_m_index + 1..];
                        }

                        if !remaining_text.is_empty() {
                            layout_job.append(
                                remaining_text,
                                0.0,
                                text_format.clone(),
                            );
                        }

                        ui.add(egui::Label::new(layout_job));
                    } else {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(history_line)
                                    .text_style(view_state.style.text_style.clone()),
                            ),
                        );
                    }
                }

                let mut terminal_was_clicked: bool = false;

                ui.input(|input_state: &egui::InputState| {
                    if input_state.pointer.primary_clicked()
                        && let Some(pointer_position) =
                            input_state.pointer.interact_pos()
                    {
                        terminal_was_clicked =
                            terminal_rect.contains(pointer_position);
                    }
                });

                ui.horizontal(|horizontal_ui: &mut egui::Ui| {
                    let prompt: String = format!(
                        "{}$",
                        self.current_directory.join("/")
                    );

                    horizontal_ui.add(
                        egui::Label::new(
                            egui::RichText::new(&prompt)
                                .text_style(view_state.style.text_style.clone())
                                .color(view_state.style.prompt_text_color),
                        ),
                    );

                    let input_response: egui::Response = horizontal_ui.add(
                        egui::TextEdit::singleline(&mut input_state.input)
                            .id_source("terminal_input_id")
                            .font(view_state.style.text_style.clone())
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .lock_focus(true),
                    );

                    let should_request_focus: bool = input_state.focus_requested;
                    input_state.focus_requested = false;

                    if terminal_was_clicked || should_request_focus {
                        input_response.request_focus();
                    }

                    if input_response.has_focus() {
                        let mut move_cursor_to_end: bool = false;

                        horizontal_ui.input(|input: &egui::InputState| {
                            if input.key_pressed(egui::Key::ArrowUp)
                                && self.history_index > 0
                            {
                                self.history_index -= 1;

                                if let Some(previous_command) =
                                    self.command_history
                                        .get(self.history_index)
                                {
                                    input_state.input =
                                        previous_command.clone();
                                    move_cursor_to_end = true;
                                }
                            }

                            if input.key_pressed(egui::Key::ArrowDown)
                                && self.history_index
                                    < self.command_history.len()
                            {
                                self.history_index += 1;

                                if let Some(next_command) =
                                    self.command_history
                                        .get(self.history_index)
                                {
                                    input_state.input =
                                        next_command.clone();
                                    move_cursor_to_end = true;
                                } else {
                                    input_state.input.clear();
                                }
                            }
                        });

                        if move_cursor_to_end
                            && let Some(mut text_edit_state) =
                                egui::TextEdit::load_state(
                                    horizontal_ui.ctx(),
                                    input_response.id,
                                )
                        {
                            let character_cursor: egui::text::CCursor =
                                egui::text::CCursor::new(
                                    input_state.input.chars().count(),
                                );

                            text_edit_state.cursor.set_char_range(
                                Some(
                                    egui::text::CCursorRange::one(
                                        character_cursor,
                                    ),
                                ),
                            );

                            text_edit_state.store(
                                horizontal_ui.ctx(),
                                input_response.id,
                            );
                        }
                    }

                    let enter_pressed: bool = horizontal_ui.input(|input: &egui::InputState| {
                        input.key_pressed(egui::Key::Enter)
                    });

                    if input_response.lost_focus() && enter_pressed {
                        let command_text: String = input_state.input.clone();

                        input_state.input.clear();

                        if !command_text.trim().is_empty() {
                            self.command_history.push(command_text.clone());

                            pending_command = Some(
                                TerminalCommandRequested {
                                    raw_command: command_text,
                                },
                            );
                        }

                        self.history_index =
                            self.command_history.len();

                        input_response.request_focus();
                    }
                });
            });

        pending_command
    }
}

pub trait TerminalWorldExtension {
    fn setup_terminal(&mut self) -> &mut Self;
}

impl TerminalWorldExtension for World {
    fn setup_terminal(&mut self) -> &mut Self {
        self.register_component::<VFSName>();
        self.register_component::<CommandRegistry>();

        let mut command_registry: CommandRegistry = CommandRegistry::default();

        command_registry.register(
            ClearCommand::name(),
            self.register_system(ClearCommand::execute),
        );

        command_registry.register(
            PwdCommand::name(),
            self.register_system(PwdCommand::execute),
        );

        command_registry.register(
            LsCommand::name(),
            self.register_system(LsCommand::execute),
        );

        command_registry.register(
            CdCommand::name(),
            self.register_system(CdCommand::execute),
        );

        command_registry.register(
            CatCommand::name(),
            self.register_system(CatCommand::execute),
        );

        command_registry.register(
            RookCommand::name(),
            self.register_system(RookCommand::execute),
        );

        command_registry.register(
            HelpCommand::name(),
            self.register_system(HelpCommand::execute),
        );

        self.insert_resource(command_registry);

        self.add_observer(execute_command);
        self.add_observer(append_terminal_output);
        self.add_observer(clear_terminal_history);
        self.add_observer(change_terminal_directory);
        self.add_observer(update_terminal_style);
        self.add_observer(request_terminal_focus);
        self.add_observer(clear_terminal_input);

        let entities_directory: Entity = self
            .spawn((
                VFSName {
                    name: ".entities".to_string(),
                },
                VFSQueryChildren {
                    filter: !crate::file_system::DynamicFilter::Never,
                },
            ))
            .id();

        let vfs_files_directory: Entity = self
            .spawn((
                VFSName {
                    name: "vfs_files".to_string(),
                },
                VFSQueryChildren {
                    filter: self.filter::<VFSName>(),
                },
            ))
            .id();

        let vfs_internals_directory: Entity = self
            .spawn((
                VFSName {
                    name: "vfs_internal".to_string(),
                },
                VFSQueryChildren {
                    filter: self.filter::<CommandRegistry>()
                        | self.filter::<TerminalSession>()
                        | self.filter::<TerminalInputState>()
                        | self.filter::<TerminalViewState>(),
                },
            ))
            .id();

        let vfs_entities_directory: Entity = self
            .spawn((
                VFSName {
                    name: ".vfs_entities".to_string(),
                },
                VFSChildren {
                    children: vec![
                        vfs_files_directory,
                        vfs_internals_directory,
                    ],
                },
            ))
            .id();

        let ecs_entities_directory: Entity = self
            .spawn((
                VFSName {
                    name: ".ecs_entities".to_string(),
                },
                VFSQueryChildren {
                    filter: self.filter::<DefaultQueryFilters>()
                        | self.filter::<Observer>()
                        | self.filter::<SystemIdMarker>(),
                },
            ))
            .id();

        let application_entities_directory: Entity = self
            .spawn((
                VFSName {
                    name: ".app_entities".to_string(),
                },
                VFSQueryChildren {
                    filter: !(self.filter::<VFSName>()
                        | self.filter::<CommandRegistry>()
                        | self.filter::<TerminalSession>()
                        | self.filter::<TerminalInputState>()
                        | self.filter::<TerminalViewState>()
                        | self.filter::<DefaultQueryFilters>()
                        | self.filter::<Observer>()
                        | self.filter::<SystemIdMarker>()),
                },
            ))
            .id();

        let root_entity: Entity = self
            .spawn((
                VFSName {
                    name: "root".to_string(),
                },
                VFSChildren {
                    children: vec![
                        entities_directory,
                        ecs_entities_directory,
                        vfs_entities_directory,
                        application_entities_directory,
                    ],
                },
            ))
            .id();

        self.insert_resource(TerminalSession::new(root_entity));
        self.insert_resource(TerminalInputState::default());
        self.insert_resource(TerminalViewState::default());

        self
    }
}
