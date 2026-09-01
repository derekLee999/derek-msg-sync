<script setup lang="ts">
import { computed, onMounted, onUnmounted, shallowRef } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import AppConfirmDialog from './components/AppConfirmDialog.vue'
import AppToast from './components/AppToast.vue'
import MessageList from './components/MessageList.vue'
import SetupPanel from './components/SetupPanel.vue'
import StatusBar from './components/StatusBar.vue'
import { useMessages } from './composables/useMessages'
import { useToast } from './composables/useToast'
import type { NotificationMode, NotificationPosition, PlatformInfo } from './types'

const {
  visibleMessages,
  status,
  loading,
  error,
  lastReceived,
  senderDevices,
  verificationFilterEnabled,
  totalCount,
  visibleTotalCount,
  endpoint,
  clearError,
  clearMessages,
  copyLocalIp,
  copyMessage,
  copyRecent,
  refresh,
  restartReceiverWithPort,
  setDirectPasteEnabled,
  setVerificationFilterEnabled,
  setNotificationMode,
  setNotificationPosition,
  setRelaySettings,
  setSenderDevices,
  testRelayConnection,
  toggleReceiver,
} = useMessages()

const { toastItems, showToast } = useToast()
const setupOpen = shallowRef(false)
const alwaysOnTop = shallowRef(false)
const platform = shallowRef<PlatformInfo | null>(null)
const activeConfirm = shallowRef<'stopReceiver' | 'clearMessages' | 'restartPort' | null>(null)
const pendingPort = shallowRef<number | null>(null)
const appWindow = getCurrentWindow()
let resolveStopConfirm: ((confirmed: boolean) => void) | null = null
let unlistenOpenSettings: UnlistenFn | null = null
const isMacos = computed(() => platform.value?.isMacos ?? false)

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
  await invoke('hide_main_window_command')
}

async function loadPlatform() {
  platform.value = await invoke<PlatformInfo>('platform_info')
  document.body.classList.toggle('macos', platform.value.isMacos)
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

async function handleNotificationModeChange(mode: NotificationMode) {
  try {
    await setNotificationMode(mode)
    const messages: Record<NotificationMode, string> = {
      all: '通知模式：全部',
      verification: '通知模式：验证码',
      off: '通知已关闭',
    }
    showToast(messages[mode])
  } catch {
    showToast('通知设置保存失败')
  }
}

async function handleNotificationPositionChange(position: NotificationPosition) {
  try {
    await setNotificationPosition(position)
    showToast('通知位置已更新')
  } catch {
    showToast('通知位置保存失败')
  }
}

async function handleDirectPasteChange(enabled: boolean) {
  try {
    await setDirectPasteEnabled(enabled)
    showToast(enabled ? '直接输入已开启' : '直接输入已关闭')
  } catch (cause) {
    showToast(String(cause) || '直接输入设置保存失败')
  }
}

async function handleSenderDevicesChange(devices: typeof senderDevices.value) {
  try {
    await setSenderDevices(devices)
  } catch {
    showToast('设备更新失败')
  }
}

async function handleRelaySettingsChange(relay: { enabled: boolean; baseUrl: string; secret: string }) {
  try {
    await setRelaySettings(relay)
    showToast(relay.enabled ? '云端接入已开启' : '云端接入已关闭')
  } catch {
    showToast('云端接入设置失败')
  }
}

async function handleRelayConnectionTest(relay: { enabled: boolean; baseUrl: string; secret: string }) {
  try {
    await testRelayConnection(relay)
    showToast('云端服务连接正常')
  } catch {
    showToast('云端服务连接失败')
  }
}

onMounted(async () => {
  await loadPlatform()
  unlistenOpenSettings = await listen('open-settings', () => {
    setupOpen.value = true
  })
})

onUnmounted(() => {
  unlistenOpenSettings?.()
  document.body.classList.remove('macos')
})
</script>

<template>
  <div class="window-frame">
    <header :class="['titlebar', { macos: isMacos }]" @mousedown="startDrag">
      <div v-if="!isMacos" class="titlebar-title">
        <img class="titlebar-app-icon" src="/app-icon.png" alt="" aria-hidden="true" />
        <span>验证码接收器</span>
        <button type="button" class="titlebar-icon" title="接入设置" @click="setupOpen = true">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 8.4a3.6 3.6 0 1 0 0 7.2 3.6 3.6 0 0 0 0-7.2Z" />
            <path d="M19.4 13.2a7.9 7.9 0 0 0 0-2.4l2-1.5-2-3.4-2.4 1a8.5 8.5 0 0 0-2.1-1.2L14.6 3h-5.2l-.3 2.7A8.5 8.5 0 0 0 7 6.9l-2.4-1-2 3.4 2 1.5a7.9 7.9 0 0 0 0 2.4l-2 1.5 2 3.4 2.4-1a8.5 8.5 0 0 0 2.1 1.2l.3 2.7h5.2l.3-2.7a8.5 8.5 0 0 0 2.1-1.2l2.4 1 2-3.4-2-1.5Z" />
          </svg>
        </button>
      </div>
      <div v-if="!isMacos" class="window-controls">
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

      <div v-if="error" class="error-banner">
        <span>{{ error }}</span>
        <button type="button" title="关闭错误提示" aria-label="关闭错误提示" @click="clearError">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="m7 7 10 10M17 7 7 17" />
          </svg>
        </button>
      </div>

      <MessageList
        :messages="visibleMessages"
        :loading="loading"
        :verification-filter-enabled="verificationFilterEnabled"
        :total-count="totalCount"
        :visible-count="visibleTotalCount"
        @clear="openClearConfirm"
        @copy="copyMessage"
        @toggle-verification-filter="setVerificationFilterEnabled"
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
            :sender-devices="senderDevices"
            :notification-mode="status?.notificationMode ?? 'verification'"
            :notification-position="status?.notificationPosition ?? 'bottomRight'"
            :direct-paste-enabled="status?.directPasteEnabled ?? false"
            :is-macos="isMacos"
            :relay-enabled="status?.relayEnabled ?? false"
            :relay-running="status?.relayRunning ?? false"
            :relay-base-url="status?.relayBaseUrl ?? ''"
            :relay-secret="status?.relaySecret ?? ''"
            :port="status?.port ?? 17866"
            @update-sender-devices="handleSenderDevicesChange"
            @set-notification-mode="handleNotificationModeChange"
            @set-notification-position="handleNotificationPositionChange"
            @set-direct-paste-enabled="handleDirectPasteChange"
            @set-relay-settings="handleRelaySettingsChange"
            @test-relay-connection="handleRelayConnectionTest"
            @show-toast="showToast"
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
  grid-template-rows: 44px minmax(0, 1fr);
  background: transparent;
}

