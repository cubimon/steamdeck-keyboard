extern crate hidapi;

use std::str::FromStr;
use std::{fs, sync::Mutex};
use std::sync::mpsc::{self, Sender};

use gtk::{gdk::WindowTypeHint, prelude::GtkWindowExt};
use log::{debug, error, log, warn, Level};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, State};
use enigo::{
    Settings,
    Enigo, Key, Keyboard,
    Direction,
};

mod plugin;

struct AppState {
    enigo: Enigo,
    pause_tx: Sender<bool>,
    config_tx: Sender<String>,
    trigger_haptic_tx: Sender<u8>,
}

static KEY_MAP: &[(&str, Key)] = &[
    ("shift", Key::Shift),
    ("left_shift", Key::LShift),
    ("right_shift", Key::RShift),
    ("control", Key::Control),
    ("lcontrol", Key::LControl),
    ("rcontrol", Key::RControl),
    ("alt", Key::Alt),
    ("meta", Key::Meta),
    ("backspace", Key::Backspace),
    ("return", Key::Return),
    ("delete", Key::Delete),
    ("home", Key::Home),
    ("page_up", Key::PageUp),
    ("page_down", Key::PageDown),
    ("up_arrow", Key::UpArrow),
    ("down_arrow", Key::DownArrow),
    ("left_arrow", Key::LeftArrow),
    ("right_arrow", Key::RightArrow),
    ("escape", Key::Escape),
    ("space", Key::Space),
    ("tab", Key::Tab),
    ("f1", Key::F1),
    ("f2", Key::F2),
    ("f3", Key::F3),
    ("f4", Key::F4),
    ("f5", Key::F5),
    ("f6", Key::F6),
    ("f7", Key::F7),
    ("f8", Key::F8),
    ("f9", Key::F9),
    ("f10", Key::F10),
    ("f11", Key::F11),
    ("f12", Key::F12),
    ("f13", Key::F13),
    ("f14", Key::F14),
    ("f15", Key::F15),
    ("f16", Key::F16),
    ("f17", Key::F17),
    ("f18", Key::F18),
    ("f19", Key::F19),
    ("f20", Key::F20),
    ("f21", Key::F21),
    ("f22", Key::F22),
    ("f23", Key::F23),
    ("f24", Key::F24),
    ("f25", Key::F25),
    ("f26", Key::F26),
    ("f27", Key::F27),
    ("f28", Key::F28),
    ("f29", Key::F29),
    ("f30", Key::F30),
    ("f31", Key::F31),
    ("f32", Key::F32),
    ("f33", Key::F33),
    ("f34", Key::F34),
    ("f35", Key::F35),
];

fn map_key(key: &str) -> Option<Key> {
    if key.len() == 1 {
        return Some(Key::Unicode(key.chars().next().unwrap()));
    }
    let key_lowercase = key.to_lowercase();
    for mapping in KEY_MAP {
        if mapping.0 == key_lowercase {
            return Some(mapping.1);
        }
    }
    error!("unknown key {}", key);
    None
}

fn map_state(state: &str) -> Option<Direction>{
    if state == "down" {
        return Some(Direction::Press);
    }
    if state == "up" {
        return Some(Direction::Release);
    }
    error!("unknown state {}", state);
    None
}

#[tauri::command]
fn read_config(
        app_state: State<'_, Mutex<AppState>>,
        app_handle: tauri::AppHandle) {
    let home_dir = match std::env::var("HOME") {
        Ok(home_dir) => home_dir,
        Err(_) => {
            error!("Failed to read home env variable, using default");
            return;
        }
    };
    let path = home_dir + "/.config/steamdeck-keyboard/config.json";
    let config_str = match fs::read_to_string(path) {
        Ok(config_str) => config_str,
        Err(_) => {
            error!("Failed to read config file, using default");
            return;
        }
    };
    let app_state = app_state.lock().unwrap();
    app_handle.emit("config", config_str.clone())
        .expect("Should be able to set config");
    app_state.config_tx.send(config_str).unwrap();
}

