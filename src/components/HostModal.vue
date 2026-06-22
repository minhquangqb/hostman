<script setup lang="ts">
import { computed, reactive, watch } from "vue";
import type { Host } from "../types";

const props = defineProps<{
  modelHost: Host;
  defaultTld: string;
}>();

const emit = defineEmits<{
  (e: "save", host: Host): void;
  (e: "close"): void;
}>();

const form = reactive<Host>({
  ...props.modelHost,
  paths: (props.modelHost.paths ?? []).map((p) => ({ ...p })),
});

watch(
  () => props.modelHost,
  (h) => Object.assign(form, { ...h, paths: (h.paths ?? []).map((p) => ({ ...p })) }),
);

const isEdit = computed(() => form.id !== "");

function addPath() {
  form.paths.push({ path: "/", target: "localhost:4000", stripPrefix: false });
}
function removePath(i: number) {
  form.paths.splice(i, 1);
}

// Auto-suggest the domain from name + defaultTld while the user hasn't typed one.
function onNameInput() {
  if (!form.domain || form.domain === lastSuggest.value) {
    const d = `${form.name}.${props.defaultTld}`;
    form.domain = form.name ? d : "";
    lastSuggest.value = form.domain;
  }
}
const lastSuggest = { value: "" } as { value: string };

const error = computed(() => {
  if (!form.name.trim()) return "Name cannot be empty";
  if (!form.domain.trim()) return "Domain cannot be empty";
  if (!/^[a-z0-9.-]+$/i.test(form.domain)) return "Domain may only contain letters, numbers, dots and hyphens";
  if (!/^[^\s:]+:\d+$/.test(form.target)) return "Target must be host:port (e.g. localhost:3000)";
  for (const [i, p] of form.paths.entries()) {
    const n = i + 1;
    if (!p.path.trim()) return `Path #${n}: path is required (e.g. /admin)`;
    if (!/^\/[\w\-./]*\*?$/.test(p.path.trim())) return `Path #${n}: invalid path (must start with /)`;
    if (!/^[^\s:]+:\d+$/.test(p.target)) return `Path #${n}: target must be host:port`;
  }
  return "";
});

function submit() {
  if (error.value) return;
  // Drop empty path rows before saving.
  const paths = form.paths
    .filter((p) => p.path.trim() && p.target.trim())
    .map((p) => ({ ...p, path: p.path.trim(), target: p.target.trim() }));
  emit("save", { ...form, paths });
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="modal">
      <h2>{{ isEdit ? "Edit host" : "Add new host" }}</h2>

      <label>Name
        <input v-model="form.name" @input="onNameInput" placeholder="myapp" autofocus />
      </label>

      <label>Domain
        <input v-model="form.domain" placeholder="myapp.test" />
      </label>

      <label>Default target (host:port)
        <input v-model="form.target" placeholder="localhost:2222" />
      </label>

      <div class="paths">
        <div class="paths-head">
          <span class="paths-title">Per-path routing <span class="hint">(optional)</span></span>
          <button type="button" class="ghost xs" @click="addPath">+ Add path</button>
        </div>
        <p v-if="form.paths.length === 0" class="paths-empty">
          None yet. E.g. <code>/admin</code> points to <code>localhost:4000</code>, everything else falls back to the default target.
        </p>
        <div v-for="(p, i) in form.paths" :key="i" class="path-row">
          <input v-model="p.path" class="p-path" placeholder="/admin" />
          <span class="arrow">→</span>
          <input v-model="p.target" class="p-target" placeholder="localhost:4000" />
          <label class="check strip" title="Strip the path prefix before proxying (handle_path)">
            <input type="checkbox" v-model="p.stripPrefix" /> strip
          </label>
          <button type="button" class="danger xs" title="Remove path" @click="removePath(i)">✕</button>
        </div>
      </div>

      <div class="row">
        <label class="check"><input type="checkbox" v-model="form.https" /> HTTPS</label>
        <label class="check"><input type="checkbox" v-model="form.enabled" /> Enabled</label>
      </div>

      <p v-if="error" class="error">{{ error }}</p>

      <div class="actions">
        <button class="ghost" @click="emit('close')">Cancel</button>
        <button class="primary" :disabled="!!error" @click="submit">Save</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: grid;
  place-items: center;
  z-index: 50;
}
.modal {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 24px;
  width: 420px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
}
h2 { margin: 0; font-size: 18px; }
label { display: flex; flex-direction: column; gap: 6px; font-size: 13px; color: var(--muted); }
input[type="text"], input:not([type]) {
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  color: var(--text);
  font-size: 14px;
}
.row { display: flex; gap: 20px; }
.check { flex-direction: row; align-items: center; gap: 8px; color: var(--text); }
.error { color: #ff6b6b; font-size: 13px; margin: 0; }
.actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 4px; }

.paths { display: flex; flex-direction: column; gap: 8px; }
.paths-head { display: flex; justify-content: space-between; align-items: center; }
.paths-title { font-size: 13px; color: var(--muted); }
.hint { font-weight: 400; opacity: 0.7; }
.paths-empty { margin: 0; font-size: 12px; color: var(--muted); }
.path-row { display: flex; align-items: center; gap: 8px; }
.path-row input { padding: 8px 10px; font-size: 13px; }
.p-path { width: 110px; flex-shrink: 0; }
.p-target { flex: 1; min-width: 0; }
.arrow { color: var(--muted); flex-shrink: 0; }
.strip { font-size: 12px; color: var(--muted); flex-shrink: 0; }
button.xs { padding: 5px 10px; font-size: 12px; }
</style>
