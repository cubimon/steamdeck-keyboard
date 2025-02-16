extern crate hidapi;

use std::collections::VecDeque;
use std::path::Path;
use std::str::FromStr;
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::{fs, sync::Mutex};
use std::sync::mpsc::{self, Sender};

use gtk::{gdk::WindowTypeHint, prelude::GtkWindowExt};
use log::{debug, error, info, log, trace, warn, Level};

use hidapi::{DeviceInfo, HidDevice};
use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder};
use tauri::{Emitter, Manager, State};
use enigo::{
    Settings,
    Enigo, Key, Keyboard,
    Direction,
};

#[derive(Debug, Serialize, Deserialize)]
#[derive(Default)]
#[serde()]
struct Config {
    steam_pid: Option<i32>,
}

impl Config {
    fn new() -> Self {
        Default::default()
    }
}


struct AppState {
    enigo: Enigo,
    steam_pid: Option<i32>,
    pause_tx: Sender<bool>,
    trigger_haptic_tx: Sender<u8>,
    config: Config,
}

struct TouchEntry {
    x: i16,
    y: i16,
    force: u16,
    time: Instant,
}

impl TouchEntry {
    fn is_touched(&self) -> bool {
        return self.x != 0 || self.y != 0;
    }
}

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
    let mut app_state = app_state.lock().unwrap();
    let new_config: Config = serde_json::from_str(config_str.as_str())
        .expect("Config string could not be parsed");
    if app_state.config.steam_pid.is_none() {
        match new_config.steam_pid {
            Some(steam_pid) => {
                debug!("Using steam pid {} from config", steam_pid);
                app_state.steam_pid = Some(steam_pid);
                send_steam_signal(steam_pid, true);
            },
            None => {},
        };
    }
    app_state.config = new_config;
    debug!("new app_state config {}", serde_json::to_string(&app_state.config).unwrap());
    app_handle.emit("config", config_str)
        .expect("Should be able to set config");
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
    // check if steam pid is alive on toggle window
    if app_state.steam_pid.is_some() && !is_pid_alive(app_state.steam_pid.unwrap()) {
        match get_steam_pid() {
            Some(steam_pid) => {
                app_state.steam_pid = Some(steam_pid)
            },
            None => {}
        }
    }
    match app_state.steam_pid {
        Some(steam_pid) => {
            send_steam_signal(steam_pid, is_visible);
        },
        None => {},
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
            let steam_pid = get_steam_pid();
            let (pause_tx, pause_rx) = mpsc::channel::<bool>();
            let (trigger_haptic_tx, trigger_haptic_rx) = mpsc::channel::<u8>();
            app.manage(Mutex::new(AppState {
                enigo: Enigo::new(&Settings::default()).unwrap(),
                steam_pid: steam_pid,
                pause_tx: pause_tx,
                trigger_haptic_tx: trigger_haptic_tx,
                config: Config::new(),
            }));
            match steam_pid {
                Some(steam_pid) => send_steam_signal(steam_pid, false),
                None => {},
            };
            let win = app.get_webview_window("main").unwrap();
            win.set_fullscreen(true).expect(
                "Should be able to set fullscreen");
            win.set_always_on_top(true).expect(
                "Should be able to set always on top");
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async_read_usb_hid_touchpad(
                app_handle, pause_rx, trigger_haptic_rx));
            let quit_menu_item= MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_menu_item])?;
            let tray = TrayIconBuilder::new()
                .menu(&menu)
                .menu_on_left_click(true)
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

