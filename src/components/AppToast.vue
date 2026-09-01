<script setup lang="ts">
import type { ToastItem } from '../composables/useToast'

defineProps<{
  items: readonly ToastItem[]
}>()
</script>

<template>
  <Teleport to="body">
    <TransitionGroup name="toast" tag="div" class="toast-stack" aria-live="polite">
      <div v-for="item in items" :key="item.id" class="toast-item">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M20 6 9 17l-5-5" />
        </svg>
        <span>{{ item.text }}</span>
      </div>
    </TransitionGroup>
  </Teleport>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  top: 140px;
  left: 50%;
  z-index: 80;
  display: grid;
  gap: 8px;
  pointer-events: none;
  transform: translateX(-50%);
}

.toast-item {
  min-width: 150px;
  max-width: calc(100vw - 40px);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 10px 16px;
  border: 1px solid rgba(255, 255, 255, 0.4);
  border-radius: 999px;
  color: var(--text-primary);
  background: rgba(255, 255, 255, 0.25);
  background-image: linear-gradient(135deg, rgba(255, 255, 255, 0.4) 0%, rgba(255, 255, 255, 0.1) 100%);
  backdrop-filter: blur(24px) saturate(150%);
  box-shadow: 
    0 8px 32px rgba(0, 0, 0, 0.12),
    inset 0 1px 0 rgba(255, 255, 255, 0.5);
  font-size: 14px;
  font-weight: 600;
}

.toast-item svg {
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
  color: #34c759;
  fill: none;
  stroke: currentColor;
  stroke-width: 2.5;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 160ms ease,
    transform 160ms ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
