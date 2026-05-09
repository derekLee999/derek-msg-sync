<script setup lang="ts">
import { computed } from 'vue'
import type { IncomingMessage } from '../types'

const props = defineProps<{
  messages: IncomingMessage[]
  loading: boolean
  totalCount: number
}>()

const emit = defineEmits<{
  clear: []
  copy: [message: IncomingMessage]
}>()

const hasMessages = computed(() => props.messages.length > 0)

function formatTime(value: string) {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  return new Intl.DateTimeFormat('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    month: '2-digit',
    day: '2-digit',
  }).format(date)
}

function formatReceivedAt(value: string) {
  const time = formatTime(value)
  return `${time}`
}

function highlightText(message: IncomingMessage) {
  const text = message.text || message.copiedText
  const code = message.code
  if (!code) {
    return [{ text, highlight: false }]
  }

  const index = text.indexOf(code)
  if (index < 0) {
    return [{ text, highlight: false }]
  }

  return [
    { text: text.slice(0, index), highlight: false },
    { text: code, highlight: true },
    { text: text.slice(index + code.length), highlight: false },
  ].filter((part) => part.text)
}
</script>

<template>
  <section class="message-panel">
    <div class="panel-head">
      <div>
        <h2 class="panel-title">消息收件台 <span>{{ totalCount }}</span></h2>
      </div>
      <button type="button" :disabled="!hasMessages" title="清空消息" @click="emit('clear')">清空</button>
    </div>

    <div v-if="loading" class="empty-state">正在连接本地接收服务...</div>

    <div v-else-if="!hasMessages" class="empty-state">
      <strong>等待第一条 iPhone 消息</strong>
      <span>收到短信后，验证码会立即进入 Windows 剪切板。</span>
    </div>

    <div v-else class="message-list">
      <article v-for="message in messages" :key="message.id" class="message-item">
        <div class="message-main">
          <p class="message-text">
            <template v-for="part in highlightText(message)" :key="part.text">
              <mark v-if="part.highlight">{{ part.text }}</mark>
              <span v-else>{{ part.text }}</span>
            </template>
          </p>
          <div class="meta-row">
            <span>{{ formatReceivedAt(message.receivedAt) }}</span>
            <span>{{ message.sender || 'iPhone' }}</span>
          </div>
        </div>
        <button type="button" class="code-button" title="再次复制" @click="emit('copy', message)">
          <span>{{ message.code ?? message.copiedText }}</span>
        </button>
      </article>
    </div>
  </section>
</template>

<style scoped>
.message-panel {
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  padding: 14px;
  border: 1px solid rgba(28, 39, 54, 0.09);
  border-radius: 8px;
  background: #ffffff;
  box-shadow: 0 18px 48px rgba(28, 39, 54, 0.08);
}

.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  margin-bottom: 10px;
}

.panel-title {
  margin: 0;
  color: #17202f;
  font-size: 18px;
  line-height: 24px;
}

.panel-title span {
  display: inline-grid;
  place-items: center;
  min-width: 22px;
  height: 20px;
  margin-left: 6px;
  padding: 0 7px;
  border-radius: 999px;
  color: #1769e0;
  background: #e8f1ff;
  font-size: 13px;
  font-weight: 800;
  vertical-align: 2px;
}

button {
  height: 34px;
  border: 0;
  border-radius: 7px;
  color: #1769e0;
  background: #e8f1ff;
  cursor: pointer;
  font: inherit;
}

button:disabled {
  color: #98a2b3;
  background: #eef1f5;
  cursor: default;
}

.panel-head button {
  padding: 0 13px;
}

.empty-state {
  min-height: 230px;
  display: grid;
  place-content: center;
  gap: 8px;
  color: #667085;
  text-align: center;
}

.empty-state strong {
  color: #17202f;
  font-size: 18px;
}

.message-list {
  flex: 1 1 auto;
  min-height: 0;
  overflow: auto;
  display: grid;
  gap: 6px;
  padding-right: 4px;
}

.message-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(116px, 150px);
  align-items: stretch;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid #e7ebf1;
  border-radius: 8px;
  background: #fbfcfe;
}

.message-main {
  min-width: 0;
}

.meta-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.meta-row {
  color: #667085;
  font-size: 12px;
}

.message-text {
  display: -webkit-box;
  overflow: hidden;
  margin: 0 0 5px;
  color: #17202f;
  font-size: 15px;
  font-weight: 650;
  line-height: 20px;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.message-text mark {
  padding: 1px 4px;
  border-radius: 5px;
  color: #0f4fb5;
  background: #dbeafe;
  font-weight: 800;
}

.meta-row span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.code-button {
  width: 100%;
  height: auto;
  min-height: 46px;
  padding: 6px 8px;
  border: 1px solid rgba(23, 105, 224, 0.12);
  color: #ffffff;
  background: #1769e0;
  box-shadow: 0 8px 18px rgba(23, 105, 224, 0.18);
  transition:
    background 140ms ease,
    border-color 140ms ease,
    box-shadow 140ms ease,
    transform 140ms ease;
}

.code-button:hover {
  border-color: rgba(23, 105, 224, 0.34);
  background: #0f57c4;
  box-shadow: 0 12px 24px rgba(23, 105, 224, 0.28);
  transform: translateY(-1px);
}

.code-button:active {
  background: #0d4cab;
  box-shadow: 0 6px 14px rgba(23, 105, 224, 0.2);
  transform: translateY(0);
}

.code-button:focus-visible {
  outline: 3px solid rgba(23, 105, 224, 0.22);
  outline-offset: 2px;
}

.code-button span {
  display: block;
  overflow: hidden;
  font-size: 17px;
  font-weight: 800;
  line-height: 22px;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: transform 140ms ease;
}

.code-button:hover span {
  transform: scale(1.03);
}

@media (max-width: 760px) {
  .message-item {
    grid-template-columns: minmax(0, 1fr);
  }

  .code-button {
    min-height: 42px;
  }
}
</style>
