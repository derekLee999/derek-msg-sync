<script setup lang="ts">
import { computed } from 'vue'
import type { ReceiverStatus } from '../types'

const props = defineProps<{
  status: ReceiverStatus | null
  lastReceived: string
  receiverAction: 'starting' | 'stopping' | null
}>()

const receiverBusy = computed(() => props.receiverAction !== null)
const receiverButtonTitle = computed(() => {
  if (props.receiverAction === 'stopping') {
    return '正在停止监听服务'
  }
  if (props.receiverAction === 'starting') {
    return '正在启动监听服务'
  }

  return props.status?.receiverRunning === false ? '启动监听服务' : '停止监听服务'
})

const receiverButtonText = computed(() => {
  if (props.receiverAction === 'stopping') {
    return '停止中'
  }
  if (props.receiverAction === 'starting') {
    return '启动中'
  }

  return props.status?.receiverRunning === false ? '未启动' : '监听中'
})

const emit = defineEmits<{
  copyIp: []
  copyRecent: []
  refresh: []
  toggleReceiver: []
}>()
</script>

<template>
  <section class="status-bar">
    <button
      type="button"
      :class="[
        'status-item',
        'receiver-button',
        {
          stopped: status && !status.receiverRunning,
          busy: receiverBusy,
        },
      ]"
      :disabled="receiverBusy"
      :title="receiverButtonTitle"
      @click="emit('toggleReceiver')"
    >
      <span v-if="receiverBusy" class="receiver-spinner" aria-hidden="true"></span>
      <strong>{{ receiverButtonText }}</strong>
    </button>
    <div class="status-item">
      <p class="status-label">端口</p>
      <strong>{{ status?.port ?? 17866 }}</strong>
    </div>
    <button
      type="button"
      class="status-item status-action status-ip"
      :disabled="!status?.localIp"
      title="复制本机 IP 地址"
      @click="emit('copyIp')"
    >
      <p class="status-label">本机地址</p>
      <strong>{{ status?.localIp ?? '检测中' }}</strong>
    </button>
    <button
      type="button"
      class="status-item status-action status-recent"
      :disabled="!lastReceived"
      title="复制最近接收的验证码"
      @click="emit('copyRecent')"
    >
      <p class="status-label">最近接收</p>
      <strong>{{ lastReceived || '--' }}</strong>
    </button>
    <button type="button" class="refresh-icon-button" title="刷新状态" @click="emit('refresh')">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M20 6v5h-5" />
        <path d="M4 18v-5h5" />
        <path d="M18.2 9A7 7 0 0 0 6.7 6.4L4 9" />
        <path d="M5.8 15A7 7 0 0 0 17.3 17.6L20 15" />
      </svg>
    </button>
  </section>
</template>

<style scoped>
.status-bar {
  display: grid;
  grid-template-columns:
    minmax(118px, 0.82fr)
    minmax(82px, 0.52fr)
    minmax(158px, 1.04fr)
    minmax(132px, 0.86fr)
    42px;
  gap: 8px;
}

.status-item {
  min-width: 0;
  min-height: 42px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border: 1px solid var(--glass-border);
  border-radius: var(--glass-radius-small);
  background: var(--glass-bg-medium);
  backdrop-filter: var(--glass-blur);
  box-shadow: var(--glass-shadow);
}

.status-item > div {
  min-width: 0;
}

.receiver-button {
  justify-content: center;
  border-color: rgba(52, 199, 89, 0.4);
  color: #248a3d;
  background: rgba(52, 199, 89, 0.15);
  cursor: pointer;
  font: inherit;
  text-align: center;
  transition:
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    background-color 0.16s ease,
    color 0.16s ease;
}

.receiver-button strong {
  color: #248a3d;
  font-size: 16px;
}

.receiver-button:hover {
  border-color: rgba(52, 199, 89, 0.6);
  background: rgba(52, 199, 89, 0.25);
  box-shadow: 0 8px 24px rgba(52, 199, 89, 0.2);
}

.receiver-button:disabled {
  cursor: wait;
}

.receiver-button.stopped {
  border-color: var(--glass-border-light);
  color: var(--text-secondary);
  background: var(--glass-bg-light);
}

.receiver-button.busy {
  border-color: rgba(23, 105, 224, 0.46);
  color: #1769e0;
  background: #e8f1ff;
  box-shadow:
    0 0 0 3px rgba(23, 105, 224, 0.1),
    0 12px 24px rgba(23, 105, 224, 0.12);
}

.receiver-button.busy strong {
  color: #1769e0;
}

.receiver-spinner {
  width: 14px;
  height: 14px;
  flex: 0 0 auto;
  border: 2px solid rgba(23, 105, 224, 0.22);
  border-top-color: #1769e0;
  border-radius: 50%;
  animation: receiver-spin 0.8s linear infinite;
}

@keyframes receiver-spin {
  to {
    transform: rotate(360deg);
  }
}

.receiver-button.stopped strong {
  color: var(--text-secondary);
}

.status-action {
  width: 100%;
  border: 1px solid var(--glass-border);
  color: inherit;
  cursor: pointer;
  font: inherit;
  text-align: left;
  transition:
    border-color 0.2s ease,
    box-shadow 0.2s ease,
    transform 0.2s cubic-bezier(0.25, 1, 0.5, 1),
    background-color 0.2s ease;
}

.status-action:disabled {
  cursor: default;
  opacity: 0.7;
}

.status-action:not(:disabled):hover {
  border-color: var(--accent-blue);
  background: rgba(0, 122, 255, 0.08);
  box-shadow: 0 8px 24px rgba(0, 122, 255, 0.12);
  transform: translateY(-1px);
}

.status-action:not(:disabled):hover .status-label {
  color: var(--accent-blue);
}

.status-action:not(:disabled):hover strong {
  color: var(--accent-blue-hover);
}

.status-label {
  margin: 0 0 1px;
  color: var(--text-secondary);
  font-size: 12px;
}

strong {
  display: block;
  min-width: 0;
  overflow: hidden;
  color: var(--text-primary);
  font-size: 15px;
  line-height: 19px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-ip strong {
  font-size: 14px;
}

.status-recent strong {
  font-size: 16px;
}

.refresh-icon-button {
  width: 42px;
  min-width: 0;
  min-height: 42px;
  display: grid;
  place-items: center;
  border: 1px solid var(--glass-border);
  border-radius: var(--glass-radius-small);
  color: var(--accent-blue);
  background: var(--glass-bg-medium);
  backdrop-filter: var(--glass-blur);
  cursor: pointer;
  box-shadow: var(--glass-shadow);
}

.refresh-icon-button:hover {
  background: var(--glass-bg-heavy);
  box-shadow: var(--glass-shadow-heavy);
}

.refresh-icon-button svg {
  width: 18px;
  height: 18px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}
</style>
