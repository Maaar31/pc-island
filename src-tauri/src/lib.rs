use std::sync::Mutex;
use std::time::Duration;

use anyhow::anyhow;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow,
};

mod modules;

const PANEL_WIDTH: u32 = 520;
const PANEL_HEIGHT: u32 = 340;
const WAKE_WIDTH: u32 = 240;
const WAKE_HEIGHT: u32 = 10;

fn tray_icon() -> tauri::image::Image<'static> {
    let size = 16u32;
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let dx = x as i32 - 8;
            let dy = y as i32 - 8;
            if dx * dx + dy * dy <= 42 {
                let index = ((y * size + x) * 4) as usize;
                pixels[index..index + 4].copy_from_slice(&[137, 180, 250, 255]);
            }
        }
    }
    tauri::image::Image::new_owned(pixels, size, size)
}

#[derive(Default)]
struct WindowState {
    open: Mutex<bool>,
}

fn place_top_center(window: &WebviewWindow, width: u32, height: u32) -> tauri::Result<()> {
    let monitor = window
        .current_monitor()?
        .or(window.primary_monitor()?)
        .ok_or_else(|| tauri::Error::Anyhow(anyhow!("No monitor available")))?;
    let position = monitor.position();
    let size = monitor.size();
    let x = position.x + (size.width as i32 - width as i32) / 2;
    let y = position.y;
    window.set_size(PhysicalSize::new(width, height))?;
    window.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}

fn configure_window(window: &WebviewWindow, width: u32, height: u32) -> tauri::Result<()> {
    place_top_center(window, width, height)?;
    window.set_always_on_top(true)?;
    window.set_focusable(false)?;
    window.set_ignore_cursor_events(false)?;
    Ok(())
}

#[cfg(windows)]
fn set_native_region(window: &WebviewWindow, rect: Option<(i32, i32, i32, i32)>) -> tauri::Result<()> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, SetWindowRgn};

    let hwnd = window
        .hwnd()
        ?;
    let hwnd = HWND(hwnd.0 as _);
    let region = match rect {
        Some((x, y, width, height)) if width > 0 && height > 0 => unsafe {
            CreateRectRgn(x, y, x + width, y + height)
        },
        _ => unsafe { CreateRectRgn(0, 0, 0, 0) },
    };
    if region.is_invalid() {
        return Err(tauri::Error::Anyhow(anyhow!("CreateRectRgn failed")));
    }

    let result = unsafe { SetWindowRgn(hwnd, Some(region), true) };
    if result == 0 {
        unsafe {
            let _ = DeleteObject(region.into());
        }

        return Err(tauri::Error::Anyhow(anyhow!("SetWindowRgn failed")));
    }
    Ok(())
}

