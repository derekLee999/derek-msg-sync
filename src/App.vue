<script setup lang="ts">
import { shallowRef } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import AppConfirmDialog from './components/AppConfirmDialog.vue'
import AppToast from './components/AppToast.vue'
import MessageList from './components/MessageList.vue'
import SetupPanel from './components/SetupPanel.vue'
import StatusBar from './components/StatusBar.vue'
import { useMessages } from './composables/useMessages'
import { useToast } from './composables/useToast'

const {
  messages,
  status,
  loading,
  error,
  lastReceived,
  token,
  defaultSender,
  totalCount,
  endpoint,
  isTokenEnabled,
  clearMessages,
  copyLocalIp,
  copyMessage,
  copyRecent,
  refresh,
  restartReceiverWithPort,
  saveDefaultSender,
  saveToken,
  setNotificationEnabled,
  toggleReceiver,
} = useMessages()

const { toastItems, showToast } = useToast()
const setupOpen = shallowRef(false)
const alwaysOnTop = shallowRef(false)
const activeConfirm = shallowRef<'stopReceiver' | 'clearMessages' | 'restartPort' | null>(null)
const pendingPort = shallowRef<number | null>(null)
const appWindow = getCurrentWindow()
let resolveStopConfirm: ((confirmed: boolean) => void) | null = null

async function startDrag(event: MouseEvent) {
  if ((event.target as HTMLElement).closest('button')) {
    return
  }

  await appWindow.startDragging()
}

async function toggleAlwaysOnTop() {
  alwaysOnTop.value = !alwaysOnTop.value
  await appWindow.setAlwaysOnTop(alwaysOnTop.value)
}

async function minimizeWindow() {
  await appWindow.minimize()
}

async function hideWindow() {
  await appWindow.hide()
}

async function handleCopyLocalIp() {
  if (!status.value?.localIp) {
    return
  }

  await copyLocalIp()
  showToast('本机地址已复制')
}

async function handleCopyRecent() {
  if (!lastReceived.value) {
    return
  }

  await copyRecent()
  showToast('最近接收已复制')
}

function confirmStopReceiver() {
  activeConfirm.value = 'stopReceiver'

  return new Promise<boolean>((resolve) => {
    resolveStopConfirm = resolve
  })
}

function settleStopConfirm(confirmed: boolean) {
  activeConfirm.value = null
  resolveStopConfirm?.(confirmed)
  resolveStopConfirm = null
}

async function handleToggleReceiver() {
  await toggleReceiver({
    confirmStop: confirmStopReceiver,
  })
}

function openClearConfirm() {
  activeConfirm.value = 'clearMessages'
}

function closeClearConfirm() {
  activeConfirm.value = null
}

async function confirmClearMessages() {
  activeConfirm.value = null
  await clearMessages()
}

function requestPortChange(port: number) {
  pendingPort.value = port
  activeConfirm.value = 'restartPort'
}

function closePortConfirm() {
  pendingPort.value = null
  activeConfirm.value = null
}

async function confirmPortChange() {
  const port = pendingPort.value
  closePortConfirm()
  if (!port) {
    return
  }

  try {
    await restartReceiverWithPort(port)
    showToast('端口已更新，接收服务已重启')
  } catch {
    showToast('端口更新失败，请检查端口占用')
  }
}

async function handleNotificationChange(enabled: boolean) {
  try {
    await setNotificationEnabled(enabled)
    showToast(enabled ? '通知已开启' : '通知已关闭')
  } catch {
    showToast('通知设置保存失败')
  }
}
</script>

