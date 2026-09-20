use bevy_ecs::{entity::Entity, world::World};
use rook_terminal::{
    command::{Command, CommandResult, HelpCommand},
    file_system::{Binary, Text, VFSChildren, VFSName},
    Terminal,
};

pub struct TerminalApp {
    pub terminal: Terminal,
    pub world: World,
}

impl TerminalApp {
    pub fn example() -> Self {
        let mut world = World::new();
        world.insert_resource(Terminal::base_command_registry());

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

        let root: Entity = Terminal::base_root_entity(&mut world);
        let mut root_children: bevy_ecs::world::Mut<'_, VFSChildren> =
            world.get_mut::<VFSChildren>(root).unwrap();
        root_children.children.push(test_directory);

        let mut terminal = Terminal::new(root);
        let command_result: CommandResult = HelpCommand::execute(&mut terminal, &mut world, &[]);
        terminal.push_command_result(&command_result);

        Self { terminal, world }
    }
}

impl eframe::App for TerminalApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            self.terminal.ui(ui, &mut self.world);
        });
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
