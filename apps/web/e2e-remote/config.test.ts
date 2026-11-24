import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// The local suite never runs the remote config. These assertions keep the
// lab-gated config honest: the lab ingress is the only base URL, certificate
// verification is never disabled, and the gate prints `Unverified` off-lab.

const web = join(import.meta.dir, "..");
const config = readFileSync(join(web, "playwright.remote.config.ts"), "utf8");
const gate = readFileSync(join(web, "e2e-remote", "gate.ts"), "utf8");
const remote = readFileSync(join(web, "e2e-remote", "remote.e2e.ts"), "utf8");
const pkg = JSON.parse(readFileSync(join(web, "package.json"), "utf8")) as {
  scripts: Record<string, string>;
};

describe("remote Playwright config", () => {
  test("targets the lab ingress", () => {
    expect(config).toContain('baseURL: "https://quantrule.dev"');
    expect(config).toContain("QR_REMOTE_URL");
    expect(config).toContain("QR_REMOTE_BROWSER");
  });

  test("never disables certificate verification", () => {
    for (const source of [config, remote]) {
      expect(source).not.toContain("ignoreHTTPSErrors");
      expect(source).not.toContain("ignore-certificate-errors");
    }
  });

  test("has no local server and no route mocks", () => {
    expect(config).not.toContain("webServer");
    expect(remote).not.toContain("page.route");
    expect(remote).not.toContain("route.fulfill");
  });

  test("is lab-gated and prints Unverified off-lab", () => {
    expect(gate).toContain("QR_REMOTE_BROWSER");
    expect(gate).toContain("QR_REMOTE_URL");
    expect(gate).toContain("QR_REMOTE_TOKEN");
    expect(gate).toContain("Unverified");
    expect(pkg.scripts["test:remote"]).toContain("playwright.remote.config.ts");
    expect(pkg.scripts["test:remote"]).toContain("e2e-remote/gate.ts");
  });
});
