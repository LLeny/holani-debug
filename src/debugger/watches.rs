use std::sync::Arc;
use egui::{mutex::RwLock, RichText, ScrollArea, Widget};
use holani::ram::Ram;

use super::hex_input;

pub struct Watches {
    input: u16,
    buffer: Arc<RwLock<String>>
}

impl Watches {
    pub fn new() -> Self {
        Self {
            input: 0,
            buffer: Default::default(),
        }
    }

    pub fn show_ui(&mut self, ui: &mut egui::Ui, whs: &mut Vec<u16>, ram: &Ram) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Watch").strong());
            hex_input::HexInput::new(&mut self.input, self.buffer.clone(), 4).ui(ui);
            if ui.button("Add").clicked() {
                whs.push(self.input);
            }
        });

        let scroll = ScrollArea::vertical()
            .id_source("watches_scroll")
            .max_height(f32::INFINITY)
            .auto_shrink([false, true]);

        let row_height = ui.text_style_height(&egui::TextStyle::Body);

        let mut to_delete: Option<usize> = None;

        scroll.show_rows(ui, row_height, whs.len(), |ui, line_range| {
            egui::Grid::new("watch_grid")
                .striped(true)
                .spacing(egui::Vec2::new(8.0, ui.style().spacing.item_spacing.y))
                .show(ui, |ui| {
                    let mut current_line = line_range.start;

                    while current_line != line_range.end {
                        let addr = whs[current_line];
                        let v = ram.get(addr);
                        ui.monospace(format!("${:04X}: ${:02X} b{:08b} {}", addr, v, v, v));
                        
                        if ui.add(egui::Button::new("❌").frame(false)).clicked() {
                            to_delete = Some(current_line);
                        }  
                        
                        ui.end_row();
                        current_line += 1;
                    }
                });
        });

        if let Some(d) = to_delete {
            whs.remove(d);
        }
    }
}