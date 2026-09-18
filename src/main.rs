use bevy_ecs::{reflect::AppTypeRegistry, world::World};
use rook_terminal::{
    command::{
        CatCommand, CdCommand, ClearCommand, Command, CommandRegistry, HelpCommand, LsCommand,
        PwdCommand,
    },
    file_system::{FileName, VFSChildren, VFSDirectory, VFSParent, VFSTextFile},
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
        registry.register::<HelpCommand>();
        world.insert_resource(registry);

        world.insert_resource(AppTypeRegistry::default());

        let root_entity = world
            .spawn((
                FileName("".to_string()),
                VFSDirectory,
                VFSChildren::default(),
            ))
            .id();

        let readme_entity = world
            .spawn((
                FileName("readme.txt".to_string()),
                VFSTextFile("README lol!, bird larping frfr".to_string()),
                VFSParent(root_entity),
            ))
            .id();

        let bird_dir_entity = world
            .spawn((
                FileName("bird".to_string()),
                VFSDirectory,
                VFSChildren(vec![readme_entity]),
                VFSParent(root_entity),
            ))
            .id();

        world
            .entity_mut(root_entity)
            .get_mut::<VFSChildren>()
            .unwrap()
            .0
            .push(bird_dir_entity);

        let mut terminal = Terminal::new(root_entity);

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