#[cfg(not(windows))]
fn set_native_region(_window: &WebviewWindow, _rect: Option<(i32, i32, i32, i32)>) -> tauri::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn start_wake_poller(app: AppHandle) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetForegroundWindow, GetWindowRect,
    };

    fn foreground_is_fullscreen() -> bool {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground.is_invalid() {
                return false;
            }
            let monitor = MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST);
            if monitor.is_invalid() {
                return false;
            }
            let mut monitor_info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let mut window_rect = windows::Win32::Foundation::RECT::default();
            if GetMonitorInfoW(monitor, &mut monitor_info).as_bool()
                && GetWindowRect(foreground, &mut window_rect).is_ok()
            {
                return window_rect == monitor_info.rcMonitor;
            }
            false
        }
    }

    std::thread::spawn(move || {
        let mut was_inside = false;
        loop {
            let Some(window) = app.get_webview_window("island") else {
                break;
            };
            let inside = (|| -> tauri::Result<bool> {
                if foreground_is_fullscreen() {
                    return Ok(false);
                }
                let position = window.outer_position()?;
                let size = window.outer_size()?;
                let scale = window.scale_factor()?.max(0.1);
                let mut cursor = POINT::default();
                if unsafe { GetCursorPos(&mut cursor) }.is_err() {
                    return Ok(false);
                }
                let local_x = cursor.x - position.x;
                let local_y = cursor.y - position.y;
                let wake_width = (WAKE_WIDTH as f64 * scale).round() as i32;
                let wake_height = (WAKE_HEIGHT as f64 * scale).round() as i32;
                let left = (size.width as i32 - wake_width) / 2;
                Ok(local_x >= left
                    && local_x <= left + wake_width
                    && local_y >= 0
                    && local_y <= wake_height)
            })()
            .unwrap_or(false);

            if inside != was_inside {
                was_inside = inside;
                let _ = app.emit(if inside { "wake-enter" } else { "wake-leave" }, ());
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    });
}

#[tauri::command]
fn set_open(app: AppHandle, state: State<WindowState>, open: bool) -> tauri::Result<()> {
    *state.open.lock().unwrap() = open;
    let island = app
        .get_webview_window("island")
        .ok_or_else(|| tauri::Error::Anyhow(anyhow!("Island window missing")))?;
    island.set_ignore_cursor_events(!open)?;
    Ok(())
}

#[tauri::command]
fn set_wake_visible(app: AppHandle, visible: bool) -> tauri::Result<()> {
    let island = app
        .get_webview_window("island")
        .ok_or_else(|| tauri::Error::Anyhow(anyhow!("Island window missing")))?;
    let scale = island.scale_factor()?.max(0.1);
    if visible {
        set_native_region(
            &island,
            Some((
                ((PANEL_WIDTH as f64 - WAKE_WIDTH as f64) * scale / 2.0).round() as i32,
                0,
                (WAKE_WIDTH as f64 * scale).round() as i32,
                (WAKE_HEIGHT as f64 * scale).round() as i32,
            )),
        )?;
    }
    Ok(())
}

#[tauri::command]
fn set_panel_rect(
    app: AppHandle,
    state: State<WindowState>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> tauri::Result<()> {
    let _ = state;
    let island = app
        .get_webview_window("island")
        .ok_or_else(|| tauri::Error::Anyhow(anyhow!("Island window missing")))?;
    let scale = island.scale_factor()?.max(0.1);
    let rect = (
        (x * scale).round() as i32,
        (y * scale).round() as i32,
        (width * scale).round() as i32,
        (height * scale).round() as i32,
    );
    set_native_region(&island, Some(rect))
}

#[tauri::command]
fn fit_zoom(app: AppHandle, dpr: f64) -> tauri::Result<bool> {
    let island = app
        .get_webview_window("island")
        .ok_or_else(|| tauri::Error::Anyhow(anyhow!("Island window missing")))?;
    let scale = island.scale_factor()?.max(0.1);
    if !dpr.is_finite() || dpr <= 0.0 || (dpr - scale).abs() / scale < 0.02 {
        return Ok(false);
    }

    island.set_zoom((scale / dpr).clamp(0.25, 4.0))?;
    Ok(true)
}

#[tauri::command]
fn pomodoro_start(state: State<modules::ModuleState>, duration_seconds: u64) -> modules::PomodoroSnapshot {
    modules::start_pomodoro(&state, duration_seconds)
}

#[tauri::command]
fn pomodoro_stop(state: State<modules::ModuleState>) -> modules::PomodoroSnapshot {
    modules::stop_pomodoro(&state)
}

#[tauri::command]
fn pomodoro_status(state: State<modules::ModuleState>) -> modules::PomodoroSnapshot {
    modules::pomodoro_snapshot(&state)
}

#[tauri::command]
fn system_volume() -> Result<u8, String> {
    modules::master_volume()
}

#[tauri::command]
fn set_system_volume(value: u8) -> Result<u8, String> {
    modules::set_master_volume(value)
}

#[tauri::command]
fn system_muted() -> Result<bool, String> {
    modules::master_muted()
}

#[tauri::command]
fn set_system_muted(muted: bool) -> Result<bool, String> {
    modules::set_master_muted(muted)
}

#[tauri::command]
fn radio_status() -> modules::RadioSnapshot {
    modules::radio_status()
}

#[tauri::command]
fn set_radio(kind: String, enabled: bool) -> Result<modules::RadioSnapshot, String> {
    modules::set_radio(&kind, enabled)
}

#[tauri::command]
fn clipboard_read() -> Result<String, String> {
    modules::read_clipboard()
}

#[tauri::command]
fn clipboard_write(state: State<modules::ClipboardHistoryState>, text: String) -> Result<(), String> {
    modules::write_clipboard(&text)?;
    modules::remember_clipboard(&state, text);
    Ok(())
}

#[tauri::command]
fn clipboard_history(state: State<modules::ClipboardHistoryState>) -> Vec<String> {
    modules::clipboard_history(&state)
}

#[cfg(windows)]
fn autostart_enabled() -> Result<bool, String> {
    use windows::core::w;
    use windows::Win32::System::Registry::{
        RegOpenKeyExW, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
    };
    let mut key = HKEY_CURRENT_USER;
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            Some(0),
            KEY_QUERY_VALUE,
            &mut key,
        )
    };
    if result.is_err() {
        return Ok(false);
    }
    let mut size = 0u32;
    let result = unsafe {
        windows::Win32::System::Registry::RegGetValueW(
            key,
            None,
            w!("Island"),
            windows::Win32::System::Registry::RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
    };
    unsafe { let _ = windows::Win32::System::Registry::RegCloseKey(key); };
    Ok(result.is_ok() && size > 0)
}

#[cfg(not(windows))]
fn autostart_enabled() -> Result<bool, String> {
    Ok(false)
}

#[cfg(windows)]
fn set_autostart(enabled: bool) -> Result<bool, String> {
    use windows::core::w;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY_CURRENT_USER,
        KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };
    let mut key = HKEY_CURRENT_USER;
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
    };
    if status.is_err() {
        return Err(format!("Erreur registre Windows: {:?}", status));
    }
    let result = if enabled {
        let path = std::env::current_exe().map_err(|error| error.to_string())?;
        let value = format!("\"{}\"", path.display());
        let mut data = value.encode_utf16().collect::<Vec<_>>();
        data.push(0);
        let bytes = unsafe {
            std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2)
        };
        unsafe { RegSetValueExW(key, w!("Island"), None, REG_SZ, Some(bytes)) }
    } else {
        unsafe { RegDeleteValueW(key, w!("Island")) }
    };
    unsafe { let _ = RegCloseKey(key); };
    if result.is_err() {
        return Err(format!("Erreur registre Windows: {:?}", result));
    }
    Ok(enabled)
}

