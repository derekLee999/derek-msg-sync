<script setup lang="ts">
import type { ReceiverStatus } from '../types'

defineProps<{
  status: ReceiverStatus | null
  lastReceived: string
}>()

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
      :class="['status-item', 'receiver-button', { stopped: status && !status.receiverRunning }]"
      :title="status?.receiverRunning === false ? '启动监听服务' : '停止监听服务'"
      @click="emit('toggleReceiver')"
    >
      <strong>{{ status?.receiverRunning === false ? '未启动' : '监听中' }}</strong>
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
      <strong>{{ lastReceived || '等待中...' }}</strong>
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
  min-height: 40px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border: 1px solid rgba(28, 39, 54, 0.09);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.76);
  box-shadow: 0 10px 24px rgba(28, 39, 54, 0.06);
}

.status-item > div {
  min-width: 0;
}

.receiver-button {
  justify-content: center;
  border-color: rgba(32, 180, 134, 0.42);
  color: #0b7a56;
  background: rgba(32, 180, 134, 0.12);
  cursor: pointer;
  font: inherit;
  text-align: center;
}

.receiver-button strong {
  color: #0b7a56;
  font-size: 16px;
}

.receiver-button:hover {
  border-color: rgba(32, 180, 134, 0.7);
  background: rgba(32, 180, 134, 0.18);
  box-shadow:
    0 0 0 3px rgba(32, 180, 134, 0.12),
    0 12px 24px rgba(32, 180, 134, 0.12);
}

.receiver-button.stopped {
  border-color: rgba(102, 112, 133, 0.28);
  color: #667085;
  background: #f2f4f7;
}

.receiver-button.stopped strong {
  color: #667085;
}

.status-action {
  width: 100%;
  border: 1px solid rgba(28, 39, 54, 0.09);
  color: inherit;
  cursor: pointer;
  font: inherit;
  text-align: left;
  transition:
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    transform 0.16s ease,
    background-color 0.16s ease;
}

.status-action:disabled {
  cursor: default;
}

.status-action:not(:disabled):hover {
  border-color: rgba(23, 105, 224, 0.58);
  background: #f4f8ff;
  box-shadow:
    0 0 0 3px rgba(23, 105, 224, 0.1),
    0 14px 28px rgba(23, 105, 224, 0.16);
  transform: translateY(-1px);
}

.status-action:not(:disabled):hover .status-label {
  color: #1769e0;
}

.status-action:not(:disabled):hover strong {
  color: #0f4fb5;
}

.status-label {
  margin: 0 0 1px;
  color: #667085;
  font-size: 12px;
}

strong {
  display: block;
  min-width: 0;
  overflow: hidden;
  color: #17202f;
  font-size: 15px;
  line-height: 19px;
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
  min-height: 40px;
  display: grid;
  place-items: center;
  border: 1px solid rgba(28, 39, 54, 0.09);
  border-radius: 8px;
  color: #1769e0;
  background: #e8f1ff;
  cursor: pointer;
  box-shadow: 0 10px 24px rgba(28, 39, 54, 0.06);
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
