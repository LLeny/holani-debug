use egui::{RichText, TextWrapMode, Vec2};
use holani::consts::TIM0BKUP;

pub struct Timers {
}

impl Timers {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn show(&mut self, timers: &holani::mikey::timers::Timers, ui: &mut egui::Ui) {
        ui.label(RichText::new("Timers").strong());

        egui::Grid::new("timers_grid")
                .striped(true)
                .spacing(Vec2::new(10.0, ui.style().spacing.item_spacing.y))
                .show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                    ui.style_mut().spacing.item_spacing.x = 3.0;

                    ui.strong("Timer");
                    ui.strong("Backup");
                    ui.strong("Static control");
                    ui.strong("Current");
                    ui.strong("Dynamic control");
                    ui.strong("Triggers @");
                    ui.end_row();

                    for i in 0..=7 {
                        ui.label(format!("{}", i));
                        ui.label(format!("{:02X}", timers.peek(TIM0BKUP+(i*4))));
                        ui.label(format!("{:08b}", timers.peek(TIM0BKUP+(i*4)+1)));
                        ui.label(format!("{:02X}", timers.peek(TIM0BKUP+(i*4)+2)));
                        ui.label(format!("{:08b}", timers.peek(TIM0BKUP+(i*4)+3)));
                        ui.label(match timers.timer_trigger(i as usize) {
                            u64::MAX => "∞".to_string(),
                            v => format!("{}", v),
                        });
                        ui.end_row();
                    }
                });
    }
}
