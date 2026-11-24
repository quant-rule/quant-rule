import { defineConfig, devices } from "@playwright/test";

// The remote suite is lab-gated. It runs against the provisioned
// https://quantrule.dev gateway through its VIP and never starts a local server
// or mocks a route. The client OS or NSS store must already trust the private
// CA; this config deliberately has no certificate-error bypass.
const remoteUrl = process.env.QR_REMOTE_URL;

if (process.env.QR_REMOTE_BROWSER !== "1" || remoteUrl !== "https://quantrule.dev") {
  console.error(
    "Unverified: the remote browser suite is lab-gated; set QR_REMOTE_BROWSER=1 and QR_REMOTE_URL=https://quantrule.dev",
  );
  process.exit(1);
}

export default defineConfig({
  testDir: "./e2e-remote",
  testMatch: "**/*.e2e.ts",
  fullyParallel: false,
  forbidOnly: true,
  retries: 0,
  reporter: "list",
  use: {
    baseURL: "https://quantrule.dev",
    acceptDownloads: false,
    trace: "on-first-retry",
  },
  projects: [{ name: "remote-chromium", use: { ...devices["Desktop Chrome"] } }],
});
