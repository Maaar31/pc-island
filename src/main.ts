import { config } from "./config";
import {
  isTauri,
  setOpen,
  setPanelRect,
  setWakeVisible,
  setZoomForDpr,
  installWakeListener,
  installWakeLeaveListener,
  pomodoroStart,
  pomodoroStop,
  pomodoroStatus,
  readClipboard,
  writeClipboard
} from "./bridge";
import "./style.css";

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("Island root is missing");

const panel = document.createElement("section");
panel.id = "panel";
panel.setAttribute("aria-hidden", "true");

type TabId = "notifications" | "pomodoro" | "clipboard";

panel.innerHTML = `
  <div class="panel-shell">
    <header class="panel-header">
      <div>
        <div class="eyebrow">Island</div>
        <h1 class="panel-title">Vue principale</h1>
      </div>
      <button class="panel-close" type="button" aria-label="Réduire">−</button>
    </header>
    <div class="main-content">
      <article class="module-card media-card">
        <div>
          <div class="module-label">Lecture en cours</div>
          <div class="media-meta">Aucune lecture</div>
          <div class="media-artist">Données de test</div>
        </div>
        <div class="media-controls" aria-label="Contrôles média">
          <button class="media-control" type="button" aria-label="Précédent">‹</button>
          <button class="media-control primary" type="button" aria-label="Lecture">▶</button>
          <button class="media-control" type="button" aria-label="Suivant">›</button>
        </div>
      </article>
      <article class="module-card sliders-card">
        <div>
          <div class="module-row"><span class="module-label">Volume</span><span class="module-value">62 %</span></div>
          <div class="fake-slider" aria-hidden="true"></div>
        </div>
        <div>
          <div class="module-row"><span class="module-label">Luminosité</span><span class="module-value">—</span></div>
          <div class="fake-slider" aria-hidden="true"></div>
        </div>
      </article>
    </div>
    <footer class="panel-footer">
      <span>Modules en préparation</span>
      <button class="expand-button" type="button">Ouvrir les pages</button>
    </footer>
  </div>
`;
app.append(panel);

const browserWake = document.createElement("div");
browserWake.id = "browser-wake";
browserWake.setAttribute("aria-label", "Zone de test du survol");
app.append(browserWake);

const shell = panel.querySelector<HTMLDivElement>(".panel-shell");
const closeButton = panel.querySelector<HTMLButtonElement>(".panel-close");
const expandButton = panel.querySelector<HTMLButtonElement>(".expand-button");
if (!shell || !closeButton || !expandButton) throw new Error("Island controls are missing");
const panelShell = shell;

const extended = document.createElement("div");
extended.className = "extended-content";
extended.hidden = true;
extended.innerHTML = `
  <nav class="tab-list" aria-label="Pages">
    <button class="tab-button active" type="button" data-tab="notifications">Alertes</button>
    <button class="tab-button" type="button" data-tab="pomodoro">Pomodoro</button>
    <button class="tab-button" type="button" data-tab="clipboard">Presse-papier</button>
  </nav>
  <div class="tab-page active" data-page="notifications">
    <h2>Notifications</h2>
    <p>La liste des notifications sera connectée pendant la phase dédiée.</p>
    <div class="placeholder-list"><div class="placeholder-item">Aucune notification de test</div></div>
  </div>
  <div class="tab-page" data-page="pomodoro">
    <h2>Pomodoro</h2>
    <p>Minuteur local, sans réseau.</p>
    <div class="placeholder-list">
      <div class="placeholder-item pomodoro-status">Prêt · 25:00</div>
      <button class="expand-button pomodoro-toggle" type="button">Démarrer</button>
    </div>
  </div>
  <div class="tab-page" data-page="clipboard">
    <h2>Presse-papier</h2>
    <p>Lecture et écriture du presse-papier courant.</p>
    <textarea class="clipboard-text" rows="3" placeholder="Texte du presse-papier"></textarea>
    <div class="placeholder-list"><button class="expand-button clipboard-copy" type="button">Copier</button></div>
  </div>
`;
const mainHeader = Object.assign(document.createElement("div"), {
  className: "panel-header",
  innerHTML: `
    <div><div class="eyebrow">Island</div><h1 class="panel-title">Vue principale</h1></div>
    <button class="panel-close" type="button" aria-label="Réduire">−</button>
  `
});
const extendedHeader = Object.assign(document.createElement("div"), {
  className: "panel-header",
  innerHTML: `
    <div><div class="eyebrow">Island</div><h1 class="panel-title">Vue étendue</h1></div>
    <button class="panel-close" type="button" aria-label="Réduire">−</button>
  `
});

