#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { constants } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

const PLATFORMS = {
  "linux-x64": "@ferrflow/linux-x64",
  "linux-arm64": "@ferrflow/linux-arm64",
  "darwin-x64": "@ferrflow/darwin-x64",
  "darwin-arm64": "@ferrflow/darwin-arm64",
  "win32-x64": "@ferrflow/win32-x64",
  "win32-arm64": "@ferrflow/win32-arm64",
  "linux-arm": "@ferrflow/linux-arm",
};

function getBinaryPath() {
  const key = `${process.platform}-${process.arch}`;
  const pkg = PLATFORMS[key];

  if (pkg) {
    try {
      const ext = process.platform === "win32" ? ".exe" : "";
      return require.resolve(`${pkg}/bin/ferrflow${ext}`);
    } catch {
      // optional dep not installed
    }
  }

  // Fallback: local dev build
  const ext = process.platform === "win32" ? ".exe" : "";
  const repoRoot = join(__dirname, "..", "..");
  const inSourceCheckout = existsSync(join(repoRoot, "Cargo.toml"));
  const devBuild = join(repoRoot, "target", "release", `ferrflow${ext}`);
  if (inSourceCheckout && existsSync(devBuild)) return devBuild;

  console.error(
    `Unsupported platform: ${process.platform}-${process.arch}\n` +
    "Install ferrflow from https://github.com/FerrLabs/FerrFlow/releases"
  );
  process.exit(1);
}

const binary = getBinaryPath();
const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`ferrflow: failed to launch ${binary}: ${result.error.message}`);
  process.exit(1);
}
if (result.signal) {
  process.exit(128 + (constants.signals[result.signal] ?? 0));
}
process.exit(result.status ?? 1);