// read hid device and send messages to js frontend
async fn async_read_usb_hid_touchpad(
        app_handle: tauri::AppHandle,
        pause_rx: mpsc::Receiver<bool>,
        trigger_haptic_rx: mpsc::Receiver<u8>) -> ! {
    let mut left_touch_history: VecDeque<TouchEntry> = VecDeque::new();
    let mut right_touch_history: VecDeque<TouchEntry> = VecDeque::new();
    let mut last_toggle_window: Instant = Instant::now();
    let mut device = hid_device_factory().unwrap();
    let mut pause = false;
    let mut is_visible = true;
    let mut last_read = Instant::now();
    loop {
        // pause flag
        match pause_rx.try_recv() {
            Ok(message_pause) => {
                debug!("[HID thread] new pause flag: {}", message_pause);
                pause = message_pause;
            },
            Err(_) => {},
        };
        // haptic
        match trigger_haptic_rx.try_recv() {
            Ok(pad) => {
                let mut haptic_report = [0u8; 12];
                haptic_report[0] = 0;
                haptic_report[1] = 0x8F; // ID_TRIGGER_HAPTIC_PULSE
                haptic_report[2] = 8; // next bytes count/size
                haptic_report[3] = pad; // 0 = right, 1 = left, 2 = both
                haptic_report[4] = 0xff; // duration lower byte
                haptic_report[5] = 0xff; // duration upper byte
                haptic_report[6] = 0x00; // interval lower byte
                haptic_report[7] = 0x00; // interval upper byte
                haptic_report[8] = 0x01; // count lower byte
                haptic_report[9] = 0x00; // count upper byte
                haptic_report[10] = 0xff; // gain
                match device.send_feature_report(&mut haptic_report) {
                    Ok(_) => {},
                    Err(_) => {
                        error!("Failed to send hid feature report for haptic");
                    },
                }
                trace!("Doing haptics now");
            },
            Err(_) => {},
        }
        // input
        let mut buf = [0u8; 64];
        let res = match device.read(&mut buf[..]) {
            Ok(res) => res,
            Err(_) => {
                warn!("Failed to read device, reopening after short delay");
                sleep(Duration::from_millis(1000));
                device = match hid_device_factory() {
                    Some(device) => device,
                    None => {
                        continue;
                    }
                };
                continue;
            }
        };
        if res != 64 {
            error!("USB hid response size wasn't 64 but {}", res);
            continue;
        }
        let now = Instant::now();
        if pause && (now - last_read).as_millis() < 50 {
            continue;
        }
        last_read = now;
        let device_report = SteamDeckDeviceReport {
            l_pad_x: i16::from_le_bytes(buf[16..18].try_into().unwrap()),
            l_pad_y: i16::from_le_bytes(buf[18..20].try_into().unwrap()),
            l_pad_force: u16::from_le_bytes(buf[56..58].try_into().unwrap()),
            r_pad_x: i16::from_le_bytes(buf[20..22].try_into().unwrap()),
            r_pad_y: i16::from_le_bytes(buf[22..24].try_into().unwrap()),
            r_pad_force: u16::from_le_bytes(buf[58..60].try_into().unwrap()),
            l4: (u32::from_le_bytes(buf[12..16].try_into().unwrap()) & 0x200) > 0,
        };
        left_touch_history.push_back(TouchEntry {
            x: device_report.l_pad_x,
            y: device_report.l_pad_y,
            force: device_report.l_pad_force,
            time: now,
        });
        right_touch_history.push_back(TouchEntry {
            x: device_report.r_pad_x,
            y: device_report.r_pad_y,
            force: device_report.r_pad_force,
            time: now,
        });
        while !left_touch_history.is_empty() {
            match left_touch_history.front() {
                Some(front) => {
                    if now.duration_since(front.time).as_millis() > 2000 {
                        left_touch_history.pop_front();
                        continue;
                    }
                },
                None => (),
            }
            break;
        }
        while !right_touch_history.is_empty() {
            match right_touch_history.front() {
                Some(front) => {
                    if now.duration_since(front.time).as_millis() > 2000 {
                        right_touch_history.pop_front();
                        continue;
                    }
                },
                None => (),
            }
            break;
        }
        if check_keyboard_toggle(
                &left_touch_history,
                &right_touch_history,
                last_toggle_window,
                is_visible) {
            debug!("[HID thread] toggle window");
            last_toggle_window = Instant::now();
            let state = app_handle.state::<Mutex<AppState>>();
            is_visible = toggle_window(state, app_handle.clone());
        }
        trace!("[HID thread] \
            left touch history size {}, \
            right touch history size {}",
            left_touch_history.len(), right_touch_history.len());
        if !pause {
            app_handle.emit("input", device_report).expect(
                "Should be able to emit device report");
        }
    }
}

