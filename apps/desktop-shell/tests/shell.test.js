import { describe, expect, it } from "bun:test";
import { readFileSync } from "node:fs";
import {
  canNavigateWithoutProbe,
  checkForUpdate,
  DEFAULT_GATEWAY_URL,
  healthUrl,
  isLocalGateway,
  isNewerVersion,
  launchPlan,
  normalizeGatewayUrl,
  parseVersion,
  probeGateway,
  releaseSummary,
  SHELL_VERSION,
} from "../ui/shell-lib.js";

const root = new URL("..", import.meta.url);

function read(name) {
  return readFileSync(new URL(name, root), "utf8");
}

function releaseBody(tag, overrides = {}) {
  return {
    tag_name: tag,
    html_url: `https://github.com/quant-rule/quant-rule/releases/tag/${tag}`,
    ...overrides,
  };
}

function responds(body, status = 200) {
  return async () => new Response(JSON.stringify(body), { status });
}

describe("gateway URL", () => {
  it("uses the local gateway on a first run", () => {
    expect(DEFAULT_GATEWAY_URL).toBe("http://localhost:8080");
    expect(normalizeGatewayUrl(DEFAULT_GATEWAY_URL)).toBe(DEFAULT_GATEWAY_URL);
  });

  it("normalizes a trailing slash and surrounding whitespace", () => {
    expect(normalizeGatewayUrl("  http://localhost:8080/  ")).toBe(
      "http://localhost:8080",
    );
    expect(normalizeGatewayUrl("https://quant.example.com/")).toBe(
      "https://quant.example.com",
    );
  });

  it("accepts an IPv6 loopback origin", () => {
    expect(normalizeGatewayUrl("http://[::1]:8080/")).toBe("http://[::1]:8080");
    expect(isLocalGateway("http://[::1]:8080")).toBe(true);
  });

  it("rejects a path, query, fragment, credentials, or another scheme", () => {
    expect(normalizeGatewayUrl("http://localhost:8080/api")).toBeNull();
    expect(normalizeGatewayUrl("http://localhost:8080/?q=1")).toBeNull();
    expect(normalizeGatewayUrl("http://localhost:8080/#top")).toBeNull();
    expect(normalizeGatewayUrl("http://user:pass@localhost:8080")).toBeNull();
    expect(normalizeGatewayUrl("ftp://localhost:8080")).toBeNull();
    expect(normalizeGatewayUrl("javascript:alert(1)")).toBeNull();
    expect(normalizeGatewayUrl("file:///etc/passwd")).toBeNull();
  });

  it("rejects an empty value and a value without a scheme", () => {
    expect(normalizeGatewayUrl("")).toBeNull();
    expect(normalizeGatewayUrl(null)).toBeNull();
    expect(normalizeGatewayUrl("localhost:8080")).toBeNull();
  });

  it("probes the root health route next to the API", () => {
    expect(healthUrl("http://localhost:8080")).toBe(
      "http://localhost:8080/health",
    );
  });

  it("tries a remote plain-HTTP gateway without a probe, but not a local one", () => {
    expect(canNavigateWithoutProbe("http://localhost:8080")).toBe(false);
    expect(canNavigateWithoutProbe("http://127.0.0.1:8080")).toBe(false);
    expect(canNavigateWithoutProbe("https://quant.example.com")).toBe(false);
    expect(canNavigateWithoutProbe("http://192.168.1.20:8080")).toBe(true);
    expect(isLocalGateway("quant.example.com")).toBe(false);
  });
});

describe("launch plan", () => {
  const gateway = "http://localhost:8080";

  it("opens a reachable gateway", () => {
    expect(
      launchPlan({ reachable: true, updateAvailable: false, gatewayUrl: gateway }),
    ).toEqual({ open: true, showUpdate: false, showOffline: false });
  });

  it("shows the bundled client when a local gateway is unreachable", () => {
    expect(
      launchPlan({ reachable: false, updateAvailable: false, gatewayUrl: gateway }),
    ).toEqual({ open: false, showUpdate: false, showOffline: true });
  });

  it("tries a remote plain-HTTP gateway even when the probe fails", () => {
    expect(
      launchPlan({
        reachable: false,
        updateAvailable: false,
        gatewayUrl: "http://192.168.1.20:8080",
      }),
    ).toEqual({ open: true, showUpdate: false, showOffline: false });
  });

  it("keeps a newer release visible instead of opening", () => {
    expect(
      launchPlan({ reachable: true, updateAvailable: true, gatewayUrl: gateway }),
    ).toEqual({ open: false, showUpdate: true, showOffline: false });
  });

  it("shows both the update and the fallback when the gateway is down", () => {
    expect(
      launchPlan({ reachable: false, updateAvailable: true, gatewayUrl: gateway }),
    ).toEqual({ open: false, showUpdate: true, showOffline: true });
  });
});

