<script setup lang="ts">
import { computed } from 'vue'
import type { IncomingMessage } from '../types'

const props = defineProps<{
  messages: IncomingMessage[]
  loading: boolean
  totalCount: number
  visibleCount: number
  verificationFilterEnabled: boolean
}>()

const emit = defineEmits<{
  clear: []
  copy: [message: IncomingMessage]
  toggleVerificationFilter: [enabled: boolean]
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
        <h2 class="panel-title">
          消息收件台
          <span>{{ verificationFilterEnabled ? visibleCount : totalCount }}</span>
        </h2>
      </div>
      <div class="panel-actions">
        <button
          type="button"
          :class="['filter-toggle', { active: verificationFilterEnabled }]"
          role="switch"
          :aria-checked="verificationFilterEnabled"
          title="开启后仅显示包含验证码和校验码关键字的消息"
          @click="emit('toggleVerificationFilter', !verificationFilterEnabled)"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 5h16l-6 7v5l-4 2v-7L4 5Z" />
          </svg>
          {{ verificationFilterEnabled ? '验证码' : '全部' }}
        </button>
        <button type="button" :disabled="totalCount === 0" title="清空消息" @click="emit('clear')">清空</button>
      </div>
    </div>

    <div v-if="loading" class="empty-state">正在连接本地接收服务...</div>

    <div v-else-if="!hasMessages" class="empty-state">
      <strong>{{ totalCount > 0 ? '没有匹配的验证码消息' : '等待第一条 iPhone 消息' }}</strong>
      <span>
        {{ totalCount > 0 ? '关闭筛选可查看全部消息。' : '收到短信后，验证码会立即进入 Windows 剪切板。' }}
      </span>
    </div>

    <div v-else class="message-list">
      <article v-for="message in messages" :key="message.id" :class="['message-item', { plain: !message.code }]">
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
        <button v-if="message.code" type="button" class="code-button" title="再次复制" @click="emit('copy', message)">
          <span>{{ message.code }}</span>
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
  padding: 16px;
  border: 1px solid var(--glass-border);
  border-radius: var(--glass-radius-large);
  background: var(--glass-bg-medium);
  backdrop-filter: var(--glass-blur);
  box-shadow: var(--glass-shadow-heavy);
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
  color: var(--text-primary);
  font-size: 18px;
  line-height: 24px;
  font-weight: 600;
  letter-spacing: 0.2px;
}

.panel-title span {
  display: inline-grid;
  place-items: center;
  min-width: 22px;
  height: 20px;
  margin-left: 6px;
  padding: 0 7px;
  border-radius: 999px;
  color: var(--accent-blue);
  background: rgba(0, 122, 255, 0.1);
  font-size: 13px;
  font-weight: 700;
  vertical-align: 2px;
}

.panel-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

button {
  height: 34px;
  border: 0;
  border-radius: var(--glass-radius-small);
  color: var(--text-primary);
  background: var(--glass-bg-heavy);
  cursor: pointer;
  font: inherit;
  font-weight: 500;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
}

button:disabled {
  color: var(--text-secondary);
  background: var(--glass-bg-light);
  cursor: default;
  opacity: 0.5;
  box-shadow: none;
}

.panel-head button {
  padding: 0 13px;
}

.filter-toggle {
  min-width: 84px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  color: var(--text-secondary);
  background: var(--glass-bg-heavy);
  font-weight: 600;
}

.filter-toggle svg {
  width: 14px;
  height: 14px;
  flex: 0 0 auto;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.filter-toggle:hover {
  color: var(--accent-blue);
}

.filter-toggle.active {
  color: #ffffff;
  background: var(--accent-gradient);
  border: 0;
}

.empty-state {
  min-height: 230px;
  display: grid;
  place-content: center;
  gap: 8px;
  color: var(--text-secondary);
  text-align: center;
}

.empty-state strong {
  color: var(--text-primary);
  font-size: 18px;
  font-weight: 600;
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
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid var(--glass-border);
  border-radius: var(--glass-radius-medium);
  background: var(--glass-bg-heavy);
  backdrop-filter: var(--glass-blur);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.04);
}

.message-item.plain {
  grid-template-columns: minmax(0, 1fr);
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
  color: var(--text-secondary);
  font-size: 12px;
}

.message-text {
  display: -webkit-box;
  overflow: hidden;
  margin: 0 0 6px;
  color: var(--text-primary);
  font-size: 15px;
  font-weight: 600;
  line-height: 1.4;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.message-text mark {
  padding: 2px 6px;
  border-radius: 6px;
  color: var(--accent-blue-hover);
  background: rgba(0, 122, 255, 0.15);
  font-weight: 700;
}

.meta-row span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.code-button {
  width: 100%;
  height: auto;
  min-height: 48px;
  padding: 6px 8px;
  border: 0;
  border-radius: var(--glass-radius-small);
  color: #ffffff;
  background: var(--accent-gradient);
  box-shadow: 0 8px 24px rgba(0, 122, 255, 0.3);
  transition:
    background 0.2s ease,
    box-shadow 0.2s ease,
    transform 0.2s cubic-bezier(0.25, 1, 0.5, 1);
}

.code-button:hover {
  filter: brightness(1.1);
  box-shadow: 0 12px 32px rgba(0, 122, 255, 0.4);
  transform: translateY(-2px) scale(1.02);
}

.code-button:active {
  box-shadow: 0 4px 16px rgba(0, 122, 255, 0.2);
  transform: translateY(0) scale(0.98);
}

.code-button:focus-visible {
  outline: 3px solid rgba(23, 105, 224, 0.22);
  outline-offset: 2px;
}

.code-button span {
  display: block;
  overflow: hidden;
  font-size: 20px;
  font-weight: 700;
  letter-spacing: 1px;
  line-height: 24px;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: transform 0.2s ease;
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
