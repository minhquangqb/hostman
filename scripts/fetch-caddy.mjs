#!/usr/bin/env node
// Download the Caddy binary to use as a Tauri sidecar.
// Placed at src-tauri/binaries/caddy-<target-triple>(.exe) following the externalBin convention.
//
// Usage:
//   node scripts/fetch-caddy.mjs                          # download for the current machine
//   node scripts/fetch-caddy.mjs --target <rust-triple>   # download for a specific target (CI)
//
// Example: node scripts/fetch-caddy.mjs --target aarch64-apple-darwin

import { mkdir, writeFile, chmod } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const binDir = join(root, "src-tauri", "binaries");

// Rust target triple -> { os, arch, ext } for the Caddy download API.
const TRIPLE_MAP = {
  "x86_64-apple-darwin": { os: "darwin", arch: "amd64", ext: "" },
  "aarch64-apple-darwin": { os: "darwin", arch: "arm64", ext: "" },
  "x86_64-pc-windows-msvc": { os: "windows", arch: "amd64", ext: ".exe" },
  "aarch64-pc-windows-msvc": { os: "windows", arch: "arm64", ext: ".exe" },
  "x86_64-unknown-linux-gnu": { os: "linux", arch: "amd64", ext: "" },
  "aarch64-unknown-linux-gnu": { os: "linux", arch: "arm64", ext: "" },
};

// Detect the target triple of the current machine (when --target is not passed).
function currentTriple() {
  const p = process.platform;
  const a = process.arch === "arm64" ? "aarch64" : "x86_64";
  if (p === "darwin") return `${a}-apple-darwin`;
  if (p === "win32") return `${a}-pc-windows-msvc`;
  if (p === "linux") return `${a}-unknown-linux-gnu`;
  throw new Error(`Unsupported platform: ${p}`);
}

function parseArgs() {
  const idx = process.argv.indexOf("--target");
  if (idx !== -1 && process.argv[idx + 1]) return process.argv[idx + 1];
  return currentTriple();
}

async function main() {
  const triple = parseArgs();
  const t = TRIPLE_MAP[triple];
  if (!t) {
    throw new Error(
      `Unsupported target: ${triple}\nValid targets: ${Object.keys(TRIPLE_MAP).join(", ")}`,
    );
  }

  const url = `https://caddyserver.com/api/download?os=${t.os}&arch=${t.arch}`;
  const out = join(binDir, `caddy-${triple}${t.ext}`);

  if (existsSync(out)) {
    console.log(`Already present: ${out}`);
    return;
  }

  await mkdir(binDir, { recursive: true });
  console.log(`Downloading Caddy for ${triple} (${t.os}/${t.arch}) ...`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Download failed: HTTP ${res.status}`);
  const buf = Buffer.from(await res.arrayBuffer());
  await writeFile(out, buf);
  if (t.ext === "") await chmod(out, 0o755);
  console.log(`Done: ${out} (${(buf.length / 1e6).toFixed(1)} MB)`);
  console.log(`\nDev: export HOSTMAN_CADDY="${out}"`);
}

main().catch((e) => {
  console.error(e.message);
  process.exit(1);
});
