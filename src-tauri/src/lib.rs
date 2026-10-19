use std::sync::Mutex;

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow};

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
        .ok_or_else(|| tauri::Error::Anyhow("No monitor available".into()))?;
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
    window.set_ignore_cursor_events(true)?;
    Ok(())
}

#[cfg(windows)]
fn set_native_region(window: &WebviewWindow, rect: Option<(i32, i32, i32, i32)>) -> tauri::Result<()> {
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, SetWindowRgn};

    let hwnd = window
        .hwnd()
        .map_err(|error| tauri::Error::Anyhow(error.to_string().into()))?;
    let hwnd = HWND(hwnd.0 as _);
    let region = match rect {
        Some((x, y, width, height)) if width > 0 && height > 0 => unsafe {
            CreateRectRgn(x, y, x + width, y + height)
        },
        _ => unsafe { CreateRectRgn(0, 0, 0, 0) },
    };
    if region.is_invalid() {
        return Err(tauri::Error::Anyhow("CreateRectRgn failed".into()));
    }
    let result = unsafe { SetWindowRgn(hwnd, region, BOOL(1)) };
    if result == 0 {
        unsafe {
            let _ = DeleteObject(region.into());
        }
        return Err(tauri::Error::Anyhow("SetWindowRgn failed".into()));
    }
    Ok(())
}

#[cfg(not(windows))]
fn set_native_region(_window: &WebviewWindow, _rect: Option<(i32, i32, i32, i32)>) -> tauri::Result<()> {
    Ok(())
}

#[tauri::command]
fn set_open(app: AppHandle, state: State<WindowState>, open: bool) -> tauri::Result<()> {
    *state.open.lock().unwrap() = open;
    let island = app
        .get_webview_window("island")
        .ok_or_else(|| tauri::Error::Anyhow("Island window missing".into()))?;
    let wake = app
        .get_webview_window("wake")
        .ok_or_else(|| tauri::Error::Anyhow("Wake window missing".into()))?;
    island.set_ignore_cursor_events(!open)?;
    wake.set_ignore_cursor_events(true)?;
    if open {
        wake.hide()?;
    } else {
        set_native_region(&island, None)?;
        wake.show()?;
    }
    Ok(())
}

#[tauri::command]
fn set_wake_visible(app: AppHandle, visible: bool) -> tauri::Result<()> {
    let wake = app
        .get_webview_window("wake")
        .ok_or_else(|| tauri::Error::Anyhow("Wake window missing".into()))?;
    if visible {
        place_top_center(&wake, WAKE_WIDTH, WAKE_HEIGHT)?;
        wake.show()?;
    } else {
        wake.hide()?;
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
    if !*state.open.lock().unwrap() {
        return Ok(());
    }
    let island = app
        .get_webview_window("island")
        .ok_or_else(|| tauri::Error::Anyhow("Island window missing".into()))?;
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
        .ok_or_else(|| tauri::Error::Anyhow("Island window missing".into()))?;
    let scale = island.scale_factor()?.max(0.1);
    if !dpr.is_finite() || dpr <= 0.0 || (dpr - scale).abs() / scale < 0.02 {
        return Ok(false);
    }
    island.set_zoom((scale / dpr).clamp(0.25, 4.0))?;
    Ok(true)
}

pub fn run() {
    tauri::Builder::default()
        .manage(WindowState::default())
        .invoke_handler(tauri::generate_handler![set_open, set_wake_visible, set_panel_rect, fit_zoom])
        .setup(|app| {
            let island = app
                .get_webview_window("island")
                .ok_or_else(|| tauri::Error::Anyhow("Island window missing".into()))?;
            let wake = app
                .get_webview_window("wake")
                .ok_or_else(|| tauri::Error::Anyhow("Wake window missing".into()))?;
            configure_window(&island, PANEL_WIDTH, PANEL_HEIGHT)?;
            configure_window(&wake, WAKE_WIDTH, WAKE_HEIGHT)?;
            wake.set_ignore_cursor_events(false)?;
            set_native_region(&island, None)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Island");
}
