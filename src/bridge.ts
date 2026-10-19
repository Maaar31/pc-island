import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { emit, listen } from "@tauri-apps/api/event";

export const isTauri = "__TAURI_INTERNALS__" in window;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T | null> {
  if (!isTauri) return null;
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    console.error(`[island] ${command} failed`, error);
    return null;
  }
}

export function setPanelRect(rect: { x: number; y: number; width: number; height: number }) {
  return call("set_panel_rect", rect);
}

export function setOpen(open: boolean) {
  return call("set_open", { open });
}

export function setWakeVisible(visible: boolean) {
  return call("set_wake_visible", { visible });
}

export function setZoomForDpr(dpr: number) {
  return call<boolean>("fit_zoom", { dpr });
}

export function pomodoroStart(durationSeconds = 25 * 60) {
  return call<{ running: boolean; remainingSeconds: number; durationSeconds: number }>(
    "pomodoro_start",
    { durationSeconds }
  );
}

export function pomodoroStop() {
  return call<{ running: boolean; remainingSeconds: number; durationSeconds: number }>("pomodoro_stop");
}

export function pomodoroStatus() {
  return call<{ running: boolean; remainingSeconds: number; durationSeconds: number }>("pomodoro_status");
}

export function readClipboard() {
  return call<string>("clipboard_read");
}

export function writeClipboard(text: string) {
  return call<void>("clipboard_write", { text });
}

export function systemVolume() {
  return call<number>("system_volume");
}

export function setSystemVolume(value: number) {
  return call<number>("set_system_volume", { value });
}

export function microphoneVolume() {
  return call<number>("microphone_volume");
}

export function setMicrophoneVolume(value: number) {
  return call<number>("set_microphone_volume", { value });
}

export type RadioSnapshot = { wifi: boolean | null; bluetooth: boolean | null };

export function radioStatus() {
  return call<RadioSnapshot>("radio_status");
}

export function setRadio(kind: "wifi" | "bluetooth", enabled: boolean) {
  return call<RadioSnapshot>("set_radio", { kind, enabled });
}

export type MediaSnapshot = {
  available: boolean;
  title: string;
  artist: string;
  playing: boolean;
};

export function mediaStatus() {
  return call<MediaSnapshot>("media_status");
}

export function mediaControl(action: "toggle" | "next" | "previous") {
  return call<MediaSnapshot>("media_control", { action });
}

export async function installWakeListener(onEnter: () => void) {
  if (!isTauri) return;
  await listen("wake-enter", onEnter);
}

export async function installWakeLeaveListener(onLeave: () => void) {
  if (!isTauri) return;
  await listen("wake-leave", onLeave);
}

export async function emitWakeEnter() {
  if (isTauri) await emit("wake-enter");
}

export async function currentWindowLabel() {
  return isTauri ? (await getCurrentWindow()).label : "browser";
}
