import { config } from "./config";
import {
  currentWindowLabel,
  emitWakeEnter,
  installWakeListener,
  isTauri,
  setOpen,
  setPanelRect,
  setWakeVisible,
  setZoomForDpr
} from "./bridge";
import "./style.css";

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("Island root is missing");

const panel = document.createElement("section");
panel.id = "panel";
panel.setAttribute("aria-hidden", "true");
app.append(panel);

let openTimer: number | undefined;
let closeTimer: number | undefined;
let open = false;
let regionFrame: number | undefined;

function clearTimer(timer: number | undefined) {
  if (timer !== undefined) window.clearTimeout(timer);
}

function pushPanelRect() {
  const rect = panel.getBoundingClientRect();
  void setPanelRect({ x: rect.x, y: rect.y, width: rect.width, height: rect.height });
}

function stopRegionAnimation() {
  if (regionFrame !== undefined) {
    window.cancelAnimationFrame(regionFrame);
    regionFrame = undefined;
  }
}

function animatePanelRegion() {
  pushPanelRect();
  if (open) regionFrame = window.requestAnimationFrame(animatePanelRegion);
}

function openPanel() {
  clearTimer(closeTimer);
  closeTimer = undefined;
  if (open) return;
  open = true;
  panel.classList.add("open");
  panel.setAttribute("aria-hidden", "false");
  void setOpen(true);
  void setWakeVisible(false);
  stopRegionAnimation();
  regionFrame = window.requestAnimationFrame(animatePanelRegion);
}

function closePanel() {
  clearTimer(openTimer);
  openTimer = undefined;
  if (!open) return;
  open = false;
  panel.classList.remove("open");
  panel.setAttribute("aria-hidden", "true");
  void setOpen(false);
  stopRegionAnimation();
  window.setTimeout(() => {
    if (!open) void setWakeVisible(true);
  }, config.timing.animationMs);
  window.requestAnimationFrame(pushPanelRect);
}

function scheduleOpen() {
  clearTimer(closeTimer);
  closeTimer = undefined;
  if (open || openTimer !== undefined) return;
  openTimer = window.setTimeout(() => {
    openTimer = undefined;
    openPanel();
  }, config.timing.hoverOpenMs);
}

function scheduleClose() {
  clearTimer(openTimer);
  openTimer = undefined;
  if (!open || closeTimer !== undefined) return;
  closeTimer = window.setTimeout(() => {
    closeTimer = undefined;
    closePanel();
  }, config.timing.hoverCloseMs);
}

panel.addEventListener("mouseenter", scheduleOpen);
panel.addEventListener("mouseleave", scheduleClose);
panel.addEventListener("transitionend", () => {
  if (!open) pushPanelRect();
});

if (isTauri) {
  void currentWindowLabel().then((label) => {
    if (label === "wake") {
      window.addEventListener("mouseenter", () => void emitWakeEnter());
      return;
    }
    void installWakeListener(scheduleOpen);
    void setZoomForDpr(window.devicePixelRatio);
    void setWakeVisible(true);
  });
}

window.addEventListener("resize", pushPanelRect);
