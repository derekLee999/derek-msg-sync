<script setup lang="ts">
defineProps<{
  title: string
  message: string
  confirmText?: string
  cancelText?: string
}>()

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()
</script>

<template>
  <Teleport to="body">
    <div class="confirm-backdrop" @click.self="emit('cancel')">
      <section class="confirm-dialog" role="dialog" aria-modal="true" :aria-label="title">
        <div class="confirm-body">
          <h2 class="confirm-title">{{ title }}</h2>
          <p class="confirm-message">{{ message }}</p>
        </div>
        <div class="confirm-actions">
          <button type="button" class="confirm-button secondary" @click="emit('cancel')">
            {{ cancelText ?? '取消' }}
          </button>
          <button type="button" class="confirm-button primary" @click="emit('confirm')">
            {{ confirmText ?? '确定' }}
          </button>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.confirm-backdrop {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(15, 23, 42, 0.36);
}

.confirm-dialog {
  width: min(420px, calc(100vw - 48px));
  overflow: hidden;
  border: 1px solid rgba(28, 39, 54, 0.1);
  border-radius: 10px;
  background: #ffffff;
  box-shadow: 0 24px 70px rgba(28, 39, 54, 0.26);
}

.confirm-body {
  padding: 22px 24px 8px;
}

.confirm-title {
  margin: 0 0 12px;
  color: #17202f;
  font-size: 20px;
  line-height: 28px;
}

.confirm-message {
  margin: 0;
  color: #465160;
  font-size: 15px;
  line-height: 24px;
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 18px 24px 22px;
}

.confirm-button {
  min-width: 86px;
  height: 38px;
  border-radius: 8px;
  cursor: pointer;
  font: inherit;
  font-size: 15px;
  font-weight: 700;
  transition:
    background 140ms ease,
    border-color 140ms ease,
    box-shadow 140ms ease,
    transform 140ms ease;
}

.confirm-button:hover {
  transform: translateY(-1px);
}

.confirm-button:active {
  transform: translateY(0);
}

.confirm-button.primary {
  border: 1px solid #1769e0;
  color: #ffffff;
  background: #1769e0;
  box-shadow: 0 10px 22px rgba(23, 105, 224, 0.24);
}

.confirm-button.primary:hover {
  background: #0f57c4;
}

.confirm-button.secondary {
  border: 1px solid #d0d5dd;
  color: #17202f;
  background: #ffffff;
}

.confirm-button.secondary:hover {
  border-color: #b8c1cc;
  background: #f6f8fb;
}
</style>
