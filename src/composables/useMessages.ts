import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { computed, onMounted, onUnmounted, shallowRef } from 'vue'
import type { IncomingMessage, NotificationPosition, ReceiverStatus, SenderDevice } from '../types'

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
  const senderDevices = shallowRef<SenderDevice[]>([])

  const latestMessage = computed(() => messages.value[0] ?? null)
  const totalCount = computed(() => messages.value.length)
  const endpoint = computed(() => status.value?.endpoint ?? `http://<Windows局域网IP>:${status.value?.port ?? 17866}/otp`)

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
      senderDevices.value = cloneSenderDevices(nextStatus.senderDevices)
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

  async function setSenderDevices(devices: SenderDevice[]) {
    error.value = ''
    senderDevices.value = cloneSenderDevices(devices)
    try {
      await invoke('set_sender_devices', { devices })
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

  async function setNotificationPosition(position: NotificationPosition) {
    error.value = ''
    try {
      await invoke('set_notification_position', { position })
      await refreshStatus()
    } catch (cause) {
      error.value = String(cause)
      throw cause
    }
  }

  async function refreshStatus() {
    status.value = await invoke<ReceiverStatus>('receiver_status')
    senderDevices.value = cloneSenderDevices(status.value.senderDevices)
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
    senderDevices,
    latestMessage,
    totalCount,
    endpoint,
    clearMessages,
    copyLocalIp,
    copyMessage,
    copyRecent,
    refresh,
    restartReceiverWithPort,
    setNotificationEnabled,
    setNotificationPosition,
    setSenderDevices,
    toggleReceiver,
  }
}

function cloneSenderDevices(devices: SenderDevice[] | undefined) {
  const nextDevices = (devices ?? []).map((device) => ({ ...device }))
  return nextDevices.length > 0
    ? nextDevices
    : [
        {
          id: 'default-iphone',
          name: 'iPhone',
          deviceId: '',
        },
      ]
}
