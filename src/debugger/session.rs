use std::{f32::consts::FRAC_PI_2, num::NonZeroU32, path::PathBuf, thread::{self, JoinHandle}, time::Duration};
use egui::{vec2, Color32, RichText, TextureOptions, Vec2, Widget};
use egui_memory_editor::MemoryEditor;
use governor::{Quota, RateLimiter};
use holani::{consts::INTSET, mikey::{cpu::M6502Flags, video::RGB_SCREEN_BUFFER_LEN}, suzy::registers::{Joystick, Switches}, Lynx};

use super::{breakpoints::Breakpoints, disassembler::DisasmWidget, settings, timers::Timers, watches::Watches};
use holani::consts::*;

macro_rules! cond_strong_label {
    ($ui:ident, $txt: expr, $cond: expr) => {
        let mut t = RichText::new($txt).monospace();
        if $cond {
            t = t.strong();
        } 
        $ui.label(t);
    };
}

macro_rules! pal_color {
    ($regs: ident, $index: expr) => {{
        let g = $regs.data(GREEN0+$index) * 16;
        let br = $regs.data(BLUERED0+$index);
        let r = (br & 0xf ) * 16;
        let b = (br >> 4 ) * 16;
        Color32::from_rgb(r, g, b)
    }};
}

#[derive(Clone, Copy, PartialEq)]
pub enum RunnerStatus {
    Paused,
    RunningAsked,
    Running,
    Step,
    Reset,
}

pub struct LynxSession {
    thread_nr: usize,
    controlled_speed: bool,
    lynx: Lynx,
    disassembler: DisasmWidget,
    timers: Timers,
    ram: MemoryEditor,
    status: RunnerStatus,
    breakpoints: Vec<(bool, u16)>,
    breakpoints_edit: Breakpoints,
    watches: Vec<u16>,
    watches_edit: Watches,
    joystick: Joystick,
    switches: Switches,
    rotation: u8,
    cartridge: Option<PathBuf>,
    screen_buffer: Vec<u8>,
}

impl LynxSession {
    fn new(thread_nr: usize) -> Self {
        let mut slf = Self {
            thread_nr,
            controlled_speed: false,
            lynx: Lynx::new(),
            disassembler: DisasmWidget::new(),
            timers: Timers::new(),
            ram: MemoryEditor::new()                
                .with_address_range("All", 0..0xFFFF+1)
                .with_window_title("RAM"),
            status: RunnerStatus::Paused,
            breakpoints: vec![],
            breakpoints_edit: Breakpoints::new(),
            watches: vec![],
            watches_edit: Watches::new(),
            joystick: Joystick::empty(),
            switches: Switches::empty(),
            rotation: 0,
            cartridge: None,
            screen_buffer: vec![0; RGB_SCREEN_BUFFER_LEN],
        };

        let mut opts = slf.ram.options.clone();
        opts.address_text_colour = Color32::GRAY;
        opts.is_options_collapsed = true;
        opts.show_ascii = false;
        slf.ram.set_options(opts);

        slf
    }

    fn show(&mut self, ctx: &egui::Context) {

        let title = match &self.cartridge {
            None => format!("Lynx {}", self.thread_nr),
            Some(cart) => cart.file_name().unwrap().to_str().unwrap().to_string()
        };

        egui::Window::new(title)
            .default_size(vec2(900., 600.))
            .resizable(true)
            .vscroll(false)
            .show(ctx, |ui| {
                egui::TopBottomPanel::top("top_panel")
                    .resizable(false)
                    .default_height(30.0)
                    .show_inside(ui, |ui| self.top_panel(ui));

                egui::SidePanel::left("central_left_panel")
                    .resizable(true)
                    .default_width(250.0)
                    .show_inside(ui, |ui| self.left_panel(ui));
            
                egui::SidePanel::right("central_right_panel")
                    .resizable(true)
                    .default_width(250.0)
                    .show_inside(ui, |ui| self.right_panel(ui));

                egui::CentralPanel::default()
                    .show_inside(ui, |ui| self.central_panel(ui));
            });   
    }

    fn right_panel(&mut self, ui: &mut egui::Ui) {
        self.palette_show(ui);
        ui.separator();
        self.breakpoints_edit.show_ui(ui, &mut self.breakpoints);
        ui.separator();
        self.watches_edit.show_ui(ui, &mut self.watches, self.lynx.ram());
        ui.separator();
        self.interrupts_show(ui);
        ui.separator();
        self.timers.show(self.lynx.mikey().timers(), ui);
    }

    fn top_panel(&mut self, _ui: &mut egui::Ui) {
        
    }