#[tauri::command]
fn send_key(
        app_state: State<'_, Mutex<AppState>>,
        key: &str,
        state: &str) {
    println!("sending key {key}");
    let mapped_key = map_key(key);
    let mapped_direction = map_state(state);
    if mapped_key.is_none() || mapped_direction.is_none() {
        error!("Unknown key {} or state {}", key, state);
        return;
    }
    debug!("key {} state {}", key, state);
    let mut app_state = app_state.lock().unwrap();
    app_state.enigo.key(mapped_key.unwrap(), mapped_direction.unwrap()).expect(
        "Should be able to send key");
}

#[tauri::command]
fn toggle_window(
        app_state: State<'_, Mutex<AppState>>,
        app_handle: tauri::AppHandle) -> bool {
    let win = app_handle.get_webview_window("main").unwrap();
    let gtk_window = win.gtk_window().unwrap();
    gtk_window.set_type_hint(WindowTypeHint::Dock);
    let is_visible = win.is_visible().expect(
        "should be able to check if window is visible");
    debug!("toggling window");
    if !is_visible {
        debug!("maximizing window");
        win.show().expect("Should be able to show window");
    } else {
        debug!("minimizing window");
        win.hide().expect("Should be able to hide window");
    }
    // pause/resume steam client/process
    let mut app_state = app_state.lock().unwrap();
    if !is_visible {
        app_state.pause_tx.send(false).unwrap();
    } else {
        app_state.pause_tx.send(true).unwrap();
    }
    // release all modifier
    app_state.enigo.key(Key::Shift, Direction::Release).expect(
        "Should be able to release shift key");
    app_state.enigo.key(Key::Alt, Direction::Release).expect(
        "Should be able to release alt key");
    app_state.enigo.key(Key::Control, Direction::Release).expect(
        "Should be able to release control key");
    app_state.enigo.key(Key::Meta, Direction::Release).expect(
        "Should be able to release meta key");
    return !is_visible;
}

#[tauri::command]
fn trigger_haptic_pulse(
        app_state: State<'_, Mutex<AppState>>,
        pad: u8) {
    let app_state = app_state.lock().unwrap();
    app_state.trigger_haptic_tx.send(pad).unwrap();
}

#[tauri::command]
fn log(level: &str, message: &str) {
    let level = match Level::from_str(level) {
        Ok(level) => level,
        Err(_) => {
            warn!("Invalid log level from js {}", level);
            Level::Debug
        }
    };
    log!(level, "{}", message);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    tauri::Builder::default()
        .setup(|app| {
            let (pause_tx, pause_rx) = mpsc::channel::<bool>();
            let (config_tx, config_rx) = mpsc::channel::<String>();
            let (trigger_haptic_tx, trigger_haptic_rx) = mpsc::channel::<u8>();
            let plugin = Box::new(plugin::SteamdeckPlugin::new());
            app.manage(Mutex::new(AppState {
                enigo: Enigo::new(&Settings::default()).unwrap(),
                // steam_pid: steam_pid,
                pause_tx: pause_tx,
                config_tx: config_tx,
                trigger_haptic_tx: trigger_haptic_tx,
            }));
            let win = app.get_webview_window("main").unwrap();
            win.set_fullscreen(true).expect(
                "Should be able to set fullscreen");
            win.set_always_on_top(true).expect(
                "Should be able to set always on top");
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(plugin_thread(plugin, app_handle, config_rx, pause_rx, trigger_haptic_rx));
            let quit_menu_item= MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_menu_item])?;
            let tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                      debug!("Quit menu item was clicked");
                      app.exit(0);
                    }
                    _ => {
                      error!("Menu item {:?} not handled", event.id);
                    }
                })
                .build(app);
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            read_config,
            send_key,
            toggle_window,
            trigger_haptic_pulse,
            log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn plugin_thread(
        mut plugin: Box<dyn plugin::Plugin>,
        app_handle: tauri::AppHandle,
        config_rx: mpsc::Receiver<String>,
        pause_rx: mpsc::Receiver<bool>,
        trigger_haptic_rx: mpsc::Receiver<u8>) {
    plugin.thread_fn(app_handle, config_rx, pause_rx, trigger_haptic_rx);
}