const mainView = document.createElement("div");
mainView.className = "main-view";
mainView.innerHTML = `
  <div class="main-content">
    <article class="module-card media-card">
      <div><div class="module-label">Lecture en cours</div><div class="media-meta">Aucune lecture</div><div class="media-artist">Données de test</div></div>
      <div class="media-controls" aria-label="Contrôles média">
        <button class="media-control" type="button" aria-label="Précédent">‹</button>
        <button class="media-control primary" type="button" aria-label="Lecture">▶</button>
        <button class="media-control" type="button" aria-label="Suivant">›</button>
      </div>
    </article>
    <article class="module-card sliders-card">
      <div><div class="module-row"><span class="module-label">Volume</span><span class="module-value">62 %</span></div><div class="fake-slider" aria-hidden="true"></div></div>
      <div><div class="module-row"><span class="module-label">Luminosité</span><span class="module-value">—</span></div><div class="fake-slider" aria-hidden="true"></div></div>
    </article>
  </div>
  <footer class="panel-footer"><span>Modules en préparation</span><button class="expand-button" type="button">Ouvrir les pages</button></footer>
`;
panelShell.replaceChildren(mainHeader, mainView, extendedHeader, extended);
panelShell.querySelectorAll(".panel-close").forEach((button) => button.addEventListener("click", closePanel));
panelShell.querySelectorAll(".expand-button").forEach((button) => button.addEventListener("click", () => setExtended(true)));

function setExtended(value: boolean) {
  panel.classList.toggle("extended", value);
  mainView.hidden = value;
  extended.hidden = !value;
  panelShell.querySelectorAll(".panel-header").forEach((header) => {
    header.classList.toggle("extended-header", value);
    header.toggleAttribute("hidden", value ? header === mainHeader : header === extendedHeader);
  });
  window.requestAnimationFrame(pushPanelRect);
}

function selectTab(tab: TabId) {
  panelShell.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((button) => {
    button.classList.toggle("active", button.dataset.tab === tab);
  });
  panelShell.querySelectorAll<HTMLElement>("[data-page]").forEach((page) => {
    page.classList.toggle("active", page.dataset.page === tab);
  });
}

extended.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((button) => {
  button.addEventListener("click", () => selectTab(button.dataset.tab as TabId));
});

const pomodoroStatusElement = extended.querySelector<HTMLElement>(".pomodoro-status");
const pomodoroToggle = extended.querySelector<HTMLButtonElement>(".pomodoro-toggle");
const clipboardText = extended.querySelector<HTMLTextAreaElement>(".clipboard-text");
const clipboardCopy = extended.querySelector<HTMLButtonElement>(".clipboard-copy");
if (pomodoroStatusElement && pomodoroToggle) {
  const renderPomodoro = (status: { running: boolean; remainingSeconds: number }) => {
    const minutes = Math.floor(status.remainingSeconds / 60).toString().padStart(2, "0");
    const seconds = (status.remainingSeconds % 60).toString().padStart(2, "0");
    pomodoroStatusElement.textContent = `${status.running ? "En cours" : "Prêt"} · ${minutes}:${seconds}`;
    pomodoroToggle.textContent = status.running ? "Arrêter" : "Démarrer";
  };
  pomodoroToggle.addEventListener("click", async () => {
    const status = pomodoroToggle.textContent === "Arrêter"
      ? await pomodoroStop()
      : await pomodoroStart();
    if (status) renderPomodoro(status);
  });
  if (isTauri) {
    void pomodoroStatus().then((status) => {
      if (status) renderPomodoro(status);
    });
    window.setInterval(() => {
      void pomodoroStatus().then((status) => {
        if (status) renderPomodoro(status);
      });
    }, 1000);
  }
}
if (clipboardText && clipboardCopy) {
  clipboardCopy.addEventListener("click", () => {
    void writeClipboard(clipboardText.value);
  });
  if (isTauri) {
    void readClipboard().then((text) => {
      if (text !== null) clipboardText.value = text;
    });
  }
}

setExtended(false);

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
  void installWakeListener(scheduleOpen);
  void installWakeLeaveListener(scheduleClose);
}

browserWake.addEventListener("pointerenter", scheduleOpen);
browserWake.addEventListener("pointerleave", scheduleClose);
window.addEventListener("pointermove", (event) => {
  const zone = event.clientX >= (window.innerWidth - config.panel.collapsedWidth) / 2
    && event.clientX <= (window.innerWidth + config.panel.collapsedWidth) / 2
    && event.clientY >= config.panel.offsetY
    && event.clientY <= config.panel.offsetY + config.panel.collapsedHeight;
  const overPanel = panel.matches(":hover");
  if (zone || overPanel) {
    scheduleOpen();
  } else if (!openTimer) {
    scheduleClose();
  }
});
if (isTauri) {
  void setZoomForDpr(window.devicePixelRatio);
  void setWakeVisible(true);
}

window.addEventListener("resize", pushPanelRect);
