use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Settings {
    boot_rom_path: Option<PathBuf>,
}

impl Settings {
    pub fn boot_rom_path(&self) -> Option<&PathBuf> {
        self.boot_rom_path.as_ref()
    }
    
    pub fn set_boot_rom_path(&mut self, boot_rom_path: PathBuf) {
        self.boot_rom_path = Some(boot_rom_path);
    }
}