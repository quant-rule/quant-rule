/**
 * The desktop shell bootstrap.
 *
 * The page is bundled with the app. It remembers one setting, the gateway URL,
 * and it checks GitHub Releases for a newer shell. It owns no data: when the
 * gateway answers, the same window navigates to it and the built Svelte client
 * is the UI. The bundled client is shown in an iframe only when the gateway is
 * unreachable, and in that state it can only report that it is disconnected.
 *
 * The active profile is not stored here. The web client already persists
 * `quantrule.profile` in its own storage and sends `X-QuantRule-Profile` on
 * every call; the shell leaves that mechanism alone.
 */

import {
  checkForUpdate,
  DEFAULT_GATEWAY_URL,
  GATEWAY_STORAGE_KEY,
  launchPlan,
  normalizeGatewayUrl,
  probeGateway,
  SHELL_VERSION,
  UPDATE_TIMEOUT_MS,
} from "./shell-lib.js";

const elements = {
  form: document.getElementById("gateway-form"),
  gateway: document.getElementById("gateway"),
  error: document.getElementById("gateway-error"),
  retry: document.getElementById("retry"),
  useDefault: document.getElementById("default"),
  status: document.getElementById("status"),
  updateNote: document.getElementById("update-note"),
  version: document.getElementById("version"),
  update: document.getElementById("update"),
  updateText: document.getElementById("update-text"),
  updateLink: document.getElementById("update-link"),
  continueToGateway: document.getElementById("continue"),
  offline: document.getElementById("offline"),
  fallback: document.getElementById("fallback"),
};

let gateway = DEFAULT_GATEWAY_URL;
let opening = false;
let run = 0;

function storedGateway() {
  try {
    const value = localStorage.getItem(GATEWAY_STORAGE_KEY);
    return value === null ? null : normalizeGatewayUrl(value);
  } catch {
    // A restricted webview may refuse storage; the default still works.
    return null;
  }
}

function rememberGateway(value) {
  try {
    localStorage.setItem(GATEWAY_STORAGE_KEY, value);
  } catch {
    // Persistence is a convenience, not a requirement for this session.
  }
}

function setStatus(text, tone) {
  elements.status.textContent = text;
  elements.status.dataset.tone = tone;
}

function showGatewayError(text) {
  elements.error.textContent = text;
  elements.error.hidden = false;
}

function hideGatewayError() {
  elements.error.hidden = true;
  elements.error.textContent = "";
}

function showUpdateNote(text) {
  elements.updateNote.textContent = text;
  elements.updateNote.hidden = false;
}

function hideUpdateNote() {
  elements.updateNote.hidden = true;
  elements.updateNote.textContent = "";
}

function hideOffline() {
  elements.offline.hidden = true;
}

function showOffline(candidate) {
  setStatus(`Gateway unreachable at ${candidate}. Showing the bundled client.`, "bad");
  if (elements.fallback.getAttribute("src") !== "index.html") {
    elements.fallback.setAttribute("src", "index.html");
  }
  elements.offline.hidden = false;
  elements.gateway.focus();
  elements.gateway.select();
}

function showUpdate(update) {
  elements.update.hidden = false;
  elements.updateText.textContent =
    `Version ${update.tag} is available. This shell is v${SHELL_VERSION}.`;
  elements.updateLink.href = update.url;
  elements.updateLink.textContent = update.url;
  elements.updateLink.hidden = false;
}

function hideUpdate() {
  elements.update.hidden = true;
  elements.updateLink.hidden = true;
  elements.updateLink.removeAttribute("href");
}

function openGateway() {
  if (opening) {
    return;
  }
  opening = true;
  rememberGateway(gateway);
  setStatus(`Gateway reachable at ${gateway}. Opening…`, "ok");
  window.location.replace(gateway);
}

function connect() {
  const candidate = normalizeGatewayUrl(elements.gateway.value);
  if (candidate === null) {
    showGatewayError(
      "Enter an http:// or https:// origin, with no path, query, or fragment.",
    );
    setStatus("The gateway URL is not valid.", "bad");
    return;
  }
  gateway = candidate;
  elements.gateway.value = candidate;
  hideGatewayError();
  rememberGateway(candidate);
  opening = true;
  setStatus(`Opening ${candidate}…`, "busy");
  window.location.replace(candidate);
}

async function evaluate() {
  const token = ++run;
  const candidate = normalizeGatewayUrl(elements.gateway.value);
  if (candidate === null) {
    showGatewayError(
      "Enter an http:// or https:// origin, with no path, query, or fragment.",
    );
    setStatus("The gateway URL is not valid.", "bad");
    return;
  }

  gateway = candidate;
  elements.gateway.value = candidate;
  rememberGateway(candidate);
  hideGatewayError();
  hideUpdate();
  hideUpdateNote();
  hideOffline();
  setStatus(`Checking ${candidate}…`, "busy");

  const [reachable, update] = await Promise.all([
    probeGateway(window.fetch.bind(window), candidate),
    checkForUpdate(window.fetch.bind(window), SHELL_VERSION, UPDATE_TIMEOUT_MS),
  ]);

  // An explicit Open, or a newer retry, wins over this result.
  if (token !== run || opening) {
    return;
  }

  const updateAvailable = update.status === "available";
  const plan = launchPlan({
    reachable,
    updateAvailable,
    gatewayUrl: candidate,
  });

  if (plan.showUpdate) {
    showUpdate(update);
  } else if (update.status === "failed") {
    showUpdateNote("Update check unavailable.");
  } else {
    showUpdateNote(`Shell is up to date (${update.tag}).`);
  }

  if (plan.showOffline) {
    showOffline(candidate);
  }

  if (!plan.open) {
    if (updateAvailable) {
      setStatus(
        `Gateway reachable at ${candidate}. A newer release is available.`,
        "ok",
      );
    }
    return;
  }

  openGateway();
}

function useDefault() {
  elements.gateway.value = DEFAULT_GATEWAY_URL;
  hideGatewayError();
  elements.gateway.focus();
  elements.gateway.select();
}

function boot() {
  elements.version.textContent = `v${SHELL_VERSION}`;
  gateway = storedGateway() ?? DEFAULT_GATEWAY_URL;
  elements.gateway.value = gateway;

  elements.form.addEventListener("submit", (event) => {
    event.preventDefault();
    connect();
  });
  elements.retry.addEventListener("click", () => {
    void evaluate();
  });
  elements.useDefault.addEventListener("click", useDefault);
  elements.continueToGateway.addEventListener("click", connect);

  void evaluate();
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", boot, { once: true });
} else {
  boot();
}
