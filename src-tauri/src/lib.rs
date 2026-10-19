use std::sync::Mutex;
use std::time::Duration;

use anyhow::anyhow;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow};

mod modules;

const PANEL_WIDTH: u32 = 520;
const PANEL_HEIGHT: u32 = 340;
const WAKE_WIDTH: u32 = 240;
const WAKE_HEIGHT: u32 = 10;

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
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    std::thread::spawn(move || {
        let mut was_inside = false;
        loop {
            let Some(window) = app.get_webview_window("island") else {
                break;
            };
            let inside = (|| -> tauri::Result<bool> {
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
fn clipboard_read() -> Result<String, String> {
    modules::read_clipboard()
}

#[tauri::command]
fn clipboard_write(text: String) -> Result<(), String> {
    modules::write_clipboard(&text)
}

pub fn run() {
    tauri::Builder::default()
        .manage(WindowState::default())
        .manage(modules::ModuleState::default())
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
            clipboard_read,
            clipboard_write
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Island");
}
