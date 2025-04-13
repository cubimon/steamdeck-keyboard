mod steamdeck;

use std::sync::mpsc;

pub use steamdeck::SteamdeckPlugin;

pub trait Plugin : Send {
    fn thread_fn(
        &mut self,
        app_handle: tauri::AppHandle,
        config_rx: mpsc::Receiver<String>,
        pause_rx: mpsc::Receiver<bool>,
        trigger_haptic_rx: mpsc::Receiver<u8>);
}
