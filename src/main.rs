use bevy_ecs::{
    entity::Entity,
    world::{Mut, World},
};
use rook_terminal::{
    command::HelpCommand,
    event::TerminalCommandRequested,
    file_system::{Binary, Text, VFSChildren, VFSName},
    TerminalInputState, TerminalSession, TerminalViewState, TerminalWorldExtension,
};

pub struct TerminalApp {
    pub world: World,
}

impl TerminalApp {
    pub fn example() -> Self {
        let mut world: World = World::new();

        world.setup_terminal();

        let test_text: Entity = world
            .spawn((
                VFSName {
                    name: "test_readme.txt".to_string(),
                },
                Text {
                    text: "this is some testing informations".to_string(),
                },
            ))
            .id();

        let test_binary: Entity = world
            .spawn((
                VFSName {
                    name: "test_binary.bin".to_string(),
                },
                Binary {
                    binary: vec![123, 124, 21, 21, 42, 135, 25, 45, 24, 124],
                },
            ))
            .id();

        let test_directory: Entity = world
            .spawn((
                VFSName {
                    name: "test".to_string(),
                },
                VFSChildren {
                    children: vec![test_text, test_binary],
                },
            ))
            .id();

        let root_entity: Entity = world.resource::<TerminalSession>().root_entity;

        if let Some(mut vfs_children) = world.get_mut::<VFSChildren>(root_entity) {
            vfs_children.children.push(test_directory);
        }

        world.trigger(TerminalCommandRequested {
            raw_command: HelpCommand::name().to_string(),
        });
        world.flush();

        Self { world }
    }
}

impl eframe::App for TerminalApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui: &mut egui::Ui| {
            let pending_event: Option<TerminalCommandRequested> = self.world.resource_scope(
                |world: &mut World, mut terminal: Mut<TerminalSession>| {
                    world.resource_scope(
                        |world: &mut World, mut input_state: Mut<TerminalInputState>| {
                            let view_state: &TerminalViewState =
                                world.resource::<TerminalViewState>();

                            terminal.ui(&mut input_state, view_state, ui)
                        },
                    )
                },
            );

            if let Some(event) = pending_event {
                self.world.trigger(event);
                self.world.flush();
            }
        });
    }
}

fn main() -> eframe::Result {
    let native_options: eframe::NativeOptions = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_maximized(true),
        ..Default::default()
    };

    eframe::run_native(
        "egui-experiments",
        native_options,
        Box::new(|_creation_context: &eframe::CreationContext| {
            Ok(Box::new(TerminalApp::example()))
        }),
    )
}
