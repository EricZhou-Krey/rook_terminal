use bevy_ecs::{
    entity::Entity,
    schedule::Schedule,
    world::{Mut, World},
};
use rook_terminal::{
    command::{Command, CommandResult, HelpCommand},
    file_system::{Binary, Text, VFSChildren, VFSName},
    Terminal, TerminalScheduleExtension, TerminalWorldExtension,
};

pub struct TerminalApp {
    pub world: World,
    pub schedule: Schedule,
}

impl TerminalApp {
    pub fn example() -> Self {
        let mut world = World::new();
        let mut schedule: Schedule = Schedule::default();

        world.setup_terminal();
        schedule.setup_terminal();

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

        let root_entity: Entity = world.resource::<Terminal>().root_entity;

        if let Some(mut vfs_children) = world.get_mut::<VFSChildren>(root_entity) {
            vfs_children.children.push(test_directory);
        }

        let command_result: CommandResult = HelpCommand::execute(&mut world, &[]);
        world
            .resource_mut::<Terminal>()
            .push_command_result(&command_result);

        Self { world, schedule }
    }
}

impl eframe::App for TerminalApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            self.world
                .resource_scope(|_world: &mut World, mut terminal: Mut<Terminal>| {
                    terminal.ui(ui);
                });
        });
    }

    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.schedule.run(&mut self.world);
    }
}

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_maximized(true),
        ..Default::default()
    };

    eframe::run_native(
        "egui-experiments",
        native_options,
        Box::new(|_cc| Ok(Box::new(TerminalApp::example()))),
    )
}