#[cfg(not(windows))]
fn set_autostart(_enabled: bool) -> Result<bool, String> {
    Err("L'autostart est disponible uniquement sous Windows".into())
}

#[tauri::command]
fn autostart_status() -> Result<bool, String> {
    autostart_enabled()
}

#[tauri::command]
fn set_autostart_command(enabled: bool) -> Result<bool, String> {
    set_autostart(enabled)
}

#[tauri::command]
fn media_status() -> modules::MediaSnapshot {
    modules::media_status()
}

#[tauri::command]
fn media_control(action: String) -> Result<modules::MediaSnapshot, String> {
    modules::media_control(&action)
}

pub fn run() {
    tauri::Builder::default()
        .manage(WindowState::default())
        .manage(modules::ModuleState::default())
        .manage(modules::ClipboardHistoryState::default())
        .invoke_handler(tauri::generate_handler![
            set_open,
            set_wake_visible,
            set_panel_rect,
            fit_zoom,
            pomodoro_start,
            pomodoro_stop,
            pomodoro_status,
            system_volume,
            set_system_volume,
            system_muted,
            set_system_muted,
            radio_status,
            set_radio,
            clipboard_read,
            clipboard_write,
            clipboard_history,
            autostart_status,
            set_autostart_command,
            media_status,
            media_control
        ])
        .setup(|app| {
            let island = app
                .get_webview_window("island")
                .ok_or_else(|| tauri::Error::Anyhow(anyhow!("Island window missing")))?;
            configure_window(&island, PANEL_WIDTH, PANEL_HEIGHT)?;
            let scale = island.scale_factor()?.max(0.1);
            set_native_region(
                &island,
                Some((
                    ((PANEL_WIDTH as f64 - WAKE_WIDTH as f64) * scale / 2.0).round() as i32,
                    0,
                    (WAKE_WIDTH as f64 * scale).round() as i32,
                    (WAKE_HEIGHT as f64 * scale).round() as i32,
                )),
            )?;
            #[cfg(windows)]
            start_wake_poller(app.handle().clone());

            let quit = MenuItem::with_id(app, "quit", "Quitter Island", true, None::<&str>)?;
            let autostart = CheckMenuItem::with_id(
                app,
                "autostart",
                "Lancer au démarrage de Windows",
                true,
                autostart_enabled().unwrap_or(false),
                None::<&str>,
            )?;
            let menu = Menu::with_items(app, &[&autostart, &quit])?;
            let autostart_for_handler = autostart.clone();
            TrayIconBuilder::with_id("island-tray")
                .icon(tray_icon())
                .tooltip("Island")
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    } else if event.id() == "autostart" {
                        let current = autostart_enabled().unwrap_or(false);
                        if let Ok(enabled) = set_autostart(!current) {
                            let _ = autostart_for_handler.set_checked(enabled);
                        }
                    }
                })
                .build(app)?;
            let history = app.state::<modules::ClipboardHistoryState>().history.clone();
            modules::start_clipboard_watcher(history);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Island");
}
