use eframe::egui::mutex::RwLock;
use eframe::egui::{Response, Ui};
use std::sync::{Arc, LazyLock};

/// Hexadecimal input.
pub struct HexInput<'a> {
    tgt: &'a mut u16,
    buffer: Arc<RwLock<String>>,
    key: usize,
}
impl<'a> HexInput<'a> {
    pub fn new(target: &'a mut u16, buffer: Arc<RwLock<String>>, key: usize) -> Self {
        Self {
            tgt: target,
            buffer,
            key,
        }
    }
}

static KEY: LazyLock<RwLock<usize>> = LazyLock::new(|| RwLock::new(0));

impl<'a> eframe::egui::Widget for HexInput<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut text_buffer = if self.key == *KEY.read() {
            self.buffer.read().clone()
        } else {
            format!("{:04X}", self.tgt)
        };

        let text_edit = eframe::egui::TextEdit::singleline(&mut text_buffer).desired_width(32.0);

        let response = ui.add(text_edit);

        if response.changed() {
            if let Ok(v) = u16::from_str_radix(&text_buffer, 16) {
                *self.tgt = v;
                *self.buffer.write()=text_buffer;
                // lock buffer
                *KEY.write()=self.key;
            }
        }
        response
    }
}
