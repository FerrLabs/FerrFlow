#!/usr/bin/env node
import { spawnSync } from "child_process";
import { existsSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import { createRequire } from "module";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

const PLATFORMS = {
  "linux-x64": "@ferrflow/linux-x64",
  "linux-arm64": "@ferrflow/linux-arm64",
  "darwin-x64": "@ferrflow/darwin-x64",
  "darwin-arm64": "@ferrflow/darwin-arm64",
  "win32-x64": "@ferrflow/win32-x64",
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
  const devBuild = join(__dirname, "..", "..", "target", "release", `ferrflow${ext}`);
  if (existsSync(devBuild)) return devBuild;

  console.error(
    `Unsupported platform: ${process.platform}-${process.arch}\n` +
    "Install ferrflow from https://github.com/FerrFlow-Org/FerrFlow/releases"
  );
  process.exit(1);
}

const binary = getBinaryPath();
const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
process.exit(result.status ?? 1);
