use hidapi::{DeviceInfo, HidDevice};
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fs,
    path::Path,
    sync::{mpsc, Mutex},
    thread::sleep,
    time::{Duration, Instant, SystemTime},
};
use tauri::Emitter;

use crate::{toggle_window, AppState};

use super::Plugin;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde()]
struct Config {
    steam_pid: Option<i32>,
    deadzone_dist: Option<f32>,
    deadzone_pressure: Option<u16>,
}

impl Config {
    fn new() -> Self {
        Default::default()
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

pub struct SteamdeckPlugin {
    config: Config,
    left_touch_history: VecDeque<TouchEntry>,
    right_touch_history: VecDeque<TouchEntry>,
    deadzone_dist_square: f32,
    deadzone_pressure: u16,
    last_toggle_window: Instant,
    device: HidDevice,
    pause: bool,
    is_visible: bool,
    last_read: Instant,
    last_emitted_report: SteamDeckDeviceReport,
    start: Instant,
}

impl SteamdeckPlugin {
    pub fn new() -> Self {
        let mut config: Config = Config::new();
        config.steam_pid = get_steam_pid();
        match config.steam_pid {
            Some(steam_pid) => send_steam_signal(steam_pid, true),
            None => (),
        }
        return Self {
            config: config,
            left_touch_history: VecDeque::new(),
            right_touch_history: VecDeque::new(),
            deadzone_dist_square: 500.0 * 500.0,
            deadzone_pressure: 500,
            last_toggle_window: Instant::now(),
            device: hid_device_factory().unwrap(),
            pause: false,
            is_visible: true,
            last_read: Instant::now(),
            last_emitted_report: SteamDeckDeviceReport {
                l_pad_x: -100,
                l_pad_y: -100,
                l_pad_force: 0,
                r_pad_x: -100,
                r_pad_y: -100,
                r_pad_force: 0,
                l4: false,
            },
            start: Instant::now(),
        };
    }
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

impl Plugin for SteamdeckPlugin {
    fn thread_fn(
        &mut self,
        app_handle: tauri::AppHandle,
        config_rx: mpsc::Receiver<String>,
        pause_rx: mpsc::Receiver<bool>,
        stop_rx: mpsc::Receiver<()>,
        trigger_haptic_rx: std::sync::mpsc::Receiver<u8>,
    ) {
        thread_fn(
            self,
            app_handle,
            config_rx,
            pause_rx,
            stop_rx,
            trigger_haptic_rx,
        );
    }
}

// read hid device and send messages to js frontend
fn thread_fn(
    plugin: &mut SteamdeckPlugin,
    app_handle: tauri::AppHandle,
    config_rx: mpsc::Receiver<String>,
    pause_rx: mpsc::Receiver<bool>,
    stop_rx: mpsc::Receiver<()>,
    trigger_haptic_rx: mpsc::Receiver<u8>,
) -> ! {
    loop {
        plugin_thread_loop(
            plugin,
            &app_handle,
            &config_rx,
            &pause_rx,
            &stop_rx,
            &trigger_haptic_rx,
        );
    }
}

fn plugin_thread_loop(
    plugin: &mut SteamdeckPlugin,
    app_handle: &tauri::AppHandle,
    config_rx: &mpsc::Receiver<String>,
    pause_rx: &mpsc::Receiver<bool>,
    stop_rx: &mpsc::Receiver<()>,
    trigger_haptic_rx: &mpsc::Receiver<u8>,
) {
    // stop flag
    match stop_rx.try_recv() {
        Ok(()) => {
            debug!("Sending SIGCONT to steam process");
            match plugin.config.steam_pid {
                Some(steam_pid) => {
                    send_steam_signal(steam_pid, true);
                }
                None => {}
            };
            debug!("Exiting now");
            app_handle.exit(0);
        }
        Err(_) => {}
    };
    // pause flag
    match pause_rx.try_recv() {
        Ok(message_pause) => {
            debug!("[HID thread] new pause flag: {}", message_pause);
            plugin.pause = message_pause;
            pause_update(plugin);
        }
        Err(_) => {}
    };
    // new config
    match config_rx.try_recv() {
        Ok(config_str) => {
            config_update(plugin, config_str);
        }
        Err(_) => {}
    }
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
            match plugin.device.send_feature_report(&mut haptic_report) {
                Ok(_) => {}
                Err(_) => {
                    error!("Failed to send hid feature report for haptic");
                }
            }
        }
        Err(_) => {}
    }
    // input
    let mut buf = [0u8; 64];
    let res = match plugin.device.read(&mut buf[..]) {
        Ok(res) => res,
        Err(_) => {
            warn!("Failed to read device, reopening after short delay");
            sleep(Duration::from_millis(1000));
            plugin.device = match hid_device_factory() {
                Some(device) => device,
                None => {
                    return;
                }
            };
            return;
        }
    };
    if res != 64 {
        error!("USB hid response size wasn't 64 but {}", res);
        return;
    }
    let now = Instant::now();
    if plugin.pause && (now - plugin.last_read).as_millis() < 50 {
        trace!("paused and last read within 50ms, skipping");
        return;
    }
    plugin.last_read = now;
    let device_report = SteamDeckDeviceReport {
        l_pad_x: i16::from_le_bytes(buf[16..18].try_into().unwrap()),
        l_pad_y: i16::from_le_bytes(buf[18..20].try_into().unwrap()),
        l_pad_force: u16::from_le_bytes(buf[56..58].try_into().unwrap()),
        r_pad_x: i16::from_le_bytes(buf[20..22].try_into().unwrap()),
        r_pad_y: i16::from_le_bytes(buf[22..24].try_into().unwrap()),
        r_pad_force: u16::from_le_bytes(buf[58..60].try_into().unwrap()),
        l4: (u32::from_le_bytes(buf[12..16].try_into().unwrap()) & 0x200) > 0,
    };
    plugin.left_touch_history.push_back(TouchEntry {
        x: device_report.l_pad_x,
        y: device_report.l_pad_y,
        force: device_report.l_pad_force,
        time: now,
    });
    plugin.right_touch_history.push_back(TouchEntry {
        x: device_report.r_pad_x,
        y: device_report.r_pad_y,
        force: device_report.r_pad_force,
        time: now,
    });
    while !plugin.left_touch_history.is_empty() {
        match plugin.left_touch_history.front() {
            Some(front) => {
                if now.duration_since(front.time).as_millis() > 2000 {
                    plugin.left_touch_history.pop_front();
                    continue;
                }
            }
            None => (),
        }
        break;
    }
    while !plugin.right_touch_history.is_empty() {
        match plugin.right_touch_history.front() {
            Some(front) => {
                if now.duration_since(front.time).as_millis() > 2000 {
                    plugin.right_touch_history.pop_front();
                    continue;
                }
            }
            None => (),
        }
        break;
    }
    if check_keyboard_toggle(
        &plugin.left_touch_history,
        &plugin.right_touch_history,
        plugin.last_toggle_window,
        plugin.is_visible,
    ) {
        debug!("[HID thread] toggle window");
        plugin.last_toggle_window = Instant::now();
        let state = app_handle.state::<Mutex<AppState>>();
        plugin.is_visible = toggle_window(state, app_handle.clone());
    }
    trace!(
        "[HID thread] \
        left touch history size {}, \
        right touch history size {}",
        plugin.left_touch_history.len(),
        plugin.right_touch_history.len()
    );
    if !plugin.pause {
        let l_x_diff: f32 = (plugin.last_emitted_report.l_pad_x - device_report.l_pad_x).into();
        let l_y_diff: f32 = (plugin.last_emitted_report.l_pad_y - device_report.l_pad_y).into();
        let r_x_diff: f32 = (plugin.last_emitted_report.r_pad_x - device_report.r_pad_x).into();
        let r_y_diff: f32 = (plugin.last_emitted_report.r_pad_y - device_report.r_pad_y).into();
        let l_square_dist = l_x_diff * l_x_diff + l_y_diff * l_y_diff;
        let r_square_dist = r_x_diff * r_x_diff + r_y_diff * r_y_diff;
        let l_pressure_diff = if plugin.last_emitted_report.l_pad_force > device_report.l_pad_force
        {
            plugin.last_emitted_report.l_pad_force - device_report.l_pad_force
        } else {
            device_report.l_pad_force - plugin.last_emitted_report.l_pad_force
        };
        let r_pressure_diff = if plugin.last_emitted_report.r_pad_force > device_report.r_pad_force
        {
            plugin.last_emitted_report.r_pad_force - device_report.r_pad_force
        } else {
            device_report.r_pad_force - plugin.last_emitted_report.r_pad_force
        };
        if l_square_dist > plugin.deadzone_dist_square
            || r_square_dist > plugin.deadzone_dist_square
            || l_pressure_diff > plugin.deadzone_pressure
            || r_pressure_diff > plugin.deadzone_pressure
        {
            plugin.last_emitted_report = device_report.clone();
            app_handle
                .emit("input", device_report)
                .expect("Should be able to emit device report");
        }
    }
}

fn pause_update(plugin: &mut SteamdeckPlugin) {
    if plugin.config.steam_pid.is_some() && !is_pid_alive(plugin.config.steam_pid.unwrap()) {
        match get_steam_pid() {
            Some(steam_pid) => plugin.config.steam_pid = Some(steam_pid),
            None => {}
        }
    }
    match plugin.config.steam_pid {
        Some(steam_pid) => {
            send_steam_signal(steam_pid, !plugin.is_visible);
        }
        None => {}
    }
}

fn config_update(plugin: &mut SteamdeckPlugin, config_str: String) {
    plugin.config =
        serde_json::from_str(config_str.as_str()).expect("Config string could not be parsed");
    match plugin.config.deadzone_dist {
        Some(deadzone_dist) => {
            trace!("got deadzone_dist {}", deadzone_dist);
            plugin.deadzone_dist_square = deadzone_dist * deadzone_dist;
        }
        None => {}
    }
    match plugin.config.deadzone_pressure {
        Some(deadzone_pressure) => {
            trace!("got deadzone_pressure {}", deadzone_pressure);
            plugin.deadzone_pressure = deadzone_pressure;
        }
        None => {}
    }
    let config_steam_pid = plugin.config.steam_pid;
    plugin.config.steam_pid = get_steam_pid();
    if plugin.config.steam_pid.is_none() {
        match config_steam_pid {
            Some(steam_pid) => {
                debug!("Using steam pid {} from config", steam_pid);
                plugin.config.steam_pid = Some(steam_pid);
                send_steam_signal(steam_pid, true);
            }
            None => {}
        };
    }
}

fn hid_device_factory() -> Option<HidDevice> {
    let api = hidapi::HidApi::new().unwrap();
    for device in api.device_list() {
        debug!(
            "[HID thread] device: {:#?}, path:{:?}, vendor id: {}, product id: {}",
            device,
            device.path(),
            device.vendor_id(),
            device.product_id()
        );
    }
    let (vid, pid) = (0x28de, 0x1205);
    let device_info_opt = api
        .device_list()
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
        }
        Err(_) => None,
    };
}

fn check_keyboard_toggle(
    left_touch_history: &VecDeque<TouchEntry>,
    right_touch_history: &VecDeque<TouchEntry>,
    last_toggle_window: Instant,
    is_visible: bool,
) -> bool {
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
            Err(_) => continue,
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
        debug!("Sending stop signal to steam process");
        signal = libc::SIGSTOP;
    } else {
        debug!("Sending continue signal to steam process");
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
        Ok(_) => {}
        Err(e) => {
            error!("failed to write hid settings {}", e);
        }
    }
}