    fn left_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.cpu_show(ui);
            ui.separator();
            self.controls_show(ui);
        });        
        ui.separator();
        self.buttons_show(ui);
        ui.separator();
        self.disassembler.disasm_show(ui, self.lynx.mikey().cpu().last_ir_pc, &self.lynx);
    }

    fn central_panel(&mut self, ui: &mut egui::Ui) {
        if self.lynx.redraw_requested() {
            self.screen_buffer.copy_from_slice(self.lynx.screen_rgb().as_slice());
        }
        let image = egui::ColorImage::from_rgb([160, 102], &self.screen_buffer);
        let texture = ui.ctx().load_texture("screen", image, TextureOptions::LINEAR);
        let mut img = egui::Image::new(&texture);
        match self.rotation {
            1 => img = img.rotate(FRAC_PI_2, Vec2::splat(0.5)),
            2 => img = img.rotate(FRAC_PI_2*3.0, Vec2::splat(0.5)),
            _ => ()
        }
        img.fit_to_exact_size(ui.available_size()).ui(ui);
        self.ram.draw_editor_contents_read_only(ui, &mut self.lynx, |lx, addr| lx.cpu_mem(addr as u16).into());
    }

    fn interrupts_show(&mut self, ui: &mut egui::Ui) {
        ui.strong("Interrupts");
        let ints = self.lynx.mikey().registers().data(INTSET);
        ui.horizontal(|ui| {
            cond_strong_label!(ui, "7", ints & 128 != 0);
            cond_strong_label!(ui, "6", ints & 64 != 0);
            cond_strong_label!(ui, "5", ints & 32 != 0);
            cond_strong_label!(ui, "4", ints & 16 != 0);
            cond_strong_label!(ui, "3", ints & 8 != 0);
            cond_strong_label!(ui, "2", ints & 4 != 0);
            cond_strong_label!(ui, "1", ints & 2 != 0);
            cond_strong_label!(ui, "0", ints & 1 != 0);
        });
    }

    fn cpu_show(&mut self, ui: &mut egui::Ui) {
        let ticks = self.lynx.ticks();
        let cpu = self.lynx.mikey().cpu();
        ui.vertical(|ui| {
            ui.label(RichText::new("CPU").strong());
            ui.monospace(format!("A:${:02X} X:${:02X} Y:${:02X}", cpu.a(), cpu.x(), cpu.y()))
                .on_hover_ui(|ui| {
                    let a = cpu.a();
                    let x = cpu.x();
                    let y = cpu.y();
                    ui.monospace(format!("A:${:02X} b{:08b} {:03}", a, a, a));
                    ui.monospace(format!("X:${:02X} b{:08b} {:03}", x, x, x));
                    ui.monospace(format!("Y:${:02X} b{:08b} {:03}", y, y, y));
                });
            ui.monospace(format!("S:${:02X} PC:${:04X}", cpu.s(), cpu.pc()));
            ui.monospace(format!("ticks: {}", ticks));
            ui.horizontal(|ui| {
                let flags = cpu.flags();
                cond_strong_label!(ui, "N", flags.contains(M6502Flags::N));
                cond_strong_label!(ui, "V", flags.contains(M6502Flags::V));
                cond_strong_label!(ui, "X", flags.contains(M6502Flags::X));
                cond_strong_label!(ui, "B", flags.contains(M6502Flags::B));
                cond_strong_label!(ui, "D", flags.contains(M6502Flags::D));
                cond_strong_label!(ui, "I", flags.contains(M6502Flags::I));
                cond_strong_label!(ui, "Z", flags.contains(M6502Flags::Z));
                cond_strong_label!(ui, "C", flags.contains(M6502Flags::C));
            });
        });        
    }

    fn controls_show(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                match self.status {
                    RunnerStatus::Paused => if ui.button("⏵")
                            .on_hover_text("Run")
                            .clicked() {
                        self.status = RunnerStatus::RunningAsked;
                    }
                    RunnerStatus::Running => if ui.button("⏸")
                            .on_hover_text("Pause")
                            .clicked() {
                        self.status = RunnerStatus::Paused;
                    }
                    _ => { let _ = ui.button("-"); } ,
                }
                if ui.button("⏭")
                        .on_hover_text("Step")
                        .clicked() {
                    self.status = RunnerStatus::Step;
                }
                if ui.button("⟲")
                        .on_hover_text("Reset")
                        .clicked() {
                    self.status = RunnerStatus::Reset;
                }
            });
            ui.horizontal(|ui| {
                if ui.button("📁")
                        .on_hover_text("Save state")
                        .clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("State", &["sal"])
                        .set_title("Lynx state")
                        .save_file() {
                            let size = self.lynx.serialize_size();
                            let mut data: Vec<u8> = vec![0; size];
                            match holani::serialize(&self.lynx, data.as_mut_slice()){
                                Err(_) => panic!(),
                                Ok(_)  => if std::fs::write(path, data).is_err() { panic!() }
                            };
                    }
                }
                if ui.button("📂")
                        .on_hover_text("Load state")
                        .clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("State", &["sal"])
                        .set_title("Lynx state")
                        .pick_file() {
                            match std::fs::read(path) {
                                Err(_) => panic!(),
                                Ok(data) => match holani::deserialize(&data, &self.lynx) {
                                    Err(_) => panic!(),
                                    Ok(lynx) => self.lynx = lynx,
                                }
                            };
                    }
                }
            });
            ui.checkbox(&mut self.controlled_speed, "Control speed");
        });
    }

    fn buttons_show(&mut self, ui: &mut egui::Ui) {
        ui.strong("Buttons");
        ui.horizontal(|ui| {
            let joy = self.lynx.joystick();
            cond_strong_label!(ui, "⬆", joy.contains(Joystick::up));
            cond_strong_label!(ui, "⬇", joy.contains(Joystick::down));
            cond_strong_label!(ui, "⬅", joy.contains(Joystick::left));
            cond_strong_label!(ui, "➡", joy.contains(Joystick::right));
            cond_strong_label!(ui, "A", joy.contains(Joystick::inside));
            cond_strong_label!(ui, "B", joy.contains(Joystick::outside));
            cond_strong_label!(ui, "1", joy.contains(Joystick::option_1));
            cond_strong_label!(ui, "2", joy.contains(Joystick::option_1));
        });
    }

    fn palette_show(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Palette").strong());
        let regs = self.lynx.mikey().registers();
        ui.horizontal(|ui| {
            for i in 0..16 {
                ui.colored_label(pal_color!(regs, i),"⏹");
            }
        });        
    }
}

