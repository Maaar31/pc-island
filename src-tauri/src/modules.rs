use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

#[derive(Default)]
pub struct ModuleState {
    pub pomodoro: Mutex<Pomodoro>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSnapshot {
    pub available: bool,
    pub title: String,
    pub artist: String,
    pub playing: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RadioSnapshot {
    pub wifi: Option<bool>,
    pub bluetooth: Option<bool>,
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
fn ensure_com_initialized() -> Result<(), String> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    unsafe {
        let result = CoInitializeEx(None, COINIT_MULTITHREADED);
        if result.is_ok() || result.0 == 0x80010106u32 as i32 {
            Ok(())
        } else {
            Err(result.to_string())
        }
    }
}

#[cfg(windows)]
pub fn master_volume() -> Result<u8, String> {
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    unsafe {
        ensure_com_initialized()?;
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
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    unsafe {
        ensure_com_initialized()?;
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

#[cfg(windows)]
pub fn radio_status() -> RadioSnapshot {
    use windows::Devices::Radios::{Radio, RadioKind, RadioState};

    let radios = Radio::GetRadiosAsync().and_then(|operation| operation.get());
    let Ok(radios) = radios else {
        return RadioSnapshot { wifi: None, bluetooth: None };
    };
    let mut snapshot = RadioSnapshot { wifi: None, bluetooth: None };
    for index in 0..radios.Size().unwrap_or(0) {
        let Ok(radio) = radios.GetAt(index) else { continue };
        let (Ok(kind), Ok(state)) = (radio.Kind(), radio.State()) else { continue };
        let value = match state {
            RadioState::On => Some(true),
            RadioState::Off | RadioState::Disabled => Some(false),
            _ => None,
        };
        match kind {
            RadioKind::WiFi => snapshot.wifi = value,
            RadioKind::Bluetooth => snapshot.bluetooth = value,
            _ => {}
        }
    }
    snapshot
}

#[cfg(not(windows))]
pub fn radio_status() -> RadioSnapshot {
    RadioSnapshot { wifi: None, bluetooth: None }
}

#[cfg(windows)]
pub fn set_radio(kind: &str, enabled: bool) -> Result<RadioSnapshot, String> {
    use windows::Devices::Radios::{Radio, RadioKind, RadioState};

    let requested = match kind {
        "wifi" => RadioKind::WiFi,
        "bluetooth" => RadioKind::Bluetooth,
        _ => return Err(format!("Radio inconnue: {kind}")),
    };
    let radios = Radio::GetRadiosAsync()
        .map_err(|error| error.to_string())?
        .get()
        .map_err(|error| error.to_string())?;
    for index in 0..radios.Size().unwrap_or(0) {
        let radio = radios.GetAt(index).map_err(|error| error.to_string())?;
        if radio.Kind().map_err(|error| error.to_string())? == requested {
            let access = radio
                .SetStateAsync(if enabled { RadioState::On } else { RadioState::Off })
                .map_err(|error| error.to_string())?
                .get()
                .map_err(|error| error.to_string())?;
            if access != windows::Devices::Radios::RadioAccessStatus::Allowed {
                return Err("Windows a refusé la modification de cette radio".into());
            }
            return Ok(radio_status());
        }
    }
    Err("Cette radio n'est pas disponible".into())
}

#[cfg(not(windows))]
pub fn set_radio(_kind: &str, _enabled: bool) -> Result<RadioSnapshot, String> {
    Err("Les radios sont disponibles uniquement sous Windows".into())
}

pub fn read_clipboard() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard.get_text().map_err(|error| error.to_string())
}

pub fn write_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard.set_text(text).map_err(|error| error.to_string())
}

#[cfg(windows)]
fn current_media_session() -> Result<windows::Media::Control::GlobalSystemMediaTransportControlsSession, String> {
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;

    GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(|error| error.to_string())?
        .get()
        .map_err(|error| error.to_string())?
        .GetCurrentSession()
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
pub fn media_status() -> MediaSnapshot {
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus;

    let Ok(session) = current_media_session() else {
        return MediaSnapshot {
            available: false,
            title: String::new(),
            artist: String::new(),
            playing: false,
        };
    };
    let Ok(properties) = session.TryGetMediaPropertiesAsync().and_then(|operation| operation.get()) else {
        return MediaSnapshot {
            available: false,
            title: String::new(),
            artist: String::new(),
            playing: false,
        };
    };
    let title = properties
        .Title()
        .map(|value| value.to_string_lossy())
        .unwrap_or_default();
    let artist = properties
        .Artist()
        .map(|value| value.to_string_lossy())
        .unwrap_or_default();
    let playing = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .is_ok_and(|status| status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing);
    MediaSnapshot {
        available: !title.trim().is_empty(),
        title,
        artist,
        playing,
    }
}

#[cfg(not(windows))]
pub fn media_status() -> MediaSnapshot {
    MediaSnapshot {
        available: false,
        title: String::new(),
        artist: String::new(),
        playing: false,
    }
}

#[cfg(windows)]
pub fn media_control(action: &str) -> Result<MediaSnapshot, String> {
    let session = current_media_session()?;
    let operation = match action {
        "toggle" => session.TryTogglePlayPauseAsync().map(|operation| operation.get()),
        "next" => session.TrySkipNextAsync().map(|operation| operation.get()),
        "previous" => session.TrySkipPreviousAsync().map(|operation| operation.get()),
        _ => return Err(format!("Commande média inconnue: {action}")),
    };
    let accepted = operation
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    if !accepted {
        return Err("Le lecteur média a refusé la commande".into());
    }
    Ok(media_status())
}

#[cfg(not(windows))]
pub fn media_control(_action: &str) -> Result<MediaSnapshot, String> {
    Err("Les contrôles média sont disponibles uniquement sous Windows".into())
}
