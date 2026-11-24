// The lab gate for the remote browser suite.
//
// `bun run test:remote` executes this file before Playwright, so a machine
// without the lab prints `Unverified` instead of silently skipping the suite.
// It never prints the token.

const url = process.env.QR_REMOTE_URL;

if (process.env.QR_REMOTE_BROWSER !== "1" || url !== "https://quantrule.dev") {
  console.error(
    "Unverified: set QR_REMOTE_BROWSER=1 and QR_REMOTE_URL=https://quantrule.dev after provisioning the lab",
  );
  process.exit(1);
}

if (!process.env.QR_REMOTE_TOKEN) {
  console.error(
    "Unverified: QR_REMOTE_TOKEN is required; no token is stored in the repository",
  );
  process.exit(1);
}
