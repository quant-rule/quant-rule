#!/usr/bin/env bun
/**
 * Stage the release sidecars and the config examples for a Tauri bundle.
 *
 * This runs after the six service binaries and `qr-desktop` are built for the
 * target, and before `tauri build`. Tauri copies `resources/sidecars` and
 * `resources/config` to the resource directory; the shell passes that
 * directory to the supervisor.
 *
 * A missing binary is an error, not a warning: an installer without all seven
 * executables is not the desktop profile and must not reach a release. A
 * development build does not run this script, so the shell still builds
 * without the sidecars.
 *
 * Bun is the client toolchain and is present on both CI runners.
 */

import { chmodSync, cpSync, existsSync, mkdirSync, readdirSync, rmSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repository = resolve(root, "..", "..");
const suffix = process.platform === "win32" ? ".exe" : "";
const services = ["qr-gateway", "qr-data", "qr-model", "qr-backtest", "qr-evidence", "qr-simulation"];

function option(name, fallback) {
  const index = process.argv.indexOf(name);
  if (index === -1) {
    return fallback;
  }
  const value = process.argv[index + 1];
  if (!value) {
    console.error(`stage-package: ${name} needs a value`);
    process.exit(2);
  }
  return resolve(value);
}

const servicesDir = option("--services-dir", process.env.QR_SERVICES_DIR ?? join(repository, "target", "release"));
const supervisorOption = option(
  "--supervisor-bin",
  process.env.QR_SUPERVISOR_BIN ?? join(root, "supervisor", "target", "release", `qr-desktop${suffix}`),
);
const supervisorBin =
  existsSync(supervisorOption) && statSync(supervisorOption).isDirectory()
    ? join(supervisorOption, `qr-desktop${suffix}`)
    : supervisorOption;
const examplesDir = option("--examples-dir", join(repository, "config", "examples"));
const sidecarsOut = join(root, "resources", "sidecars");
const configOut = join(root, "resources", "config");

function fail(message) {
  console.error(`stage-package: ${message}`);
  process.exit(1);
}

function requireFile(path, label) {
  if (!existsSync(path) || !statSync(path).isFile()) {
    fail(`missing ${label} at ${path}`);
  }
  return path;
}

const executables = [
  ...services.map((name) => requireFile(join(servicesDir, `${name}${suffix}`), `${name} service`)),
  requireFile(supervisorBin, "qr-desktop supervisor"),
];

if (!existsSync(join(examplesDir, "verify-mini", "run.json"))) {
  fail(`missing the example configs at ${examplesDir}`);
}

mkdirSync(sidecarsOut, { recursive: true });
for (const entry of readdirSync(sidecarsOut)) {
  if (entry.startsWith("qr-")) {
    rmSync(join(sidecarsOut, entry), { force: true });
  }
}
for (const executable of executables) {
  const target = join(sidecarsOut, executable.split(/[\\/]/).pop());
  cpSync(executable, target);
  if (process.platform !== "win32") {
    chmodSync(target, 0o755);
  }
}

mkdirSync(configOut, { recursive: true });
for (const entry of readdirSync(configOut)) {
  if (entry !== "README.md") {
    rmSync(join(configOut, entry), { recursive: true, force: true });
  }
}
let configs = 0;
for (const entry of readdirSync(examplesDir)) {
  if (entry.startsWith(".")) {
    continue;
  }
  cpSync(join(examplesDir, entry), join(configOut, entry), { recursive: true });
  configs += 1;
}

console.log(`stage-package: ${executables.length} executables in ${sidecarsOut}`);
console.log(`stage-package: ${configs} example configs in ${configOut}`);
