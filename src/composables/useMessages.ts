import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { computed, onMounted, onUnmounted, shallowRef } from 'vue'
import type {
  IncomingMessage,
  NotificationMode,
  NotificationPosition,
  ReceiverStatus,
  RelaySettings,
  SenderDevice,
} from '../types'

const MAX_VISIBLE_MESSAGES = 100
const VERIFICATION_KEYWORDS = ['验证码', '校验码']

async function logFrontend(msg: string) {
  try {
    await invoke('log_message', { tag: 'FRONTEND', msg })
  } catch {
    // Logging should never break the app
  }
}

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
  const verificationFilterEnabled = shallowRef(true)

  const latestMessage = computed(() => messages.value[0] ?? null)
  const totalCount = computed(() => messages.value.length)
  const visibleMessages = computed(() => {
    if (!verificationFilterEnabled.value) {
      return messages.value
    }

    return messages.value.filter(hasVerificationKeyword)
  })
  const visibleTotalCount = computed(() => visibleMessages.value.length)
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
      lastReceived.value = nextMessages.find((message) => message.code)?.code ?? ''
      void logFrontend(`刷新完成: ${nextMessages.length}条消息, 接收器${nextStatus.receiverRunning ? '已启动' : '已停止'}, 云端${nextStatus.relayRunning ? '已启动' : '已停止'}`)
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

  function setVerificationFilterEnabled(enabled: boolean) {
    verificationFilterEnabled.value = enabled
  }

  function clearError() {
    error.value = ''
  }

  async function copyMessage(message: IncomingMessage) {
    await writeText(message.copiedText)
    if (message.code) {
      lastReceived.value = message.code
    }
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

  async function setNotificationMode(mode: NotificationMode) {
    error.value = ''
    try {
      await invoke('set_notification_mode', { mode })
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

  async function setDirectPasteEnabled(enabled: boolean) {
    error.value = ''
    try {
      await invoke('set_direct_paste_enabled', { enabled })
      await refreshStatus()
    } catch (cause) {
      error.value = String(cause)
      throw cause
    }
  }

  async function setRelaySettings(relay: RelaySettings) {
    error.value = ''
    void logFrontend(`设置云端: baseUrl=${relay.baseUrl}, enabled=${relay.enabled}, secretLen=${relay.secret?.length ?? 0}`)
    try {
      await invoke('set_relay_settings', { relay })
      await refreshStatus()
    } catch (cause) {
      error.value = String(cause)
      void logFrontend(`设置云端失败: ${String(cause)}`)
      throw cause
    }
  }

  async function testRelayConnection(relay: RelaySettings) {
    error.value = ''
    void logFrontend(`测试连接: ${relay.baseUrl}`)
    try {
      await invoke('test_relay_connection', { relay })
      void logFrontend('测试连接成功')
    } catch (cause) {
      error.value = String(cause)
      void logFrontend(`测试连接失败: ${String(cause)}`)
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
      error.value = ''
      void logFrontend(`收到消息: sender=${event.payload.sender}, code=${event.payload.code ?? '(无)'}, text=${event.payload.text?.substring(0, 50)}`)
      messages.value = [event.payload, ...messages.value].slice(0, MAX_VISIBLE_MESSAGES)
      writeText(event.payload.copiedText)
        .then(() => {
          if (event.payload.code) {
            lastReceived.value = event.payload.code
          }
          if (status.value?.directPasteEnabled && event.payload.code) {
            return invoke('type_verification_code', { code: event.payload.code })
          }

          return undefined
        })
        .catch((cause) => {
          error.value = `自动复制或输入失败: ${String(cause)}`
        })
      refreshStatus()
    })

    stopClearListener = await listen('messages-cleared', () => {
      messages.value = []
      refreshStatus()
    })

    stopErrorListener = await listen<string>('receiver-error', (event) => {
      error.value = event.payload
      void logFrontend(`错误: ${event.payload}`)
    })
  })

  onUnmounted(() => {
    stopMessageListener?.()
    stopClearListener?.()
    stopErrorListener?.()
  })

  return {
    messages,
    visibleMessages,
    status,
    loading,
    error,
    lastReceived,
    senderDevices,
    verificationFilterEnabled,
    latestMessage,
    totalCount,
    visibleTotalCount,
    endpoint,
    clearMessages,
    clearError,
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

function hasVerificationKeyword(message: IncomingMessage) {
  const content = `${message.text ?? ''} ${message.copiedText ?? ''}`
  return VERIFICATION_KEYWORDS.some((keyword) => content.includes(keyword))
}
