<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { onMounted, onUnmounted, shallowRef } from 'vue'
import type { IncomingMessage } from '../types'

const NOTIFICATION_SOUND = '/notification-pluck-off-269290.mp3'

const visible = shallowRef(false)
const message = shallowRef<IncomingMessage | null>(null)
let hideTimer: number | null = null
let unlistenMessage: (() => void) | null = null
let notificationAudio: HTMLAudioElement | null = null
const appWindow = getCurrentWindow()

function clearHideTimer() {
  if (hideTimer !== null) {
    window.clearTimeout(hideTimer)
    hideTimer = null
  }
}

function scheduleHide(delay: number) {
  clearHideTimer()
  hideTimer = window.setTimeout(() => {
    hideTimer = null
    hideToast()
  }, delay)
}

async function hideToast() {
  clearHideTimer()
  visible.value = false
  await window.setTimeout(() => {
    appWindow.hide()
  }, 180)
}

function showToast(nextMessage: IncomingMessage) {
  message.value = nextMessage
  visible.value = true
  playNotificationSound()
  scheduleHide(3000)
}

function playNotificationSound() {
  if (!notificationAudio) {
    notificationAudio = new Audio(NOTIFICATION_SOUND)
    notificationAudio.preload = 'auto'
  }

  notificationAudio.currentTime = 0
  notificationAudio.play().catch(() => {
    // WebView audio policy may block playback in rare cases; keep the toast visible.
  })
}

function handleMouseEnter() {
  clearHideTimer()
}

function handleMouseLeave() {
  scheduleHide(1000)
}

onMounted(async () => {
  unlistenMessage = await listen<IncomingMessage>('notification-message', (event) => {
    showToast(event.payload)
  })
  document.body.classList.add('notification-window')
})

onUnmounted(() => {
  clearHideTimer()
  unlistenMessage?.()
  document.body.classList.remove('notification-window')
})
</script>

<template>
  <main class="notification-host">
    <Transition name="toast">
      <section
        v-if="visible && message"
        class="notification-card"
        @mouseenter="handleMouseEnter"
        @mouseleave="handleMouseLeave"
      >
        <button type="button" class="close-button" title="关闭通知" @click="hideToast">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="m7 7 10 10M17 7 7 17" />
          </svg>
        </button>
        <div class="notification-content">
          <div class="notification-head">
            <div class="notification-title">
              <img src="/app-icon.png" alt="" />
              <strong>验证码接收器</strong>
            </div>
            <span>{{ message.sender || 'iPhone' }}</span>
          </div>
          <p class="notification-text">
            <template v-if="message.code">
              验证码 <span class="code-text">{{ message.code }}</span> 已复制
            </template>
            <template v-else>消息内容已复制到剪切板</template>
          </p>
          <p class="notification-message">{{ message.text || message.copiedText }}</p>
        </div>
      </section>
    </Transition>
  </main>
</template>

<style scoped>
.notification-host {
  width: 100vw;
  height: 100vh;
  display: grid;
  place-items: center;
  overflow: hidden;
  padding: 0;
  background: transparent;
}

.notification-card {
  position: relative;
  width: 380px;
  min-height: 116px;
  display: block;
  padding: 16px 42px 16px 16px;
  border: 1px solid rgba(28, 39, 54, 0.12);
  border-radius: 8px;
  color: #17202f;
  background: rgba(255, 255, 255, 0.96);
  box-shadow:
    0 18px 46px rgba(28, 39, 54, 0.22),
    0 2px 8px rgba(28, 39, 54, 0.1);
}

.close-button {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: 6px;
  color: #667085;
  background: transparent;
  cursor: pointer;
}

.close-button:hover {
  color: #17202f;
  background: #eef3f8;
}

.close-button svg {
  width: 15px;
  height: 15px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2.2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.notification-head {
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding-right: 4px;
}

.notification-title {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.notification-title img {
  width: 14px;
  height: 14px;
  flex: 0 0 auto;
}

.notification-title strong {
  overflow: hidden;
  color: #17202f;
  font-size: 13px;
  line-height: 18px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.notification-head span {
  flex: 0 0 auto;
  color: #667085;
  font-size: 12px;
}

.notification-text {
  margin: 10px 0 3px;
  color: #17202f;
  font-size: 18px;
  font-weight: 800;
  line-height: 24px;
}

.code-text {
  color: #1769e0;
}

.notification-message {
  display: -webkit-box;
  overflow: hidden;
  margin: 0;
  color: #465160;
  font-size: 12px;
  line-height: 17px;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 180ms ease,
    transform 180ms ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(16px);
}
</style>
