use egui::{menu, Ui};
use log::error;
use macroquad::window::next_frame;
use settings::Settings;
use crate::core_runner::{AddCoreConfiguration, CoreRunner};
mod breakpoints;
mod disassembler;
mod hex_input;
pub mod session;
pub mod settings;
mod timers;
mod watches;

pub struct Debugger {
    settings: Settings,
    runner: CoreRunner,
}

impl Debugger {
    pub fn new() -> Self {
        let settings = match confy::load::<Settings>("holani", None) {
            Err(e) => {
                error!("Couldn't load settings. Using defaults. '{}'", e);
                Settings::default()
            }
            Ok(s) => s,
        };

        let mut runner = CoreRunner::new();
        runner.initialize_thread();

        Self {
            settings,
            runner,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                self.file_menu_button(ui);
            });
        });
        self.runner.show(ctx);
    }

    fn file_menu_button(&mut self, ui: &mut Ui) {
        if ui.button("Open").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Cartridge", holani::valid_extensions())
                .set_title("Lynx cartridge")
                .pick_file()
            {
                if let Err(e) = self.runner.add_core(&AddCoreConfiguration { cart_path: path, settings: self.settings.clone()}) {
                    error!("Couldn't load cartridge. '{}'", e);
                }
            }
        };
        if ui.button(format!("Select boot ROM ({:?})",self.settings.boot_rom_path())).clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("img", &["img"])
                .set_title("Lynx boot ROM")
                .pick_file()
            {
                self.settings.set_boot_rom_path(path);
            }
        };
    }
}

impl std::ops::Drop for Debugger {
    fn drop(&mut self) {
        match confy::store("holani", None, &self.settings) {
            Ok(_) => (),
            Err(e) => error!("Couldn't save setings. '{}'", e),
        };
    }
}

pub async fn debugger() {
    let mut app: Debugger = Debugger::new();

    loop {
        egui_macroquad::ui(|ctx| app.show(ctx));
        egui_macroquad::draw();
        next_frame().await;
    }
}
