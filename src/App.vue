<script setup lang="ts">
import { onMounted, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import * as api from "./api";
import {
  emptyHost,
  type CaddyStatus,
  type Config,
  type GitStatus,
  type Host,
  type ServiceStatus,
} from "./types";
import HostModal from "./components/HostModal.vue";
import PreviewModal from "./components/PreviewModal.vue";

const config = ref<Config>({ defaultTld: "test", hosts: [] });
const caddy = ref<CaddyStatus>({ running: false, binary: null });
const service = ref<ServiceStatus>({ supported: false, installed: false, running: false });
const git = ref<GitStatus>({ is_repo: false, dirty: false, ahead: 0, behind: 0, remote: null });

const editing = ref<Host | null>(null);
const preview = ref<{ title: string; content: string } | null>(null);
const busy = ref(false);
const toast = ref<{ msg: string; ok: boolean } | null>(null);

function notify(msg: string, ok = true) {
  toast.value = { msg, ok };
  setTimeout(() => (toast.value = null), 3500);
}

async function run<T>(fn: () => Promise<T>, okMsg?: string): Promise<T | undefined> {
  busy.value = true;
  try {
    const r = await fn();
    if (okMsg) notify(okMsg, true);
    return r;
  } catch (e) {
    notify(String(e), false);
  } finally {
    busy.value = false;
  }
}

async function refresh() {
  config.value = (await run(api.getConfig)) ?? config.value;
  caddy.value = (await run(api.caddyStatus)) ?? caddy.value;
  service.value = (await run(api.serviceStatus)) ?? service.value;
  git.value = (await run(api.gitStatus)) ?? git.value;
}

onMounted(refresh);

// ---- Host CRUD ----
function openAdd() {
  editing.value = emptyHost();
}
function openEdit(h: Host) {
  editing.value = { ...h };
}
async function onSaveHost(h: Host) {
  const c = await run(() => api.saveHost(h), "Đã lưu host");
  if (c) config.value = c;
  editing.value = null;
}
async function onDelete(h: Host) {
  const c = await run(() => api.deleteHost(h.id), "Đã xoá");
  if (c) config.value = c;
}
async function onToggle(h: Host) {
  const c = await run(() => api.toggleHost(h.id, !h.enabled));
  if (c) config.value = c;
}

// ---- Apply ----
async function applyAll() {
  await run(api.applyAll, "Đã áp dụng: hosts file + Caddy");
  await refresh();
}
async function showHostsPreview() {
  const content = await run(api.previewHosts);
  if (content !== undefined) preview.value = { title: "hosts file (preview)", content };
}
async function showCaddyPreview() {
  const content = await run(api.previewCaddyfile);
  if (content !== undefined) preview.value = { title: "Caddyfile (preview)", content };
}
async function openHostsFile() {
  await run(api.openHostsFile);
}

// ---- Caddy ----
async function caddyStart() {
  await run(api.caddyStart, "Caddy đã khởi động");
  caddy.value = (await run(api.caddyStatus)) ?? caddy.value;
}
async function caddyStop() {
  await run(api.caddyStop, "Caddy đã dừng");
  caddy.value = (await run(api.caddyStatus)) ?? caddy.value;
}
async function caddyTrust() {
  await run(api.caddyTrust, "Đã cài CA — HTTPS local giờ được tin cậy");
}

// ---- Service (launchd) ----
async function serviceInstall() {
  const s = await run(api.serviceInstall, "Đã cài Caddy làm service (tự khởi động)");
  if (s) service.value = s;
  caddy.value = (await run(api.caddyStatus)) ?? caddy.value;
}
async function serviceUninstall() {
  const s = await run(api.serviceUninstall, "Đã gỡ service");
  if (s) service.value = s;
  caddy.value = (await run(api.caddyStatus)) ?? caddy.value;
}

// ---- Git ----
async function gitInit() {
  const s = await run(api.gitInit, "Đã khởi tạo git repo");
  if (s) git.value = s;
}
async function gitCommit() {
  const s = await run(() => api.gitCommit("chore: update dev hosts"), "Đã commit");
  if (s) git.value = s;
}
async function gitPull() {
  await run(api.gitPull, "Đã pull");
  await refresh();
}
async function gitPush() {
  await run(api.gitPush, "Đã push");
  git.value = (await run(api.gitStatus)) ?? git.value;
}
async function gitSetRemote() {
  const url = window.prompt("Remote URL (git):", git.value.remote ?? "");
  if (!url) return;
  const s = await run(() => api.gitSetRemote(url), "Đã gán remote");
  if (s) git.value = s;
}

function scheme(h: Host) {
  return h.https ? "https" : "http";
}
function hostUrl(h: Host) {
  return `${scheme(h)}://${h.domain}`;
}
async function openLink(h: Host) {
  await run(() => openUrl(hostUrl(h)));
}
</script>

<template>
  <div class="app">
    <header>
      <div class="brand">
        <div class="logo">HM</div>
        <div>
          <h1>Hostman</h1>
          <p class="sub">Quản lý dev host local · TLD mặc định <code>.{{ config.defaultTld }}</code></p>
        </div>
      </div>
      <div class="caddy-status">
        <span class="dot" :class="{ on: caddy.running }"></span>
        <span>Caddy: {{ caddy.running ? "đang chạy" : "đã dừng" }}</span>
        <button v-if="!caddy.running" class="ghost sm" :disabled="busy" @click="caddyStart">Start</button>
        <button v-else class="ghost sm" :disabled="busy" @click="caddyStop">Stop</button>
        <button class="ghost sm" :disabled="busy" title="Cài CA của Caddy vào trust store hệ thống (chạy 1 lần)" @click="caddyTrust">Trust HTTPS</button>
      </div>
    </header>

    <section class="toolbar">
      <button class="primary" @click="openAdd">+ Thêm host</button>
      <div class="spacer"></div>
      <button class="ghost" :disabled="busy" @click="showHostsPreview">Xem hosts</button>
      <button class="ghost" :disabled="busy" title="Mở /etc/hosts bằng trình soạn thảo mặc định" @click="openHostsFile">Mở file hosts</button>
      <button class="ghost" :disabled="busy" @click="showCaddyPreview">Xem Caddyfile</button>
      <button class="accent" :disabled="busy" @click="applyAll">Áp dụng ⚡</button>
    </section>

    <section class="list">
      <div v-if="config.hosts.length === 0" class="empty">
        Chưa có host nào. Bấm <strong>+ Thêm host</strong> để bắt đầu.
      </div>
      <div v-for="h in config.hosts" :key="h.id" class="card" :class="{ off: !h.enabled }">
        <button class="toggle" :class="{ on: h.enabled }" :title="h.enabled ? 'Đang bật' : 'Đang tắt'" @click="onToggle(h)">
          <span></span>
        </button>
        <div class="info">
          <div class="domain">{{ scheme(h) }}://{{ h.domain }}</div>
          <div class="target">→ {{ h.target }}</div>
        </div>
        <div class="tags">
          <span v-if="h.https" class="tag">HTTPS</span>
        </div>
        <div class="card-actions">
          <button class="ghost sm" :disabled="!h.enabled || busy" title="Mở trong trình duyệt" @click="openLink(h)">Mở ↗</button>
          <button class="ghost sm" @click="openEdit(h)">Sửa</button>
          <button class="danger sm" @click="onDelete(h)">Xoá</button>
        </div>
      </div>
    </section>

    <section class="git">
      <div class="git-head">
        <h3>Chạy nền (Service)</h3>
        <span class="git-meta">
          <span v-if="service.installed" class="badge ok">đã cài</span>
          <span v-else class="badge warn">chưa cài</span>
        </span>
      </div>
      <div v-if="!service.supported" class="git-actions">
        <span class="muted">Hệ điều hành này chưa hỗ trợ chạy Caddy như service.</span>
      </div>
      <div v-else class="git-actions">
        <span class="muted">
          Cài Caddy làm dịch vụ nền (launchd) để tự khởi động cùng máy và không phải cấp quyền admin mỗi lần.
        </span>
        <button v-if="!service.installed" class="ghost sm" :disabled="busy" @click="serviceInstall">Cài service</button>
        <button v-else class="danger sm" :disabled="busy" @click="serviceUninstall">Gỡ service</button>
      </div>
    </section>

    <section class="git">
      <div class="git-head">
        <h3>Đồng bộ (Git)</h3>
        <span class="git-meta" v-if="git.is_repo">
          <span v-if="git.dirty" class="badge warn">có thay đổi</span>
          <span v-else class="badge ok">sạch</span>
          <span v-if="git.ahead">↑{{ git.ahead }}</span>
          <span v-if="git.behind">↓{{ git.behind }}</span>
        </span>
      </div>
      <div v-if="!git.is_repo" class="git-actions">
        <span class="muted">Thư mục config chưa phải git repo.</span>
        <button class="ghost sm" :disabled="busy" @click="gitInit">Khởi tạo repo</button>
      </div>
      <div v-else class="git-actions">
        <span class="muted">Remote: {{ git.remote ?? "chưa gán" }}</span>
        <button class="ghost sm" :disabled="busy" @click="gitSetRemote">Gán remote</button>
        <button class="ghost sm" :disabled="busy" @click="gitCommit">Commit</button>
        <button class="ghost sm" :disabled="busy" @click="gitPull">Pull</button>
        <button class="ghost sm" :disabled="busy" @click="gitPush">Push</button>
      </div>
    </section>

    <HostModal
      v-if="editing"
      :model-host="editing"
      :default-tld="config.defaultTld"
      @save="onSaveHost"
      @close="editing = null"
    />
    <PreviewModal
      v-if="preview"
      :title="preview.title"
      :content="preview.content"
      @close="preview = null"
    />

    <transition name="fade">
      <div v-if="toast" class="toast" :class="{ err: !toast.ok }">{{ toast.msg }}</div>
    </transition>
  </div>
</template>

<style>
:root {
  --bg: #0e1116;
  --surface: #171c24;
  --border: #2a3340;
  --text: #e6edf3;
  --muted: #8b97a7;
  --primary: #4c8bf5;
  --accent: #2ea043;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  font-family: -apple-system, "Segoe UI", Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
}
button {
  cursor: pointer;
  border: none;
  border-radius: 8px;
  padding: 9px 16px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
  transition: filter 0.15s, background 0.15s;
}
button:disabled { opacity: 0.5; cursor: not-allowed; }
button.primary { background: var(--primary); }
button.accent { background: var(--accent); }
button.ghost { background: transparent; border: 1px solid var(--border); }
button.danger { background: transparent; border: 1px solid #5c2a2a; color: #ff8585; }
button.sm { padding: 6px 12px; font-size: 13px; }
button:not(:disabled):hover { filter: brightness(1.12); }
code { background: rgba(255,255,255,0.08); padding: 1px 6px; border-radius: 5px; font-size: 0.9em; }
</style>

<style scoped>
.app {
  max-width: 860px;
  margin: 0 auto;
  padding: 28px 24px 60px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}
header { display: flex; justify-content: space-between; align-items: center; }
.brand { display: flex; gap: 14px; align-items: center; }
.logo {
  width: 44px; height: 44px;
  border-radius: 12px;
  background: linear-gradient(135deg, var(--primary), #7c5cf5);
  display: grid; place-items: center;
  font-weight: 800; font-size: 16px;
}
h1 { margin: 0; font-size: 22px; }
.sub { margin: 2px 0 0; color: var(--muted); font-size: 13px; }
.caddy-status { display: flex; align-items: center; gap: 10px; font-size: 13px; color: var(--muted); }
.dot { width: 9px; height: 9px; border-radius: 50%; background: #6b7280; }
.dot.on { background: var(--accent); box-shadow: 0 0 8px var(--accent); }

.toolbar { display: flex; gap: 10px; align-items: center; }
.spacer { flex: 1; }

.list { display: flex; flex-direction: column; gap: 10px; }
.empty {
  border: 1px dashed var(--border);
  border-radius: 12px;
  padding: 40px;
  text-align: center;
  color: var(--muted);
}
.card {
  display: flex;
  align-items: center;
  gap: 16px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 14px 16px;
}
.card.off { opacity: 0.55; }
.info { flex: 1; min-width: 0; }
.domain { font-weight: 600; font-size: 15px; }
.target { color: var(--muted); font-size: 13px; margin-top: 2px; font-family: ui-monospace, monospace; }
.tags { display: flex; gap: 6px; }
.tag { font-size: 11px; background: rgba(76,139,245,0.15); color: #8db4ff; padding: 3px 8px; border-radius: 6px; }
.card-actions { display: flex; gap: 8px; }

.toggle {
  width: 42px; height: 24px; border-radius: 12px;
  background: #3a4350; padding: 3px; flex-shrink: 0;
}
.toggle span { display: block; width: 18px; height: 18px; border-radius: 50%; background: #fff; transition: transform 0.18s; }
.toggle.on { background: var(--accent); }
.toggle.on span { transform: translateX(18px); }

.git {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
}
.git-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
.git-head h3 { margin: 0; font-size: 15px; }
.git-meta { display: flex; gap: 8px; align-items: center; font-size: 12px; color: var(--muted); }
.badge { font-size: 11px; padding: 2px 8px; border-radius: 6px; }
.badge.ok { background: rgba(46,160,67,0.15); color: #58c46f; }
.badge.warn { background: rgba(210,153,34,0.15); color: #e3b341; }
.git-actions { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
.muted { color: var(--muted); font-size: 13px; }

.toast {
  position: fixed;
  bottom: 24px; left: 50%; transform: translateX(-50%);
  background: var(--accent); color: #fff;
  padding: 12px 20px; border-radius: 10px;
  font-size: 14px; font-weight: 600;
  box-shadow: 0 10px 30px rgba(0,0,0,0.4);
  max-width: 90%;
}
.toast.err { background: #d33; }
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
