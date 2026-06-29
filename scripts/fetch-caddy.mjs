#!/usr/bin/env node
// Download the Caddy binary to use as a Tauri sidecar.
// Placed at src-tauri/binaries/caddy-<target-triple>(.exe) following the externalBin convention.
//
// Usage:
//   node scripts/fetch-caddy.mjs                          # download for the current machine
//   node scripts/fetch-caddy.mjs --target <rust-triple>   # download for a specific target (CI)
//
// Example: node scripts/fetch-caddy.mjs --target aarch64-apple-darwin
//
// Sources, tried in order:
//   1. caddyserver.com/api/download (custom build endpoint)
//   2. GitHub releases (fallback when 1 is unreachable, e.g. IPv6-only DNS).

import { mkdir, writeFile, chmod, rm, readdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";
import { spawn } from "node:child_process";

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

// fetch with a timeout so an unreachable host fails fast instead of hanging.
async function fetchWithTimeout(url, opts = {}, ms = 30000) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), ms);
  try {
    return await fetch(url, { ...opts, signal: ctrl.signal });
  } finally {
    clearTimeout(timer);
  }
}

function run(cmd, args, cwd) {
  return new Promise((resolve, reject) => {
    const p = spawn(cmd, args, { cwd, stdio: "inherit" });
    p.on("error", reject);
    p.on("close", (code) =>
      code === 0 ? resolve() : reject(new Error(`${cmd} exited with code ${code}`)),
    );
  });
}

// Primary: caddyserver.com custom build endpoint. Returns the raw binary.
async function fromCaddyServer(t) {
  const url = `https://caddyserver.com/api/download?os=${t.os}&arch=${t.arch}`;
  console.log(`Trying caddyserver.com (${t.os}/${t.arch}) ...`);
  const res = await fetchWithTimeout(url);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return Buffer.from(await res.arrayBuffer());
}

// Fallback: GitHub releases. Downloads the archive, extracts the caddy binary.
async function fromGitHub(t) {
  // GitHub asset OS names differ from the API: darwin -> mac.
  const ghOs = t.os === "darwin" ? "mac" : t.os;
  const archive = t.os === "windows" ? "zip" : "tar.gz";
  console.log(`Falling back to GitHub releases (${ghOs}/${t.arch}) ...`);

  const rel = await fetchWithTimeout(
    "https://api.github.com/repos/caddyserver/caddy/releases/latest",
    { headers: { "User-Agent": "hostman-fetch-caddy" } },
  ).then((r) => {
    if (!r.ok) throw new Error(`GitHub API HTTP ${r.status}`);
    return r.json();
  });

  const re = new RegExp(`_${ghOs}_${t.arch}\\.${archive.replace(".", "\\.")}$`);
  const asset = (rel.assets || []).find((a) => re.test(a.name));
  if (!asset) throw new Error(`No GitHub asset matching ${re}`);
  console.log(`Found ${asset.name} (${rel.tag_name})`);

  const res = await fetchWithTimeout(asset.browser_download_url, {}, 120000);
  if (!res.ok) throw new Error(`Download HTTP ${res.status}`);
  const archiveBuf = Buffer.from(await res.arrayBuffer());

  const work = join(tmpdir(), `hostman-caddy-${process.pid}`);
  await rm(work, { recursive: true, force: true });
  await mkdir(work, { recursive: true });
  await writeFile(join(work, asset.name), archiveBuf);

  // `tar` (bsdtar) ships with macOS, Linux, and Windows 10+ and handles both
  // .tar.gz and .zip, so it covers every target without extra dependencies.
  // Pass the bare filename with cwd set: a Windows path like `C:\...` would be
  // misread as a remote `host:path` by bsdtar because of the drive-letter colon.
  await run("tar", ["-xf", asset.name], work);

  const binName = t.os === "windows" ? "caddy.exe" : "caddy";
  const entries = await readdir(work);
  if (!entries.includes(binName)) {
    throw new Error(`Extracted archive has no ${binName} (got: ${entries.join(", ")})`);
  }
  const { readFile } = await import("node:fs/promises");
  const buf = await readFile(join(work, binName));
  await rm(work, { recursive: true, force: true });
  return buf;
}

async function main() {
  const triple = parseArgs();
  const t = TRIPLE_MAP[triple];
  if (!t) {
    throw new Error(
      `Unsupported target: ${triple}\nValid targets: ${Object.keys(TRIPLE_MAP).join(", ")}`,
    );
  }

  const out = join(binDir, `caddy-${triple}${t.ext}`);
  if (existsSync(out)) {
    console.log(`Already present: ${out}`);
    return;
  }

  let buf;
  try {
    buf = await fromCaddyServer(t);
  } catch (e) {
    console.warn(`caddyserver.com failed: ${e.message}`);
    buf = await fromGitHub(t);
  }

  await mkdir(binDir, { recursive: true });
  await writeFile(out, buf);
  if (t.ext === "") await chmod(out, 0o755);
  console.log(`Done: ${out} (${(buf.length / 1e6).toFixed(1)} MB)`);
  console.log(`\nDev: export HOSTMAN_CADDY="${out}"`);
}

main().catch((e) => {
  console.error(e.message);
  process.exit(1);
});
