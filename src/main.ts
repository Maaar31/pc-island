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
  writeClipboard,
  clipboardHistory,
  mediaStatus,
  mediaControl,
  type MediaSnapshot,
  systemVolume,
  setSystemVolume,
  systemMuted,
  setSystemMuted,
  radioStatus,
  setRadio,
  type RadioSnapshot
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
if (!shell || !closeButton) throw new Error("Island controls are missing");
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
    <textarea class="clipboard-text" rows="3" placeholder="Contenu texte du presse-papier"></textarea>
    <div class="clipboard-actions">
      <button class="expand-button clipboard-read" type="button">Lire</button>
      <button class="expand-button clipboard-copy" type="button">Copier vers Windows</button>
    </div>
    <p class="clipboard-status" role="status">Le contenu actuel sera lu uniquement à votre demande.</p>
    <input class="clipboard-search" type="search" placeholder="Rechercher dans l’historique" aria-label="Rechercher dans l'historique du presse-papier">
    <div class="clipboard-history" aria-label="Historique local"></div>
  </div>
`;
const mainHeader = Object.assign(document.createElement("div"), {
  className: "panel-header",
  innerHTML: `
    <div class="brand-lockup"><div class="mascot-mini" aria-hidden="true"><span class="mascot-body"><i></i><i></i></span></div><div><div class="eyebrow">Island</div><h1 class="panel-title">Accueil</h1></div></div>
    <div class="header-right"><nav class="page-tabs" aria-label="Pages">
      <button class="page-tab active" data-page-target="overview" type="button" aria-label="Accueil">⌂</button>
      <button class="page-tab" data-page-target="notifications" type="button" aria-label="Alertes">●</button>
      <button class="page-tab" data-page-target="pomodoro" type="button" aria-label="Pomodoro">◷</button>
      <button class="page-tab" data-page-target="clipboard" type="button" aria-label="Presse-papier">▣</button>
    </nav><button class="panel-close" type="button" aria-label="Réduire">−</button></div>
  `
});
const extendedHeader = Object.assign(document.createElement("div"), {
  className: "panel-header",
  innerHTML: `
    <div class="brand-lockup"><button class="panel-back" type="button" aria-label="Retour à la vue principale">‹</button><div><div class="eyebrow">Island</div><h1 class="panel-title extended-title">Pages</h1></div></div>
    <button class="panel-close" type="button" aria-label="Réduire">−</button>
  `
});

const mainView = document.createElement("div");
mainView.className = "main-view";
mainView.innerHTML = `
  <div class="main-content">
    <article class="module-card media-card">
      <div class="welcome-row">
        <div class="mascot-stage" aria-hidden="true">
          <span class="mascot-figure"><span class="mascot-antenna"></span><span class="mascot-arm left"></span><span class="mascot-arm right"></span><span class="mascot-body"><i></i><i></i></span><span class="mascot-foot left"></span><span class="mascot-foot right"></span></span>
        </div>
        <div><div class="module-label">Bonjour</div><div class="media-meta">Votre île est prête</div><div class="media-artist">Survolez, puis choisissez une action</div></div>
      </div>
      <div class="now-playing"><div class="module-label">Lecture en cours</div><div class="media-meta">Aucune lecture</div><div class="media-artist">Aucun lecteur actif</div></div>
      <div class="media-controls" aria-label="Contrôles média">
        <button class="media-control" type="button" aria-label="Précédent">‹</button>
        <button class="media-control primary" type="button" aria-label="Lecture">▶</button>
        <button class="media-control" type="button" aria-label="Suivant">›</button>
      </div>
    </article>
    <article class="module-card sliders-card">
      <div>
        <div class="module-row"><label class="module-label" for="volume-slider">Volume</label><span class="module-value volume-value">—</span><button class="mute-button" type="button" aria-label="Couper le son">🔊</button></div>
        <input id="volume-slider" class="system-slider" type="range" min="0" max="100" value="0" aria-label="Volume système">
      </div>
      <div>
        <div class="module-row"><span class="module-label">Connexions</span></div>
      </div>
      <div class="radio-row">
        <button class="radio-toggle" data-radio="wifi" type="button"><span>Wi-Fi</span><b>—</b></button>
        <button class="radio-toggle" data-radio="bluetooth" type="button"><span>Bluetooth</span><b>—</b></button>
      </div>
    </article>
  </div>
  <footer class="panel-footer"><span>Une île discrète, toujours disponible</span><span class="footer-hint">Cliquez sur une page</span></footer>
