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
  border-radius: var(--glass-radius-large);
  border: 1px solid rgba(255, 255, 255, 0.6);
  color: var(--text-primary);
  box-shadow: 0 24px 48px rgba(0, 0, 0, 0.16);
  z-index: 1; /* Create stacking context for ::before */
}

.notification-card::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: -1;
  border-radius: inherit;
  
  /* 终极修复 Chromium 下 backdrop-filter 圆角溢出导致的四个角背景突出 Bug */
  -webkit-mask-image: -webkit-radial-gradient(white, black);
  mask-image: radial-gradient(white, black);
  overflow: hidden;
  
  background: rgba(255, 255, 255, 0.7);
  background-image: linear-gradient(135deg, rgba(255, 255, 255, 0.9) 0%, rgba(255, 255, 255, 0.4) 100%);
  backdrop-filter: blur(32px) saturate(180%);
  box-shadow: 
    inset 0 1px 0 rgba(255, 255, 255, 0.8),
    inset 1px 0 0 rgba(255, 255, 255, 0.4);
}

.close-button {
  position: absolute;
  top: 10px;
  right: 10px;
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: var(--glass-radius-small);
  color: var(--text-secondary);
  background: var(--glass-bg-light);
  cursor: pointer;
}

.close-button:hover {
  color: var(--text-primary);
  background: var(--glass-bg-medium);
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
  color: #1e293b;
  font-size: 14px;
  font-weight: 700;
  line-height: 18px;
  letter-spacing: 0.3px;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-shadow: 0 1px 2px rgba(255, 255, 255, 0.8);
}

.notification-head span {
  flex: 0 0 auto;
  color: #475569;
  font-size: 12px;
  font-weight: 500;
  text-shadow: 0 1px 1px rgba(255, 255, 255, 0.6);
}

.notification-text {
  margin: 10px 0 6px;
  color: #0f172a;
  font-size: 18px;
  font-weight: 800;
  line-height: 24px;
  letter-spacing: 0.2px;
  text-shadow: 0 2px 4px rgba(255, 255, 255, 0.8);
}

.code-text {
  display: inline-block;
  color: #0066cc;
  background: rgba(0, 102, 204, 0.12);
  border: 1px solid rgba(0, 102, 204, 0.2);
  padding: 2px 10px;
  border-radius: 8px;
  font-size: 20px;
  font-weight: 800;
  letter-spacing: 1px;
  vertical-align: bottom;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.5);
  text-shadow: none;
}

.notification-message {
  display: -webkit-box;
  overflow: hidden;
  margin: 0;
  color: #334155;
  font-size: 13px;
  line-height: 1.5;
  font-weight: 500;
  text-shadow: 0 1px 2px rgba(255, 255, 255, 0.6);
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
