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

const form = reactive<Host>({ ...props.modelHost });

watch(
  () => props.modelHost,
  (h) => Object.assign(form, h),
);

const isEdit = computed(() => form.id !== "");

// Tu dong goi y domain tu name + defaultTld neu user chua nhap domain.
function onNameInput() {
  if (!form.domain || form.domain === lastSuggest.value) {
    const d = `${form.name}.${props.defaultTld}`;
    form.domain = form.name ? d : "";
    lastSuggest.value = form.domain;
  }
}
const lastSuggest = { value: "" } as { value: string };

const error = computed(() => {
  if (!form.name.trim()) return "Tên không được để trống";
  if (!form.domain.trim()) return "Domain không được để trống";
  if (!/^[a-z0-9.-]+$/i.test(form.domain)) return "Domain chỉ gồm chữ, số, dấu chấm và gạch ngang";
  if (!/^[^\s:]+:\d+$/.test(form.target)) return "Target phải dạng host:port (vd localhost:3000)";
  return "";
});

function submit() {
  if (error.value) return;
  emit("save", { ...form });
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="modal">
      <h2>{{ isEdit ? "Sửa host" : "Thêm host mới" }}</h2>

      <label>Tên
        <input v-model="form.name" @input="onNameInput" placeholder="myapp" autofocus />
      </label>

      <label>Domain
        <input v-model="form.domain" placeholder="myapp.test" />
      </label>

      <label>Target (host:port)
        <input v-model="form.target" placeholder="localhost:2222" />
      </label>

      <div class="row">
        <label class="check"><input type="checkbox" v-model="form.https" /> HTTPS</label>
        <label class="check"><input type="checkbox" v-model="form.enabled" /> Bật</label>
      </div>

      <p v-if="error" class="error">{{ error }}</p>

      <div class="actions">
        <button class="ghost" @click="emit('close')">Huỷ</button>
        <button class="primary" :disabled="!!error" @click="submit">Lưu</button>
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
</style>
