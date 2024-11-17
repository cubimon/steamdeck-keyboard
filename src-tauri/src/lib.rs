extern crate hidapi;

use std::sync::Mutex;

use log::{debug, error};

use hidapi::DeviceInfo;
use serde::Serialize;
use tauri::{Emitter, Manager, State};
use gtk::prelude::*;
use gdk::WindowTypeHint;
use enigo::{
    Settings,
    Enigo, Key, Keyboard,
    Direction,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SteamDeckDeviceReport {
    l_pad_x: i16,
    l_pad_y: i16,
    l_pad_force: u16,
    r_pad_x: i16,
    r_pad_y: i16,
    r_pad_force: u16,
    l4: bool,
}

struct AppState {
    enigo: Enigo,
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
    (" ", Key::Space),
];

fn map_key(key: &str) -> Option<Key> {
    if key.len() == 1 {
        return Some(Key::Unicode(key.chars().next().unwrap()));
    }
    for mapping in KEY_MAP {
        if mapping.0 == key {
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
fn send_key(
        app_state: State<'_, Mutex<AppState>>,
        key: &str,
        state: &str) {
    let mut app_state = app_state.lock().unwrap();
    let mapped_key = map_key(key);
    let mapped_direction = map_state(state);
    if mapped_key.is_none() || mapped_direction.is_none() {
        error!("Unknown key {} or state {}", key, state);
        return;
    }
    debug!("key {} state {}", key, state);
    println!("key {} state {}", key, state);
    app_state.enigo.key(mapped_key.unwrap(), mapped_direction.unwrap()).expect(
        "Should be able to send key");
}

#[tauri::command]
fn toggle_window(
        app_handle: tauri::AppHandle) {
    let win = app_handle.get_webview_window("main").unwrap();
    let is_visible = win.is_visible().expect(
        "should be able to check if window is visible");
    println!("toggling window");
    if !is_visible {
        println!("max window");
        debug!("maximizing window");
        win.show().expect("Should be able to show window");
    } else {
        println!("min window");
        debug!("minimizing window");
        win.hide().expect("Should be able to hide window");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // enigo = Enigo::new(&Settings::default()).unwrap();
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Mutex::new(AppState {
                enigo: Enigo::new(&Settings::default()).unwrap()
            }));
            let win = app.get_webview_window("main").unwrap();
            let gtk_window = win.gtk_window().unwrap();
            if let Some(gdk_window) = gtk_window.window() {
                unsafe {
                    let display = gdk_window.display();
                    let screen = gdk_window.screen();
                    let center_x = gtk_window.allocated_width() as f64 / 2.0;
                    let center_y = gtk_window.allocated_height() as f64 / 2.0;
                    let empty_cursor = gdk::Cursor::from_name(&display, "none");
                    gdk_window.set_cursor(empty_cursor.as_ref());
                    let seat = display.default_seat();
                    if let Some(seat) = seat {
                        let pointer = seat.pointer().expect("pointer should exist");
                        pointer.warp(&screen, center_x as i32, center_y as i32);
                    }
                    gdk_sys::gdk_window_set_pass_through(gdk_window.as_ptr(), true as i32);
                }
            }
            gtk_window.set_type_hint(WindowTypeHint::Dock);
            gtk_window.connect_focus_in_event(|_, _| {
                // Immediately defocus when focus is attempted
                gio::glib::Propagation::Stop
            });
            win.set_fullscreen(true).expect(
                "Should be able to set fullscreen");
            win.set_always_on_top(true).expect(
                "Should be able to set always on top");
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let api = hidapi::HidApi::new().unwrap();
                for device in api.device_list() {
                    debug!("device: {:#?}, path:{:?}, vendor id: {}, product id: {}",
                        device,
                        device.path(),
                        device.vendor_id(),
                        device.product_id());
                }
                let (vid, pid) = (0x28de, 0x1205);
                let device_info = api.device_list()
                    .filter(|device_info: &&DeviceInfo| device_info.vendor_id() == vid)
                    .filter(|device_info| device_info.product_id() == pid)
                    .filter(|device_info| device_info.interface_number() == 2)
                    .next().unwrap();
                debug!("device path: {:?}", device_info.path());
                let device = device_info.open_device(&api).unwrap();
                loop {
                    let mut buf = [0u8; 64];
                    let res = device.read(&mut buf[..]).unwrap();
                    if res != 64 {
                        error!("USB hid response size wasn't 64 but {}", res);
                        continue;
                    }
                    let device_report = SteamDeckDeviceReport {
                        l_pad_x: i16::from_le_bytes(buf[16..18].try_into().unwrap()),
                        l_pad_y: i16::from_le_bytes(buf[18..20].try_into().unwrap()),
                        l_pad_force: u16::from_le_bytes(buf[56..58].try_into().unwrap()),
                        r_pad_x: i16::from_le_bytes(buf[20..22].try_into().unwrap()),
                        r_pad_y: i16::from_le_bytes(buf[22..24].try_into().unwrap()),
                        r_pad_force: u16::from_le_bytes(buf[58..60].try_into().unwrap()),
                        l4: (u32::from_le_bytes(buf[12..16].try_into().unwrap()) & 0x200) > 0,
                    };
                    app_handle.emit("input", device_report).expect(
                        "Should be able to emit device report");
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            send_key,
            toggle_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
