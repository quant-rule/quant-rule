/**
 * Pure helpers for the desktop shell bootstrap.
 *
 * The shell has no IPC command and no local data store. The bootstrap page is
 * the only place that reads or writes anything: one localStorage key for the
 * gateway URL. Everything here is a pure function so it can be tested without a
 * window, a gateway, or a network.
 */

/** The version of this shell. Keep in step with `tauri.conf.json`, `package.json`, and `Cargo.toml`. */
export const SHELL_VERSION = "0.3.0";

/** The gateway the shell opens on a first run. */
export const DEFAULT_GATEWAY_URL = "http://localhost:8080";

/** The one persisted setting. The active profile is persisted by the web client. */
export const GATEWAY_STORAGE_KEY = "quantrule.shell.gatewayUrl";

/** The GitHub API endpoint for the newest published release. */
export const RELEASE_API_URL =
  "https://api.github.com/repos/quant-rule/quant-rule/releases/latest";

/** How long a gateway probe may take before the shell treats it as unreachable. */
export const HEALTH_TIMEOUT_MS = 2500;

/** How long the release check may take before the shell continues without it. */
export const UPDATE_TIMEOUT_MS = 4000;

/**
 * Normalize a gateway setting to an origin.
 *
 * A gateway is an origin, not a path: the web client fetches `/api/v1` at the
 * root and the gateway serves its assets there. Rejecting a path, query,
 * fragment, or credentials avoids persisting a setting that cannot work and
 * keeps a `javascript:` or `file:` URL out of the persisted value.
 *
 * @param {unknown} value
 * @returns {string | null} the normalized origin, or null when invalid
 */
export function normalizeGatewayUrl(value) {
  const text = String(value ?? "").trim();
  if (text === "") {
    return null;
  }
  let url;
  try {
    url = new URL(text);
  } catch {
    return null;
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    return null;
  }
  if (url.username !== "" || url.password !== "") {
    return null;
  }
  if (url.pathname !== "" && url.pathname !== "/") {
    return null;
  }
  if (url.search !== "" || url.hash !== "") {
    return null;
  }
  return url.origin;
}

/** The reachability probe for a gateway. It answers at the root, next to the API. */
export function healthUrl(gatewayUrl) {
  return `${gatewayUrl}/health`;
}

/**
 * Whether a gateway is on the machine.
 *
 * A plain-HTTP gateway on the LAN is blocked as mixed content by the secure
 * shell origin, so a failed probe there is not proof that the gateway is down.
 * A top-level navigation is still allowed; the caller uses this to try it.
 */
export function isLocalGateway(gatewayUrl) {
  let host;
  try {
    host = new URL(gatewayUrl).hostname;
  } catch {
    return false;
  }
  return (
    host === "localhost" ||
    host.endsWith(".localhost") ||
    host === "127.0.0.1" ||
    host === "::1" ||
    host === "[::1]"
  );
}

/**
 * Whether a failed probe still justifies a navigation attempt.
 *
 * Only plain HTTP away from the machine: HTTPS probes are not mixed content,
 * and a local HTTP probe is trustworthy, so a failure there is a real failure.
 */
export function canNavigateWithoutProbe(gatewayUrl) {
  return gatewayUrl.startsWith("http://") && !isLocalGateway(gatewayUrl);
}

/**
 * Decide what a launch does with a probe and an update result.
 *
 * A newer release keeps the bootstrap page visible so the human can read it;
 * the gateway is still reachable and "Continue" opens it. The offline fallback
 * and the update card can both be visible, because an unreachable gateway is
 * exactly when the launcher is the whole UI.
 *
 * @param {{reachable: boolean, updateAvailable: boolean, gatewayUrl: string}} state
 * @returns {{open: boolean, showUpdate: boolean, showOffline: boolean}}
 */
export function launchPlan({ reachable, updateAvailable, gatewayUrl }) {
  const tryWithoutProbe = canNavigateWithoutProbe(gatewayUrl);
  return {
    open: !updateAvailable && (reachable || tryWithoutProbe),
    showUpdate: updateAvailable,
    showOffline: !reachable && !tryWithoutProbe,
  };
}