<template>
  <div class="window-frame">
    <header class="titlebar" @mousedown="startDrag">
      <div class="titlebar-title">
        <img class="titlebar-app-icon" src="/app-icon.png" alt="" aria-hidden="true" />
        <span>验证码接收器</span>
        <button type="button" class="titlebar-icon" title="接入设置" @click="setupOpen = true">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 8.4a3.6 3.6 0 1 0 0 7.2 3.6 3.6 0 0 0 0-7.2Z" />
            <path d="M19.4 13.2a7.9 7.9 0 0 0 0-2.4l2-1.5-2-3.4-2.4 1a8.5 8.5 0 0 0-2.1-1.2L14.6 3h-5.2l-.3 2.7A8.5 8.5 0 0 0 7 6.9l-2.4-1-2 3.4 2 1.5a7.9 7.9 0 0 0 0 2.4l-2 1.5 2 3.4 2.4-1a8.5 8.5 0 0 0 2.1 1.2l.3 2.7h5.2l.3-2.7a8.5 8.5 0 0 0 2.1-1.2l2.4 1 2-3.4-2-1.5Z" />
          </svg>
        </button>
      </div>
      <div class="window-controls">
        <button
          type="button"
          :class="['titlebar-icon', { active: alwaysOnTop }]"
          title="窗口置顶"
          @click="toggleAlwaysOnTop"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M7 3h10l-1 6 3 3v2H5v-2l3-3-1-6Z" />
            <path d="M12 14v7" />
          </svg>
        </button>
        <button type="button" class="titlebar-icon" title="最小化" @click="minimizeWindow">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M5 12h14" />
          </svg>
        </button>
        <button type="button" class="titlebar-icon close" title="隐藏到托盘" @click="hideWindow">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="m7 7 10 10M17 7 7 17" />
          </svg>
        </button>
      </div>
    </header>

    <main class="app-shell">
      <StatusBar
        :status="status"
        :last-received="lastReceived"
        @copy-ip="handleCopyLocalIp"
        @copy-recent="handleCopyRecent"
        @refresh="refresh"
        @toggle-receiver="handleToggleReceiver"
      />

      <p v-if="error" class="error-banner">{{ error }}</p>

      <MessageList
        :messages="messages"
        :loading="loading"
        :total-count="totalCount"
        @clear="openClearConfirm"
        @copy="copyMessage"
      />
    </main>

    <Teleport to="body">
      <div v-if="setupOpen" class="modal-backdrop" @click.self="setupOpen = false">
        <div class="modal-window">
          <header class="modal-titlebar">
            <strong>接入设置</strong>
            <button type="button" class="modal-close" title="关闭" @click="setupOpen = false">
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="m7 7 10 10M17 7 7 17" />
              </svg>
            </button>
          </header>
          <div class="modal-content">
          <SetupPanel
            :endpoint="endpoint"
            :default-sender="defaultSender"
            :token="token"
            :is-token-enabled="isTokenEnabled"
            :notification-enabled="status?.notificationEnabled ?? true"
            :port="status?.port ?? 17866"
            @save-default-sender="saveDefaultSender"
            @save-token="saveToken"
            @update-default-sender="defaultSender = $event"
            @set-notification-enabled="handleNotificationChange"
            @update-token="token = $event"
            @request-port-change="requestPortChange"
          />
          </div>
        </div>
      </div>
    </Teleport>

    <AppConfirmDialog
      v-if="activeConfirm === 'stopReceiver'"
      title="确认停止接收服务"
      message="停止监听后，iPhone 消息将无法发送到此电脑。确定停止接收服务吗？"
      confirm-text="停止监听"
      cancel-text="取消"
      @confirm="settleStopConfirm(true)"
      @cancel="settleStopConfirm(false)"
    />

    <AppConfirmDialog
      v-if="activeConfirm === 'clearMessages'"
      title="确认清空消息"
      message="清空后，当前消息收件台中的记录将被删除。确定清空吗？"
      confirm-text="清空"
      cancel-text="取消"
      @confirm="confirmClearMessages"
      @cancel="closeClearConfirm"
    />

    <AppConfirmDialog
      v-if="activeConfirm === 'restartPort'"
      title="确认重启接收服务"
      message="端口修改后需要重启接收服务，重启期间 iPhone 消息可能短暂无法发送。确定使用新端口并重启吗？"
      confirm-text="重启服务"
      cancel-text="取消"
      @confirm="confirmPortChange"
      @cancel="closePortConfirm"
    />

    <AppToast :items="toastItems" />
  </div>
</template>

<style scoped>
.window-frame {
  width: 100%;
  height: 100%;
  overflow: hidden;
  display: grid;
  grid-template-rows: 40px minmax(0, 1fr);
  background: #eef3f8;
}

.titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 0 8px 0 14px;
  border-bottom: 1px solid rgba(28, 39, 54, 0.08);
  background: rgba(245, 248, 252, 0.92);
  user-select: none;
}

.titlebar-title,
.window-controls {
  display: flex;
  align-items: center;
  gap: 6px;
}

.titlebar-title {
  min-width: 0;
  color: #17202f;
  font-size: 14px;
  font-weight: 700;
}

.titlebar-app-icon {
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
  border-radius: 4px;
  object-fit: contain;
}

.titlebar-icon {
  width: 30px;
  height: 30px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: 7px;
  color: #465160;
  background: transparent;
  cursor: pointer;
}

.titlebar-icon svg {
  width: 17px;
  height: 17px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.titlebar-icon:hover,
.titlebar-icon.active {
  color: #1769e0;
  background: #e8f1ff;
}

.titlebar-icon.close:hover {
  color: #b42318;
  background: #fee4e2;
}

.app-shell {
  min-height: 0;
  overflow: hidden;
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr);
  gap: 10px;
  padding: 16px;
  background:
    linear-gradient(135deg, rgba(23, 105, 224, 0.08), transparent 34%),
    linear-gradient(180deg, #f4f7fb 0%, #eef3f8 100%);
}

.error-banner {
  margin: 0;
  padding: 10px 12px;
  border: 1px solid #ffd8d6;
  border-radius: 8px;
  color: #a63932;
  background: #fff1f0;
  font-size: 13px;
}

.modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(15, 23, 42, 0.42);
}

.modal-window {
  width: min(520px, calc(100vw - 40px));
  max-height: calc(100vh - 40px);
  overflow: hidden;
  border: 1px solid rgba(28, 39, 54, 0.12);
  border-radius: 10px;
  background: #ffffff;
  box-shadow: 0 24px 70px rgba(28, 39, 54, 0.26);
}

.modal-titlebar {
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 0 8px 0 14px;
  border-bottom: 1px solid rgba(28, 39, 54, 0.08);
  color: #17202f;
  background: #f8fafc;
  font-size: 14px;
  font-weight: 700;
}

.modal-content {
  max-height: calc(100vh - 80px);
  overflow: auto;
}

.modal-close {
  width: 30px;
  height: 30px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: 7px;
  color: #465160;
  background: transparent;
  cursor: pointer;
}

.modal-close svg {
  width: 17px;
  height: 17px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.modal-close:hover {
  color: #b42318;
  background: #fee4e2;
}
</style>