pub fn new_lynx_session(thread_nr: usize, on_done_tx: kanal::Sender<()>, cart: PathBuf, settings: settings::Settings) -> (JoinHandle<()>, kanal::Sender<egui::Context>) {
    let (show_tx, show_rc) = kanal::unbounded::<egui::Context>();
    let handle = std::thread::Builder::new()
        .name(format!("Lynx {thread_nr}"))
        .spawn(move || {
            const CRYSTAL_FREQUENCY: u32 = 16_000_000;
            let lim = RateLimiter::direct(Quota::per_second(NonZeroU32::new(CRYSTAL_FREQUENCY).unwrap()));
            let mut speed_ok = false;
            let mut state = LynxSession::new(thread_nr);

            if let Some(path) = settings.boot_rom_path() {
                state.lynx.load_rom_from_slice(&std::fs::read(path).unwrap()).unwrap()
            };

            state.lynx.load_cart_from_slice(&std::fs::read(cart.to_str().unwrap()).unwrap()).unwrap();
            state.lynx.reset();
            state.rotation = state.lynx.rotation();
            state.cartridge = Some(cart);

            loop {
                
                if state.controlled_speed { 
                    match lim.check() {
                        Err(_) => {
                            thread::sleep(Duration::from_nanos(10));
                            speed_ok = false;
                        },
                        Ok(_) => speed_ok = true,
                    }
                }

                if !state.controlled_speed || speed_ok {
                    match state.status {
                        RunnerStatus::RunningAsked => {
                            state.lynx.step_instruction();
                            state.status = RunnerStatus::Running;
                        }
                        RunnerStatus::Running => {
                            let instr_pc = state.lynx.mikey().cpu().last_ir_pc;
                            if state.breakpoints.iter().any(|(en, addr)| { *en && *addr == instr_pc }) {
                                state.status = RunnerStatus::Paused;
                            } else {
                                let ticks = state.lynx.step_instruction();
                                let _ = lim.check_n(NonZeroU32::new(ticks as u32).unwrap());
                            }
                        }
                        RunnerStatus::Step => {
                            state.lynx.step_instruction();
                            state.status = RunnerStatus::Paused;
                        }
                        RunnerStatus::Reset => {
                            state.lynx.reset();
                            state.status = RunnerStatus::Paused;
                        }
                        RunnerStatus::Paused => ()
                    };
                }

                if show_rc.is_disconnected() {
                    break;
                }

                if let Ok(Some(ctx)) = show_rc.try_recv() {
                    state.show(&ctx);
                    ctx.request_repaint_after(Duration::from_secs(1/75));

                    let j = state.joystick;
                    let s = state.switches;                        
                    
                    ctx.input(|ui| {
                        state.joystick.set(Joystick::up, ui.key_down(egui::Key::ArrowUp));
                        state.joystick.set(Joystick::down, ui.key_down(egui::Key::ArrowDown));
                        state.joystick.set(Joystick::left, ui.key_down(egui::Key::ArrowLeft));
                        state.joystick.set(Joystick::right, ui.key_down(egui::Key::ArrowRight));
                        state.joystick.set(Joystick::option_1, ui.key_down(egui::Key::Num1));
                        state.joystick.set(Joystick::option_2, ui.key_down(egui::Key::Num2));
                        state.joystick.set(Joystick::inside, ui.key_down(egui::Key::Q));
                        state.joystick.set(Joystick::outside, ui.key_down(egui::Key::W));
                        state.switches.set(Switches::pause, ui.key_down(egui::Key::P));
                    });

                    if state.joystick != j {
                        state.lynx.set_joystick_u8(state.joystick.bits());
                    }
                    if state.switches != s {
                        state.lynx.set_switches_u8(state.switches.bits());
                    }

                    on_done_tx.send(()).unwrap();
                }
            }
        })
        .expect("failed to spawn thread");
    (handle, show_tx)
}