body.macos .window-frame {
  grid-template-rows: 0 minmax(0, 1fr);
}

.titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 0 12px 0 16px;
  border-bottom: 1px solid var(--glass-border-light);
  background: var(--glass-bg-medium);
  backdrop-filter: var(--glass-blur);
  user-select: none;
}

.titlebar.macos {
  -webkit-app-region: drag;
  justify-content: flex-start;
  height: 0;
  min-height: 0;
  overflow: hidden;
  padding: 0;
  border-bottom: 0;
  background: transparent;
  backdrop-filter: none;
}

.titlebar-title,
.window-controls {
  display: flex;
  align-items: center;
  gap: 6px;
}

.titlebar-title {
  min-width: 0;
  color: var(--text-primary);
  font-size: 15px;
  font-weight: 600;
  letter-spacing: 0.3px;
}

.titlebar-app-icon {
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
  border-radius: 4px;
  object-fit: contain;
}

.titlebar-icon {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: var(--glass-radius-small);
  color: var(--text-secondary);
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
  color: var(--accent-blue);
  background: var(--glass-bg-light);
}

.titlebar-icon.close:hover {
  color: #ff3b30;
  background: rgba(255, 59, 48, 0.1);
}

.app-shell {
  min-height: 0;
  overflow: hidden;
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr);
  gap: 12px;
  padding: 16px;
  background: transparent;
}

body.macos .app-shell {
  padding-top: 10px;
}

.error-banner {
  margin: 0;
  padding: 12px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  border: 1px solid rgba(255, 59, 48, 0.3);
  border-radius: var(--glass-radius-medium);
  color: #ff3b30;
  background: rgba(255, 59, 48, 0.12);
  backdrop-filter: var(--glass-blur);
  font-size: 14px;
  font-weight: 500;
  box-shadow: 0 4px 12px rgba(255, 59, 48, 0.1);
}

.error-banner span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.error-banner button {
  width: 28px;
  height: 28px;
  flex: 0 0 auto;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: var(--glass-radius-small);
  color: #ff3b30;
  background: rgba(255, 59, 48, 0.1);
  cursor: pointer;
}

.error-banner button:hover {
  background: rgba(255, 59, 48, 0.2);
}

.error-banner svg {
  width: 15px;
  height: 15px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2.2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: blur(12px);
}

.modal-window {
  width: min(640px, calc(100vw - 40px));
  max-height: calc(100vh - 40px);
  overflow: hidden;
  border: 1px solid var(--glass-border);
  border-radius: var(--glass-radius-large);
  background: var(--glass-bg-heavy);
  backdrop-filter: var(--glass-blur);
  box-shadow: var(--glass-shadow-heavy);
}

.modal-titlebar {
  height: 54px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 0 16px 0 20px;
  border-bottom: 1px solid var(--glass-border-light);
  color: var(--text-primary);
  background: transparent;
  font-size: 16px;
  font-weight: 600;
  letter-spacing: 0.3px;
}

.modal-content {
  max-height: calc(100vh - 80px);
  overflow: auto;
}

.modal-close {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: var(--glass-radius-small);
  color: var(--text-secondary);
  background: var(--glass-bg-light);
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
  color: var(--text-primary);
  background: var(--glass-bg-medium);
}
</style>
