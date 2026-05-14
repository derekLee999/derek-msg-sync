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
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: blur(12px);
}

.confirm-dialog {
  width: min(420px, calc(100vw - 48px));
  overflow: hidden;
  border: 1px solid var(--glass-border);
  border-radius: var(--glass-radius-large);
  background: var(--glass-bg-heavy);
  backdrop-filter: var(--glass-blur);
  box-shadow: var(--glass-shadow-heavy);
}

.confirm-body {
  padding: 22px 24px 8px;
}

.confirm-title {
  margin: 0 0 12px;
  color: var(--text-primary);
  font-size: 20px;
  line-height: 28px;
  font-weight: 600;
}

.confirm-message {
  margin: 0;
  color: var(--text-secondary);
  font-size: 15px;
  line-height: 1.5;
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
  border-radius: var(--glass-radius-small);
  cursor: pointer;
  font: inherit;
  font-size: 15px;
  font-weight: 600;
  transition:
    background 0.2s ease,
    border-color 0.2s ease,
    box-shadow 0.2s ease,
    transform 0.2s cubic-bezier(0.25, 1, 0.5, 1);
}

.confirm-button:hover {
  transform: translateY(-1px) scale(1.02);
}

.confirm-button:active {
  transform: translateY(0) scale(0.98);
}

.confirm-button.primary {
  border: 0;
  color: #ffffff;
  background: var(--accent-gradient);
  box-shadow: 0 4px 16px rgba(0, 122, 255, 0.25);
}

.confirm-button.primary:hover {
  filter: brightness(1.1);
  box-shadow: 0 8px 24px rgba(0, 122, 255, 0.35);
}

.confirm-button.secondary {
  border: 1px solid var(--glass-border);
  color: var(--text-primary);
  background: var(--glass-bg-medium);
}

.confirm-button.secondary:hover {
  background: var(--glass-bg-heavy);
}
</style>