/**
 * Parse a release tag into its numeric parts.
 *
 * @param {unknown} tag
 * @returns {{major: number, minor: number, patch: number, prerelease: string | null} | null}
 */
export function parseVersion(tag) {
  const match = /^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/.exec(
    String(tag ?? "").trim(),
  );
  if (match === null) {
    return null;
  }
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4] ?? null,
  };
}

/**
 * Whether a release tag is newer than the running shell.
 *
 * A release is newer when its numeric core is greater. On an equal core, the
 * absence of a pre-release suffix wins; ordering between two pre-releases is
 * not needed to answer "is there a newer release".
 */
export function isNewerVersion(currentVersion, candidateTag) {
  const current = parseVersion(currentVersion);
  const candidate = parseVersion(candidateTag);
  if (current === null || candidate === null) {
    return false;
  }
  if (candidate.major !== current.major) {
    return candidate.major > current.major;
  }
  if (candidate.minor !== current.minor) {
    return candidate.minor > current.minor;
  }
  if (candidate.patch !== current.patch) {
    return candidate.patch > current.patch;
  }
  return current.prerelease !== null && candidate.prerelease === null;
}

/**
 * Validate the release payload the update check consumes.
 *
 * Only the release page under the project repository is accepted. The URL is
 * displayed, never fetched, but an allowlist keeps a surprising payload from
 * placing an arbitrary link in the shell.
 *
 * @param {unknown} payload
 * @returns {{tag: string, url: string} | null}
 */
export function releaseSummary(payload) {
  if (payload === null || typeof payload !== "object") {
    return null;
  }
  const tag = payload.tag_name;
  const url = payload.html_url;
  if (typeof tag !== "string" || parseVersion(tag) === null) {
    return null;
  }
  if (
    typeof url !== "string" ||
    !url.startsWith("https://github.com/quant-rule/quant-rule/releases/")
  ) {
    return null;
  }
  return { tag, url };
}

/**
 * Probe the gateway without reading a result.
 *
 * `no-cors` makes the response opaque, which is enough: a resolved request says
 * the gateway answered, and a rejected request says it did not. The gateway is
 * not required to send CORS headers, so a normal request would be blocked by
 * the shell origin and would report every gateway as down.
 *
 * @param {typeof fetch} fetchImpl
 * @param {string} gatewayUrl
 * @param {number} timeoutMs
 * @returns {Promise<boolean>}
 */
export async function probeGateway(fetchImpl, gatewayUrl, timeoutMs = HEALTH_TIMEOUT_MS) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    await fetchImpl(healthUrl(gatewayUrl), {
      method: "GET",
      mode: "no-cors",
      credentials: "omit",
      cache: "no-store",
      signal: controller.signal,
    });
    return true;
  } catch {
    return false;
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Ask GitHub Releases for the newest published release.
 *
 * A failed check is not fatal: the shell opens the gateway regardless and the
 * caller can surface "update check unavailable". No signature and no auto
 * install: the artifact is unsigned, so the human downloads the release.
 *
 * @param {typeof fetch} fetchImpl
 * @param {string} currentVersion
 * @param {number} timeoutMs
 * @returns {Promise<{status: "available" | "current", tag: string, url: string} | {status: "failed", reason: string}>}
 */
export async function checkForUpdate(
  fetchImpl,
  currentVersion,
  timeoutMs = UPDATE_TIMEOUT_MS,
) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetchImpl(RELEASE_API_URL, {
      method: "GET",
      headers: { Accept: "application/vnd.github+json" },
      cache: "no-store",
      credentials: "omit",
      signal: controller.signal,
    });
    if (!response.ok) {
      return { status: "failed", reason: `HTTP ${response.status}` };
    }
    const release = releaseSummary(await response.json());
    if (release === null) {
      return { status: "failed", reason: "unexpected release payload" };
    }
    return isNewerVersion(currentVersion, release.tag)
      ? { status: "available", ...release }
      : { status: "current", ...release };
  } catch (error) {
    return {
      status: "failed",
      reason: error instanceof Error ? error.message : String(error),
    };
  } finally {
    clearTimeout(timer);
  }
}
