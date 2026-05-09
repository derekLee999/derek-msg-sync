import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { computed, onMounted, onUnmounted, shallowRef } from 'vue'
import type { IncomingMessage, ReceiverStatus } from '../types'

const MAX_VISIBLE_MESSAGES = 100

interface ToggleReceiverOptions {
  confirmStop?: () => boolean | Promise<boolean>
}

export function useMessages() {
  const messages = shallowRef<IncomingMessage[]>([])
  const status = shallowRef<ReceiverStatus | null>(null)
  const loading = shallowRef(true)
  const error = shallowRef('')
  const lastReceived = shallowRef('')
  const token = shallowRef('')
  const defaultSender = shallowRef('iPhone')

  const latestMessage = computed(() => messages.value[0] ?? null)
  const totalCount = computed(() => messages.value.length)
  const endpoint = computed(() => status.value?.endpoint ?? `http://<Windows局域网IP>:${status.value?.port ?? 17866}/otp`)
  const isTokenEnabled = computed(() => status.value?.tokenRequired ?? false)

  async function refresh() {
    loading.value = true
    error.value = ''
    try {
      const [nextMessages, nextStatus] = await Promise.all([
        invoke<IncomingMessage[]>('get_messages'),
        invoke<ReceiverStatus>('receiver_status'),
      ])
      messages.value = nextMessages
      status.value = nextStatus
      defaultSender.value = nextStatus.defaultSender || 'iPhone'
      lastReceived.value = nextMessages[0]?.copiedText ?? ''
    } catch (cause) {
      error.value = String(cause)
    } finally {
      loading.value = false
    }
  }

  async function clearMessages() {
    await invoke('clear_messages')
    messages.value = []
    await refreshStatus()
  }

  async function copyMessage(message: IncomingMessage) {
    await writeText(message.copiedText)
    lastReceived.value = message.copiedText
  }

  async function copyLocalIp() {
    const ip = status.value?.localIp
    if (!ip) {
      return
    }

    await writeText(ip)
  }

  async function copyRecent() {
    if (!lastReceived.value) {
      return
    }

    await writeText(lastReceived.value)
  }

  async function saveToken() {
    await invoke('set_receiver_token', { token: token.value })
    await refreshStatus()
  }

  async function saveDefaultSender() {
    error.value = ''
    try {
      await invoke('set_default_sender', { sender: defaultSender.value })
      await refreshStatus()
    } catch (cause) {
      error.value = String(cause)
      throw cause
    }
  }

  async function restartReceiverWithPort(port: number) {
    error.value = ''
    try {
      await invoke('set_receiver_port', { port })
      await refreshStatus()
    } catch (cause) {
      error.value = String(cause)
      throw cause
    }
  }

  async function setNotificationEnabled(enabled: boolean) {
    error.value = ''
    try {
      await invoke('set_notification_enabled', { enabled })
      await refreshStatus()
    } catch (cause) {
      error.value = String(cause)
      throw cause
    }
  }

  async function refreshStatus() {
    status.value = await invoke<ReceiverStatus>('receiver_status')
    defaultSender.value = status.value.defaultSender || 'iPhone'
  }

  async function toggleReceiver(options: ToggleReceiverOptions = {}) {
    const running = status.value?.receiverRunning ?? false
    if (running) {
      const confirmed = options.confirmStop ? await options.confirmStop() : true
      if (!confirmed) {
        return
      }

      await invoke('stop_receiver')
    } else {
      await invoke('start_receiver_command')
    }

    await refreshStatus()
  }

  let stopMessageListener: (() => void) | null = null
  let stopClearListener: (() => void) | null = null
  let stopErrorListener: (() => void) | null = null

  onMounted(async () => {
    await refresh()

    stopMessageListener = await listen<IncomingMessage>('message-received', (event) => {
      messages.value = [event.payload, ...messages.value].slice(0, MAX_VISIBLE_MESSAGES)
      writeText(event.payload.copiedText)
        .then(() => {
          lastReceived.value = event.payload.copiedText
        })
        .catch((cause) => {
          error.value = `自动复制失败: ${String(cause)}`
        })
      refreshStatus()
    })

    stopClearListener = await listen('messages-cleared', () => {
      messages.value = []
      refreshStatus()
    })

    stopErrorListener = await listen<string>('receiver-error', (event) => {
      error.value = event.payload
    })
  })

  onUnmounted(() => {
    stopMessageListener?.()
    stopClearListener?.()
    stopErrorListener?.()
  })

  return {
    messages,
    status,
    loading,
    error,
    lastReceived,
    token,
    defaultSender,
    latestMessage,
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
  }
}