describe("release version", () => {
  it("parses a tagged version", () => {
    expect(parseVersion("v1.2.3")).toEqual({
      major: 1,
      minor: 2,
      patch: 3,
      prerelease: null,
    });
    expect(parseVersion("1.2.3-rc.1")?.prerelease).toBe("rc.1");
    expect(parseVersion("latest")).toBeNull();
  });

  it("compares the numeric core, and a release over its pre-release", () => {
    expect(isNewerVersion("0.3.0", "v0.3.0")).toBe(false);
    expect(isNewerVersion("0.3.0", "v0.3.1")).toBe(true);
    expect(isNewerVersion("0.3.0", "v1.0.0")).toBe(true);
    expect(isNewerVersion("0.3.1", "v0.3.0")).toBe(false);
    expect(isNewerVersion("1.0.0", "v1.0.0-rc.1")).toBe(false);
    expect(isNewerVersion("1.0.0-rc.1", "v1.0.0")).toBe(true);
    expect(isNewerVersion("0.3.0", "not-a-version")).toBe(false);
  });

  it("accepts only the project release page", () => {
    expect(releaseSummary(releaseBody("v1.0.0"))).toEqual({
      tag: "v1.0.0",
      url: "https://github.com/quant-rule/quant-rule/releases/tag/v1.0.0",
    });
    expect(
      releaseSummary(releaseBody("v1.0.0", { html_url: "http://example.com" })),
    ).toBeNull();
    expect(releaseSummary({ tag_name: "nope" })).toBeNull();
    expect(releaseSummary(null)).toBeNull();
  });
});

describe("gateway probe", () => {
  it("answers with no-cors so the gateway needs no CORS header", async () => {
    let seen;
    const fake = async (url, init) => {
      seen = { url, init };
      return new Response(null, { status: 200 });
    };

    expect(await probeGateway(fake, "http://localhost:8080", 50)).toBe(true);
    expect(seen.url).toBe("http://localhost:8080/health");
    expect(seen.init.mode).toBe("no-cors");
    expect(seen.init.credentials).toBe("omit");
    expect(seen.init.signal).toBeDefined();
  });

  it("treats a rejected request as unreachable", async () => {
    const fake = async () => {
      throw new TypeError("network");
    };
    expect(await probeGateway(fake, "http://localhost:8080", 50)).toBe(false);
  });

  it("stops waiting at the timeout", async () => {
    const hanging = (_url, init) =>
      new Promise((_resolve, reject) => {
        init.signal.addEventListener("abort", () => reject(new Error("aborted")));
      });
    expect(await probeGateway(hanging, "http://localhost:8080", 10)).toBe(false);
  });
});

describe("update check", () => {
  it("reports a newer release", async () => {
    let seen;
    const fake = async (url, init) => {
      seen = { url, init };
      return new Response(JSON.stringify(releaseBody("v1.0.0")), { status: 200 });
    };

    expect(await checkForUpdate(fake, "0.3.0", 50)).toEqual({
      status: "available",
      tag: "v1.0.0",
      url: "https://github.com/quant-rule/quant-rule/releases/tag/v1.0.0",
    });
    expect(seen.url).toBe(
      "https://api.github.com/repos/quant-rule/quant-rule/releases/latest",
    );
    expect(seen.init.headers.Accept).toBe("application/vnd.github+json");
  });

  it("reports the running release", async () => {
    expect(await checkForUpdate(responds(releaseBody("v0.3.0")), "0.3.0", 50)).toEqual({
      status: "current",
      tag: "v0.3.0",
      url: "https://github.com/quant-rule/quant-rule/releases/tag/v0.3.0",
    });
  });

  it("fails softly on an HTTP error, a bad payload, or a network error", async () => {
    expect(await checkForUpdate(responds({}, 404), "0.3.0", 50)).toEqual({
      status: "failed",
      reason: "HTTP 404",
    });
    expect(
      await checkForUpdate(responds({ tag_name: "v1.0.0" }), "0.3.0", 50),
    ).toEqual({ status: "failed", reason: "unexpected release payload" });
    expect(
      await checkForUpdate(async () => {
        throw new TypeError("offline");
      }, "0.3.0", 50),
    ).toEqual({ status: "failed", reason: "offline" });
  });
});

describe("shell boundary", () => {
  it("keeps one version across the manifests and the bootstrap", () => {
    const tauri = JSON.parse(read("tauri.conf.json"));
    const pkg = JSON.parse(read("package.json"));
    const cargo = /\[package\][\s\S]*?\nversion = "([^"]+)"/.exec(read("Cargo.toml"));

    expect(SHELL_VERSION).toBe("0.3.0");
    expect(tauri.version).toBe(SHELL_VERSION);
    expect(pkg.version).toBe(SHELL_VERSION);
    expect(cargo?.[1]).toBe(SHELL_VERSION);
  });

  it("loads the bootstrap page and no IPC capability", () => {
    const tauri = JSON.parse(read("tauri.conf.json"));
    expect(tauri.app.windows).toHaveLength(1);
    expect(tauri.app.windows[0].url).toBe("shell.html");
    expect(tauri.build.frontendDist).toBe("dist");
    expect(tauri.app.security.csp).toContain("connect-src");
    expect(tauri.plugins).toBeUndefined();
    expect(tauri.app.security.capabilities).toBeUndefined();
  });

  it("declares no IPC command, local file read, or engine import", () => {
    const rust = read("src/main.rs");
    const ui = `${read("ui/shell.js")}\n${read("ui/shell.html")}`;
    const cargo = read("Cargo.toml");

    expect(rust).not.toMatch(
      /tauri::command|generate_handler!|tauri::ipc|std::fs|std::path::Path|File::open|read_to_string/,
    );
    expect(ui).not.toMatch(/invoke\(|__TAURI__|__TAURI_INTERNALS__/);
    expect(cargo).not.toMatch(
      /quantrule-(data|model|backtest|app|api|server|shell|simulation|research)\b/,
    );
    expect(cargo).not.toMatch(/tauri[-_]plugin/);
    expect(cargo).not.toMatch(/path\s*=\s*"\.\./);
  });
});
