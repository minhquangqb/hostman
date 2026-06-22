<script setup lang="ts">
defineProps<{
  title: string;
  content: string;
}>();
const emit = defineEmits<{ (e: "close"): void }>();
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="modal">
      <div class="head">
        <h2>{{ title }}</h2>
        <button class="ghost" @click="emit('close')">Đóng</button>
      </div>
      <pre>{{ content }}</pre>
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
  padding: 20px;
  width: 600px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.head { display: flex; justify-content: space-between; align-items: center; }
h2 { margin: 0; font-size: 16px; }
pre {
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 14px;
  overflow: auto;
  font-size: 13px;
  line-height: 1.5;
  margin: 0;
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
}
</style>