fn hid_device_factory() -> Option<HidDevice> {
    let api = hidapi::HidApi::new().unwrap();
    for device in api.device_list() {
        debug!("[HID thread] device: {:#?}, path:{:?}, vendor id: {}, product id: {}",
            device,
            device.path(),
            device.vendor_id(),
            device.product_id());
    }
    let (vid, pid) = (0x28de, 0x1205);
    let device_info_opt = api.device_list()
        .filter(|device_info: &&DeviceInfo| device_info.vendor_id() == vid)
        .filter(|device_info| device_info.product_id() == pid)
        .filter(|device_info| device_info.interface_number() == 2)
        .next();
    if device_info_opt.is_none() {
        return None;
    }
    let device_info = device_info_opt.unwrap();
    debug!("[HID thread] device path: {:?}", device_info.path());
    return match device_info.open_device(&api) {
        Ok(device) => {
            disable_steam_watchdog(&device);
            Some(device)
        },
        Err(_) => None,
    };
}

fn check_keyboard_toggle(
        left_touch_history: &VecDeque<TouchEntry>,
        right_touch_history: &VecDeque<TouchEntry>,
        last_toggle_window: Instant,
        is_visible: bool) -> bool {
    if left_touch_history.len() == 0 || right_touch_history.len() == 0 {
        trace!("left or right touch history is empty");
        return false;
    }
    let last_left_touch = left_touch_history.back().unwrap();
    let last_right_touch = right_touch_history.back().unwrap();
    let is_left_touched = last_left_touch.is_touched();
    let is_right_touched = last_right_touch.is_touched();
    if is_visible && (!is_left_touched && !is_right_touched) {
        debug!("Visible and both touchpads released, closing keyboard");
        return true;
    }
    let left_touch_time = get_last_touch_start_time(left_touch_history);
    let right_touch_time = get_last_touch_start_time(right_touch_history);
    if left_touch_time.is_none() || right_touch_time.is_none() {
        trace!("left or right isn't touched");
        return false;
    }
    let left_touch_time = left_touch_time.unwrap();
    let right_touch_time = right_touch_time.unwrap();
    if left_touch_time < last_toggle_window || right_touch_time < last_toggle_window {
        trace!("touch time before last toggle");
        return false;
    }
    let time_diff = if left_touch_time > right_touch_time {
        left_touch_time - right_touch_time
    } else {
        right_touch_time - left_touch_time
    };
    let time_diff = time_diff.as_millis();
    trace!("time difference {}", time_diff);
    if time_diff < 100 {
        trace!("time difference below threshold");
        return true;
    }
    return false;
}

fn get_last_touch_start_time(touch_history: &VecDeque<TouchEntry>) -> Option<Instant> {
    if touch_history.is_empty() {
        return None;
    }
    let mut curr = touch_history.back().unwrap();
    for prev in touch_history.iter().rev() {
        if curr.is_touched() && !prev.is_touched() {
            return Some(curr.time);
        }
        curr = prev;
    }
    return None;
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

fn is_pid_alive(pid: i32) -> bool {
    let pid_path = format!("/proc/{}", pid);
    let result = Path::new(pid_path.as_str()).exists();
    debug!("Checking if pid is alive, {}: {}", pid_path, result);
    return result;
}

/// Pauses or resumes steam process by pid to disable touchpad handling by steam.
fn send_steam_signal(steam_pid: i32, is_visible: bool) {
    let signal;
    if !is_visible {
        debug!("sending stop signal to steam process");
        signal = libc::SIGSTOP;
    } else {
        debug!("sending continue signal to steam process");
        signal = libc::SIGCONT;
    }
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
