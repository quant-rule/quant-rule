#!/usr/bin/env bun
/**
 * Stage the shell bootstrap and the built web client into dist/.
 *
 * `tauri-build` embeds `dist/` at compile time. The files here are generated:
 * `apps/web/dist` is the one client, copied unchanged, and `shell.html` is the
 * bootstrap that decides between the gateway and the disconnected fallback. The
 * shell does not copy a data file, a config, or an engine into this directory.
 *
 * Bun is the client toolchain, so this runs on Linux and Windows without a
 * second shell.
 */

import { cpSync, existsSync, readdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const web = resolve(root, "..", "web");
const webDist = join(web, "dist");
const out = join(root, "dist");
const skipWebBuild = process.argv.includes("--skip-web-build");

function run(args, cwd) {
  const result = Bun.spawnSync(args, { cwd, stdout: "inherit", stderr: "inherit" });
  if (result.exitCode !== 0) {
    console.error(`stage-frontend: ${args.join(" ")} failed`);
    process.exit(result.exitCode ?? 1);
  }
}

if (!skipWebBuild) {
  run(["bun", "install", "--frozen-lockfile"], web);
  run(["bun", "run", "build"], web);
}

if (!existsSync(join(webDist, "index.html"))) {
  console.error(`stage-frontend: missing ${join(webDist, "index.html")}`);
  process.exit(1);
}

rmSync(out, { recursive: true, force: true });
cpSync(webDist, out, { recursive: true });

for (const file of ["shell.html", "shell.css", "shell.js", "shell-lib.js"]) {
  cpSync(join(root, "ui", file), join(out, file));
}

const count = (() => {
  let total = 0;
  const walk = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      if (entry.isDirectory()) {
        walk(join(directory, entry.name));
      } else {
        total += 1;
      }
    }
  };
  walk(out);
  return total;
})();

console.log(`staged ${count} files in ${out}`);
