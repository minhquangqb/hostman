#!/usr/bin/env node
// Tai Caddy binary ve lam sidecar cho Tauri.
// Dat tai src-tauri/binaries/caddy-<target-triple>(.exe) theo quy uoc externalBin.
//
// Dung: node scripts/fetch-caddy.mjs
// Tai cho may hien tai. Build cross-platform thi chay tren tung OS hoac sua mapping.

import { mkdir, writeFile, chmod } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const binDir = join(root, "src-tauri", "binaries");

// Map Node platform/arch -> {caddyOs, caddyArch, targetTriple, ext}
function resolveTarget() {
  const p = process.platform;
  const a = process.arch; // 'x64' | 'arm64'
  const arch = a === "arm64" ? "arm64" : "amd64";
  if (p === "darwin") {
    return {
      os: "darwin",
      arch,
      triple: arch === "arm64" ? "aarch64-apple-darwin" : "x86_64-apple-darwin",
      ext: "",
    };
  }
  if (p === "win32") {
    return {
      os: "windows",
      arch,
      triple: arch === "arm64" ? "aarch64-pc-windows-msvc" : "x86_64-pc-windows-msvc",
      ext: ".exe",
    };
  }
  if (p === "linux") {
    return {
      os: "linux",
      arch,
      triple: arch === "arm64" ? "aarch64-unknown-linux-gnu" : "x86_64-unknown-linux-gnu",
      ext: "",
    };
  }
  throw new Error(`Platform khong ho tro: ${p}`);
}

async function main() {
  const t = resolveTarget();
  // Caddy download API tra ve binary truc tiep.
  const url = `https://caddyserver.com/api/download?os=${t.os}&arch=${t.arch}`;
  const out = join(binDir, `caddy-${t.triple}${t.ext}`);

  if (existsSync(out)) {
    console.log(`Da co: ${out}`);
    return;
  }

  await mkdir(binDir, { recursive: true });
  console.log(`Tai Caddy (${t.os}/${t.arch}) ...`);
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
