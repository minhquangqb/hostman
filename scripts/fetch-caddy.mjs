#!/usr/bin/env node
// Tai Caddy binary ve lam sidecar cho Tauri.
// Dat tai src-tauri/binaries/caddy-<target-triple>(.exe) theo quy uoc externalBin.
//
// Dung:
//   node scripts/fetch-caddy.mjs                          # tai cho may hien tai
//   node scripts/fetch-caddy.mjs --target <rust-triple>   # tai cho target chi dinh (CI)
//
// Vi du: node scripts/fetch-caddy.mjs --target aarch64-apple-darwin

import { mkdir, writeFile, chmod } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const binDir = join(root, "src-tauri", "binaries");

// Rust target triple -> { os, arch, ext } cho Caddy download API.
const TRIPLE_MAP = {
  "x86_64-apple-darwin": { os: "darwin", arch: "amd64", ext: "" },
  "aarch64-apple-darwin": { os: "darwin", arch: "arm64", ext: "" },
  "x86_64-pc-windows-msvc": { os: "windows", arch: "amd64", ext: ".exe" },
  "aarch64-pc-windows-msvc": { os: "windows", arch: "arm64", ext: ".exe" },
  "x86_64-unknown-linux-gnu": { os: "linux", arch: "amd64", ext: "" },
  "aarch64-unknown-linux-gnu": { os: "linux", arch: "arm64", ext: "" },
};

// Detect target triple cua may hien tai (khi khong truyen --target).
function currentTriple() {
  const p = process.platform;
  const a = process.arch === "arm64" ? "aarch64" : "x86_64";
  if (p === "darwin") return `${a}-apple-darwin`;
  if (p === "win32") return `${a}-pc-windows-msvc`;
  if (p === "linux") return `${a}-unknown-linux-gnu`;
  throw new Error(`Platform khong ho tro: ${p}`);
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
      `Target khong ho tro: ${triple}\nCac target hop le: ${Object.keys(TRIPLE_MAP).join(", ")}`,
    );
  }

  const url = `https://caddyserver.com/api/download?os=${t.os}&arch=${t.arch}`;
  const out = join(binDir, `caddy-${triple}${t.ext}`);

  if (existsSync(out)) {
    console.log(`Da co: ${out}`);
    return;
  }

  await mkdir(binDir, { recursive: true });
  console.log(`Tai Caddy cho ${triple} (${t.os}/${t.arch}) ...`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Tai that bai: HTTP ${res.status}`);
  const buf = Buffer.from(await res.arrayBuffer());
  await writeFile(out, buf);
  if (t.ext === "") await chmod(out, 0o755);
  console.log(`Xong: ${out} (${(buf.length / 1e6).toFixed(1)} MB)`);
  console.log(`\nDev: export HOSTMAN_CADDY="${out}"`);
}

main().catch((e) => {
  console.error(e.message);
  process.exit(1);
});
