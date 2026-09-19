use bevy_ecs::{entity::Entity, reflect::AppTypeRegistry, world::World};
use rook_terminal::{
    command::{
        CatCommand, CdCommand, ClearCommand, Command, HelpCommand, LsCommand, PwdCommand,
        RookCommand,
    },
    file_system::{Binary, CommandRegistry, Text, VFSChildren, VFSName, VFSQueryChildren},
    Terminal,
};

pub struct TerminalApp {
    pub terminal: Terminal,
    pub world: World,
}

impl TerminalApp {
    pub fn example() -> Self {
        let mut world = World::new();

        let mut registry = CommandRegistry::default();
        registry.register::<ClearCommand>();
        registry.register::<PwdCommand>();
        registry.register::<LsCommand>();
        registry.register::<CdCommand>();
        registry.register::<CatCommand>();
        registry.register::<RookCommand>();
        registry.register::<HelpCommand>();
        world.insert_resource(registry);

        world.insert_resource(AppTypeRegistry::default());

        let entities_directory: Entity = world
            .spawn((
                VFSName {
                    name: ".entities".to_string(),
                },
                VFSQueryChildren {
                    required_components: Vec::new(),
                },
            ))
            .id();

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

        let root_directory_entity: Entity = world
            .spawn((
                VFSName {
                    name: "".to_string(),
                },
                VFSChildren {
                    children: vec![entities_directory, test_directory],
                },
            ))
            .id();

        let mut terminal = Terminal::new(root_directory_entity);

        HelpCommand::execute(&mut terminal, &mut world, &[]);

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
