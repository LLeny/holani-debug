use std::sync::Arc;

use egui::{mutex::RwLock, Color32, RichText, ScrollArea, Widget};

use super::hex_input;

pub struct Breakpoints {
    input: u16,
    buffer: Arc<RwLock<String>>
}

impl Breakpoints {
    pub fn new() -> Self {
        Self {
            input: 0,
            buffer: Default::default(),
        }
    }

    pub fn show_ui(&mut self, ui: &mut egui::Ui, bps: &mut Vec<(bool, u16)>) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Breakpoints").strong());
            hex_input::HexInput::new(&mut self.input, self.buffer.clone(), 4).ui(ui);
            if ui.button("Add").clicked() {
                bps.push((true, self.input));
            }
        });

        let scroll = ScrollArea::vertical()
            .id_source("breakpoints_scroll")
            .max_height(f32::INFINITY)
            .auto_shrink([false, true]);

        let row_height = ui.text_style_height(&egui::TextStyle::Body);

        let mut to_delete: Option<usize> = None;
        let mut switch_status: Option<usize> = None;

        scroll.show_rows(ui, row_height, bps.len(), |ui, line_range| {
            egui::Grid::new("breakpoint_grid")
                .striped(true)
                .spacing(egui::Vec2::new(8.0, ui.style().spacing.item_spacing.y))
                .show(ui, |ui| {
                    let mut current_line = line_range.start;

                    while current_line != line_range.end {
                        let (en, addr) = bps[current_line];

                        let icon = if en {RichText::new("⏺").color(Color32::RED)} else {RichText::new("○")};
                        
                        if ui.add(egui::Button::new(icon).frame(false)).clicked() {
                            switch_status = Some(current_line);
                        }
                        
                        ui.monospace(format!("${:04X}", addr));
                        
                        if ui.add(egui::Button::new("❌").frame(false)).clicked() {
                            to_delete = Some(current_line);
                        }  
                        
                        ui.end_row();
                        current_line += 1;
                    }
                });
        });

        if let Some(d) = to_delete {
            bps.remove(d);
        }

        if switch_status.is_some() {
            let i = switch_status.unwrap();
            bps[i].0 = !bps[i].0; 
        }
    }
}