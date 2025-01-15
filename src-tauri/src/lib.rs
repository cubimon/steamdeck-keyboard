extern crate hidapi;

use std::{fs, sync::Mutex};

use gtk::{gdk::WindowTypeHint, prelude::GtkWindowExt};
use log::{debug, error, warn, info};

use hidapi::{DeviceInfo, HidDevice};
use serde::Serialize;
use tauri::{Emitter, Manager, State};
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
    is_open: bool,
    steam_pid: Option<i32>,
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
        app_handle: tauri::AppHandle) {
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
    let mut app_state = app_state.lock().unwrap();
    match app_state.steam_pid {
        Some(steam_pid) => {
            send_steam_signal(steam_pid, is_visible);
        },
        None => {},
    }
    app_state.is_open = !is_visible;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let steam_pid = get_steam_pid();
            app.manage(Mutex::new(AppState {
                enigo: Enigo::new(&Settings::default()).unwrap(),
                is_open: true,
                steam_pid: steam_pid,
            }));
            match steam_pid {
                Some(steam_pid) => send_steam_signal(steam_pid, true),
                None => {},
            };
            let win = app.get_webview_window("main").unwrap();
            win.set_fullscreen(true).expect(
                "Should be able to set fullscreen");
            win.set_always_on_top(true).expect(
                "Should be able to set always on top");
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async_read_usb_hid_touchpad(app_handle));
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

// read hid device and send messages to js frontend
async fn async_read_usb_hid_touchpad(app_handle: tauri::AppHandle) -> ! {
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
    disable_steam_watchdog(&device);
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
}

/// Find steam pid in /proc
fn get_steam_pid() -> Option<i32> {
    let paths = fs::read_dir("/proc").unwrap();
    for path in paths {
        let path = match path {
            Ok(path) => path,
            Err(_) => continue
        };
        let pid = match path.file_name().to_str().unwrap().parse::<i32>() {
            Ok(pid) => pid,
            Err(_) => continue,
        };
        let mut proc_exe_path = path.path().clone();
        proc_exe_path.push("exe");
        let exe_path_result = fs::read_link(proc_exe_path);
        let exe_path = match exe_path_result {
            Ok(exe) => exe,
            Err(_) => continue,
        };
        if exe_path.to_str().unwrap() == "/home/deck/.local/share/Steam/ubuntu12_32/steam" {
            info!("steam pid {}", pid);
            return Some(pid);
        }
    }
    warn!("steam pid not found");
    return None;
}

/// Pauses or resumes steam process by pid to disable touchpad handling by steam.
fn send_steam_signal(steam_pid: i32, is_visible: bool) {
    let signal;
    if !is_visible {
        signal = libc::SIGSTOP;
    } else {
        signal = libc::SIGCONT;
    }
    debug!("sending signal to steam process");
    unsafe {
        libc::kill(steam_pid, signal);
    }
}

// Disable steam watchdog, so when pausing steam process
// the steamdeck controller doesn't reset itself to default hid settings
fn disable_steam_watchdog(device: &HidDevice) {
    // see https://github.com/torvalds/linux/blob/master/drivers/hid/hid-steam.c
    debug!("disabling steam watchdog");
    let mut buf = [0u8; 5];
    buf[0] = 0x87; // ID_SET_SETTINGS_VALUES
    buf[1] = 3; // number of settings bytes (without first two)
    buf[2] = 71; // SETTING_STEAM_WATCHDOG_ENABLE
    buf[3] = 0;
    buf[4] = 0;
    match device.send_feature_report(&buf) {
        Ok(_) => {},
        Err(e) => {
            error!("failed to write hid settings {}", e);
        },
    }
}