`;
panelShell.replaceChildren(mainHeader, mainView, extendedHeader, extended);
panelShell.querySelectorAll(".panel-close").forEach((button) => button.addEventListener("click", closePanel));
panelShell.querySelectorAll<HTMLButtonElement>("[data-page-target]").forEach((button) => {
  button.addEventListener("click", () => setPage(button.dataset.pageTarget as PageId));
});
extendedHeader.querySelector(".panel-back")?.addEventListener("click", () => setPage("overview"));

const mediaTitle = mainView.querySelector<HTMLElement>(".now-playing .media-meta");
const mediaArtist = mainView.querySelector<HTMLElement>(".now-playing .media-artist");
const mediaToggle = mainView.querySelector<HTMLButtonElement>(".media-control.primary");
const mediaPrevious = mainView.querySelector<HTMLButtonElement>(".media-control[aria-label='Précédent']");
const mediaNext = mainView.querySelector<HTMLButtonElement>(".media-control[aria-label='Suivant']");
const volumeSlider = mainView.querySelector<HTMLInputElement>("#volume-slider");
const volumeValue = mainView.querySelector<HTMLElement>(".volume-value");
const muteButton = mainView.querySelector<HTMLButtonElement>(".mute-button");
let muted = false;
let mediaPoll: number | undefined;
let volumePoll: number | undefined;
let pomodoroPoll: number | undefined;
let refreshPomodoro: (() => void) | undefined;
function renderMedia(media: MediaSnapshot) {
  if (!mediaTitle || !mediaArtist || !mediaToggle) return;
  mediaTitle.textContent = media.available ? media.title : "Aucune lecture";
  mediaArtist.textContent = media.available ? (media.artist || "Artiste inconnu") : "Aucun lecteur actif";
  mediaToggle.textContent = media.playing ? "Ⅱ" : "▶";
  mediaToggle.setAttribute("aria-label", media.playing ? "Pause" : "Lecture");
}
function refreshMedia() {
  if (!isTauri) return;
  void mediaStatus().then((media) => {
    if (media) renderMedia(media);
  });
}
if (mediaToggle && mediaPrevious && mediaNext) {
  mediaToggle.addEventListener("click", () => {
    void mediaControl("toggle").then((media) => {
      if (media) renderMedia(media);
    });
  });
  mediaPrevious.addEventListener("click", () => {
    void mediaControl("previous").then((media) => {
      if (media) renderMedia(media);
    });
  });
  mediaNext.addEventListener("click", () => {
    void mediaControl("next").then((media) => {
      if (media) renderMedia(media);
    });
  });
}
if (volumeSlider && volumeValue) {
  const renderMute = (value: boolean) => {
    muted = value;
    if (!muteButton) return;
    muteButton.textContent = muted ? "🔇" : "🔊";
    muteButton.setAttribute("aria-label", muted ? "Réactiver le son" : "Couper le son");
    muteButton.classList.toggle("muted", muted);
  };
  muteButton?.addEventListener("click", () => {
    void setSystemMuted(!muted).then((value) => {
      if (value !== null) renderMute(value);
    });
  });
  const renderVolume = (value: number) => {
    const normalized = Math.round(Math.max(0, Math.min(100, value)));
    volumeSlider.value = String(normalized);
    volumeValue.textContent = `${normalized} %`;
  };
  volumeSlider.addEventListener("input", () => {
    void setSystemVolume(Number(volumeSlider.value)).then((value) => {
      if (value !== null) renderVolume(value);
      else volumeValue.textContent = "Erreur";
    });
  });
}
const radioButtons = mainView.querySelectorAll<HTMLButtonElement>(".radio-toggle");
function renderRadios(snapshot: RadioSnapshot) {
  radioButtons.forEach((button) => {
    const kind = button.dataset.radio as "wifi" | "bluetooth";
    const value = snapshot[kind];
    button.classList.toggle("on", value === true);
    button.classList.toggle("unavailable", value === null);
    const state = button.querySelector("b");
    if (state) state.textContent = value === null ? "Indisponible" : value ? "Activé" : "Désactivé";
  });
}
if (radioButtons.length > 0 && isTauri) {
  void radioStatus().then((snapshot) => {
    if (snapshot) renderRadios(snapshot);
  });
  radioButtons.forEach((button) => {
    button.addEventListener("click", () => {
      const kind = button.dataset.radio as "wifi" | "bluetooth";
      const current = button.classList.contains("on");
      void setRadio(kind, !current).then((snapshot) => {
        if (snapshot) renderRadios(snapshot);
      });
    });
  });
}

type PageId = "overview" | TabId;
let currentPage: PageId = "overview";

function setPage(page: PageId) {
  currentPage = page;
  setExtended(page !== "overview");
  panelShell.querySelectorAll<HTMLButtonElement>("[data-page-target]").forEach((button) => {
    button.classList.toggle("active", button.dataset.pageTarget === page);
  });
  if (page !== "overview") {
    panelShell.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((button) => {
      button.classList.toggle("active", button.dataset.tab === page);
    });
    panelShell.querySelectorAll<HTMLElement>("[data-page]").forEach((view) => {
      view.classList.toggle("active", view.dataset.page === page);
    });
  }
  const title = extendedHeader.querySelector<HTMLElement>(".extended-title");
  if (title && page !== "overview") {
    title.textContent = page === "pomodoro" ? "Pomodoro" : page === "clipboard" ? "Presse-papier" : "Alertes";
  }
}

function setExtended(value: boolean) {
  panel.classList.toggle("extended", value);
  mainView.hidden = value;
  extended.hidden = !value;
  mainView.setAttribute("aria-hidden", String(value));
  extended.setAttribute("aria-hidden", String(!value));
  if (value) {
    const active = extended.querySelector<HTMLElement>(".tab-page.active");
    const title = extendedHeader.querySelector<HTMLElement>(".extended-title");
    if (title && active) title.textContent = active.dataset.page === "pomodoro"
      ? "Pomodoro"
      : active.dataset.page === "clipboard"
        ? "Presse-papier"
        : "Alertes";
  }
  panelShell.querySelectorAll(".panel-header").forEach((header) => {
    header.classList.toggle("extended-header", value);
    header.toggleAttribute("hidden", value ? header === mainHeader : header === extendedHeader);
  });
  window.requestAnimationFrame(pushPanelRect);
}

function selectTab(tab: TabId) {
  setPage(tab);
  panelShell.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((button) => {
    button.classList.toggle("active", button.dataset.tab === tab);
  });
  panelShell.querySelectorAll<HTMLElement>("[data-page]").forEach((page) => {
    page.classList.toggle("active", page.dataset.page === tab);
  });
  const title = extendedHeader.querySelector<HTMLElement>(".extended-title");
  if (title) title.textContent = tab === "pomodoro" ? "Pomodoro" : tab === "clipboard" ? "Presse-papier" : "Alertes";
  window.requestAnimationFrame(pushPanelRect);
}

extended.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((button) => {
  button.addEventListener("click", () => selectTab(button.dataset.tab as TabId));
});

const pomodoroStatusElement = extended.querySelector<HTMLElement>(".pomodoro-status");
const pomodoroToggle = extended.querySelector<HTMLButtonElement>(".pomodoro-toggle");
const clipboardText = extended.querySelector<HTMLTextAreaElement>(".clipboard-text");
const clipboardRead = extended.querySelector<HTMLButtonElement>(".clipboard-read");
const clipboardCopy = extended.querySelector<HTMLButtonElement>(".clipboard-copy");
const clipboardHistoryList = extended.querySelector<HTMLElement>(".clipboard-history");
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
  refreshPomodoro = () => {
    void pomodoroStatus().then((status) => {
      if (status) renderPomodoro(status);
    });
  };
}
if (clipboardText && clipboardCopy) {
  const renderClipboardHistory = (items: string[]) => {
    if (!clipboardHistoryList) return;
    clipboardHistoryList.replaceChildren(...items.map((item) => {
      const button = document.createElement("button");
      button.className = "clipboard-history-item";
      button.type = "button";
      button.textContent = item.replace(/\s+/g, " ").slice(0, 90);
      button.title = item;
      button.addEventListener("click", () => {
        clipboardText.value = item;
      });
      return button;
    }));
  };
  const clipboardSearch = extended.querySelector<HTMLInputElement>(".clipboard-search");
  let historyItems: string[] = [];
  const renderFilteredHistory = () => {
    const query = clipboardSearch?.value.trim().toLocaleLowerCase() ?? "";
    renderClipboardHistory(historyItems.filter((item) => item.toLocaleLowerCase().includes(query)));
  };
  clipboardSearch?.addEventListener("input", renderFilteredHistory);
  void clipboardHistory().then((items) => {
    if (items) {
      historyItems = items;
      renderFilteredHistory();
    }
  });
  clipboardCopy.addEventListener("click", () => {
    void writeClipboard(clipboardText.value).then((result) => {
      const status = extended.querySelector<HTMLElement>(".clipboard-status");
      if (status) status.textContent = result === null ? "Impossible d'écrire dans le presse-papier." : "Copié dans le presse-papier Windows.";
      if (result !== null) void clipboardHistory().then((items) => {
        if (items) {
          historyItems = items;
          renderFilteredHistory();
        }
      });
    });
  });
  clipboardRead?.addEventListener("click", () => {
    void readClipboard().then((text) => {
      const status = extended.querySelector<HTMLElement>(".clipboard-status");
      if (text !== null) {
        clipboardText.value = text;
        if (status) status.textContent = text ? "Contenu lu depuis le presse-papier Windows." : "Le presse-papier texte est vide.";
      } else if (status) {
        status.textContent = "Impossible de lire le presse-papier.";
      }
    });
  });
}

setExtended(false);

let openTimer: number | undefined;
let closeTimer: number | undefined;
let open = false;
let regionFrame: number | undefined;

function stopModulePolling() {
  if (mediaPoll !== undefined) window.clearInterval(mediaPoll);
  if (volumePoll !== undefined) window.clearInterval(volumePoll);
  if (pomodoroPoll !== undefined) window.clearInterval(pomodoroPoll);
  mediaPoll = undefined;
  volumePoll = undefined;
  pomodoroPoll = undefined;
}

function startModulePolling() {
  if (!isTauri) return;
  stopModulePolling();
  refreshMedia();
  refreshPomodoro?.();
  void systemVolume().then((value) => {
    if (value !== null && volumeSlider && volumeValue) {
      volumeSlider.value = String(value);
      volumeValue.textContent = `${value} %`;
    }
  });
  mediaPoll = window.setInterval(refreshMedia, 2000);
  volumePoll = window.setInterval(() => {
    void systemVolume().then((value) => {
      if (value !== null && volumeSlider && volumeValue) {
        volumeSlider.value = String(value);
        volumeValue.textContent = `${value} %`;
      }
    });
  }, 2000);
  if (refreshPomodoro) pomodoroPoll = window.setInterval(refreshPomodoro, 1000);
}

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
  startModulePolling();
  stopRegionAnimation();
  regionFrame = window.requestAnimationFrame(animatePanelRegion);
}

function closePanel() {
  clearTimer(openTimer);
  openTimer = undefined;
  if (!open) return;
  open = false;
  stopModulePolling();
  setPage("overview");
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
