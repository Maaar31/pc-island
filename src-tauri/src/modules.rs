use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

#[derive(Default)]
pub struct ModuleState {
    pub pomodoro: Mutex<Pomodoro>,
}

#[derive(Default)]
pub struct Pomodoro {
    pub started_at: Option<Instant>,
    pub duration: Duration,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroSnapshot {
    pub running: bool,
    pub remaining_seconds: u64,
    pub duration_seconds: u64,
}

pub fn pomodoro_snapshot(state: &ModuleState) -> PomodoroSnapshot {
    let timer = state.pomodoro.lock().unwrap();
    let duration = if timer.duration.is_zero() {
        Duration::from_secs(25 * 60)
    } else {
        timer.duration
    };
    let remaining = timer
        .started_at
        .map(|started| duration.saturating_sub(started.elapsed()).as_secs())
        .unwrap_or(duration.as_secs());
    PomodoroSnapshot {
        running: timer.started_at.is_some() && remaining > 0,
        remaining_seconds: remaining,
        duration_seconds: duration.as_secs(),
    }
}

pub fn start_pomodoro(state: &ModuleState, duration_seconds: u64) -> PomodoroSnapshot {
    let mut timer = state.pomodoro.lock().unwrap();
    timer.duration = Duration::from_secs(duration_seconds.clamp(60, 4 * 60 * 60));
    timer.started_at = Some(Instant::now());
    drop(timer);
    pomodoro_snapshot(state)
}

pub fn stop_pomodoro(state: &ModuleState) -> PomodoroSnapshot {
    let mut timer = state.pomodoro.lock().unwrap();
    timer.started_at = None;
    drop(timer);
    pomodoro_snapshot(state)
}

#[cfg(windows)]
pub fn master_volume() -> Result<u8, String> {
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|error| error.to_string())?;
        let devices: IMMDeviceEnumerator =
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|error| error.to_string())?;
        let device = devices
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|error| error.to_string())?;
        let endpoint: IAudioEndpointVolume = device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(|error| error.to_string())?;
        let value = endpoint
            .GetMasterVolumeLevelScalar()
            .map_err(|error| error.to_string())?;
        Ok((value * 100.0).round().clamp(0.0, 100.0) as u8)
    }
}

#[cfg(not(windows))]
pub fn master_volume() -> Result<u8, String> {
    Err("Le volume système est disponible uniquement sous Windows".into())
}

#[cfg(windows)]
pub fn set_master_volume(value: u8) -> Result<u8, String> {
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|error| error.to_string())?;
        let devices: IMMDeviceEnumerator =
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|error| error.to_string())?;
        let device = devices
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|error| error.to_string())?;
        let endpoint: IAudioEndpointVolume = device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(|error| error.to_string())?;
        endpoint
            .SetMasterVolumeLevelScalar((value.min(100) as f32) / 100.0, std::ptr::null())
            .map_err(|error| error.to_string())?;
        Ok(value.min(100))
    }
}

#[cfg(not(windows))]
pub fn set_master_volume(_value: u8) -> Result<u8, String> {
    Err("Le volume système est disponible uniquement sous Windows".into())
}

pub fn read_clipboard() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard.get_text().map_err(|error| error.to_string())
}

pub fn write_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard.set_text(text).map_err(|error| error.to_string())
}
