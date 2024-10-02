use std::{path::PathBuf, thread::JoinHandle};
use egui::{menu, Ui};
use log::error;
use session::new_lynx_session;
use macroquad::window::next_frame;
use settings::Settings;
mod session;
mod disassembler;
mod hex_input;
mod breakpoints;
mod watches;
mod settings;
mod timers;

pub struct Debugger {
    threads: Vec<(JoinHandle<()>, kanal::Sender<egui::Context>)>,
    on_done_rx: kanal::Receiver<()>,
    on_done_tx: kanal::Sender<()>,
    settings: Settings,
}

impl Debugger {
    pub fn new() -> Self {
        let (on_done_tx, on_done_rx) = kanal::unbounded();
        let settings = match confy::load::<Settings>("holani") {
            Err(e) => {
                error!("Couldn't load settings. Using defaults. '{}'", e);
                Settings::default()
            },
            Ok(s) => s,
        }; 
        Self {
            threads: vec![],
            settings,
            on_done_rx,
            on_done_tx,
        }
    }

    fn spawn_thread(&mut self, cart: PathBuf, settings: Settings) {
        let thread_nr = self.threads.len();
        self.threads.push(new_lynx_session(thread_nr, self.on_done_tx.clone(), cart, settings));
    }

    fn show(&mut self, ctx: &egui::Context) {        

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                self.file_menu_button(ui);
            });
        });

        for (_handle, show_tx) in &self.threads {
            let _ = show_tx.send(ctx.clone());
        }

        for _ in 0..self.threads.len() {
            let _ = self.on_done_rx.recv();
        }
    }

    fn file_menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button("File", |ui| {
            if ui.button("Open").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Cartridge", holani::valid_extensions())
                    .set_title("Lynx cartridge")
                    .pick_file() {
                    self.spawn_thread(path, self.settings.clone());
                }
            }
        });
        ui.menu_button("Settings", |ui| {
            if ui.button(format!("Select boot ROM ({:?})", self.settings.boot_rom_path())).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("img", &["img"])
                    .set_title("Lynx boot ROM")
                    .pick_file() {
                    self.settings.set_boot_rom_path(path);
                }
            }
        });
    }
}

impl std::ops::Drop for Debugger {
    fn drop(&mut self) {
        match confy::store("holani", &self.settings) {
            Ok(_) => (),
            Err(e) => error!("Couldn't save setings. '{}'", e),
        };
        for (handle, show_tx) in self.threads.drain(..) {
            std::mem::drop(show_tx);
            handle.join().unwrap();
        }
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