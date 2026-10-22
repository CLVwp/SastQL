#!/usr/bin/env node
"use strict";

// Launcher: resolves the platform-specific package (esbuild/biome pattern)
// that npm/pnpm/bun installed as an optionalDependency, spawns the native
// binary with forwarded stdio and exit code.

const { spawnSync } = require("child_process");
const { existsSync } = require("fs");
const { dirname, join } = require("path");

function die(msg) {
  process.stderr.write(`sastql: ${msg}\n`);
  process.exit(2);
}

// Node built against musl (Alpine) has no glibc runtime version in its report.
function isGlibc() {
  try {
    return Boolean(process.report.getReport().header.glibcVersionRuntime);
  } catch {
    return false;
  }
}

function packageName() {
  if (process.platform === "linux") {
    return `sastql-linux-${process.arch}-${isGlibc() ? "gnu" : "musl"}`;
  }
  return `sastql-${process.platform}-${process.arch}`;
}

let pkg = packageName();
// Bun on glibc may not implement process.report; fall back to the gnu package.
if (process.platform === "linux" && pkg.endsWith("-musl")) {
  const alt = pkg.replace(/-musl$/, "-gnu");
  try {
    // biome-ignore lint: probe only, path unused
    require.resolve(`${pkg}/package.json`);
  } catch {
    try {
      require.resolve(`${alt}/package.json`);
      pkg = alt;
    } catch {
      // both missing — the error below lists what was tried
    }
  }
}

let dir;
try {
  dir = dirname(require.resolve(`${pkg}/package.json`));
} catch {
  // The supported list is stamped into the manifest at release time
  // (stamp-main.mjs); a dev checkout has no list and prints the plain line.
  let listed = "";
  try {
    const platforms = require("../package.json").sastql?.platforms;
    if (Array.isArray(platforms) && platforms.length > 0) {
      listed =
        `\nsupported: ${platforms.join(", ")}` +
        `\nif your platform is listed, reinstall with optional dependencies enabled`;
    }
  } catch {}
  die(
    `no binary for ${process.platform}-${process.arch}` +
      (process.platform === "linux" ? ` (libc: ${isGlibc() ? "glibc" : "musl"})` : "") +
      (listed ||
        `\nif your platform is supported, reinstall with optional dependencies enabled`),
  );
}

const bin = join(dir, "bin", process.platform === "win32" ? "sastql.exe" : "sastql");

if (!existsSync(bin)) {
  die(`corrupt installation: ${bin} is missing — reinstall the package`);
}

const run = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
if (run.error) {
  die(`failed to spawn ${bin}: ${run.error.message}`);
}
if (run.signal) {
  process.kill(process.pid, run.signal);
}
process.exit(run.status ?? 1);
