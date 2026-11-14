import type { Page, Route } from "@playwright/test";

/** The API contract the client expects; keep in step with connection.ts. */
export const API = "quantrule.api/v1";
export const DIGEST = "sha256:" + "a".repeat(64);

export interface VersionFixture {
  product: string;
  api: string;
  image_digest: string | null;
  contracts: [string, string][];
}

export interface ServiceFixture {
  name: string;
  state: string;
  detail: string | null;
}

export interface HealthFixture {
  v: string;
  state: "healthy" | "degraded" | "down";
  services: ServiceFixture[];
  queue_depth: number;
  workers: number;
  image_digest: string | null;
}

export interface Fixture {
  version?: Partial<VersionFixture>;
  health?: Partial<HealthFixture>;
  profiles?: unknown[];
  /** Abort every API call, so the client sees an unreachable gateway. */
  unreachable?: boolean;
  /** Handle any endpoint the fixture does not know; return true when handled. */
  routes?: (path: string, route: Route) => Promise<boolean> | boolean;
}

/**
 * Install the HTTP fixture router.
 *
 * One place owns the shell endpoints (`/version`, `/health`, `/profiles`) so a
 * Playwright test declares only the endpoint it exercises. An unknown endpoint
 * is a 404 with a stable test code rather than a silent empty body.
 */
export async function installFixture(page: Page, fixture: Fixture = {}): Promise<void> {
  await page.route("**/api/v1/**", async (route) => {
    const path = new URL(route.request().url()).pathname.replace("/api/v1", "");
    if (fixture.unreachable === true) {
      await route.abort();
      return;
    }
    if (path === "/version") {
      await route.fulfill({
        json: {
          product: "1.0.0",
          api: API,
          image_digest: DIGEST,
          contracts: [],
          ...fixture.version,
        },
      });
      return;
    }
    if (path === "/health") {
      await route.fulfill({
        json: {
          v: API,
          state: "healthy",
          services: [],
          queue_depth: 0,
          workers: 1,
          image_digest: DIGEST,
          ...fixture.health,
        },
      });
      return;
    }
    if (path === "/profiles") {
      await route.fulfill({
        json: fixture.profiles ?? [
          { profile_id: "default", name: "Default", datasets: 0, runs: 0, simulations: 0 },
        ],
      });
      return;
    }
    if (fixture.routes !== undefined && (await fixture.routes(path, route))) {
      return;
    }
    await route.fulfill({
      status: 404,
      json: {
        v: API,
        error: { code: "test.unhandled", message: `${path} is not fixtured` },
      },
    });
  });
}
