export interface IncomingMessage {
  id: string
  sender: string
  text: string
  code: string | null
  copiedText: string
  receivedAt: string
  remoteAddr: string
}

export interface ReceiverStatus {
  port: number
  localIp: string | null
  endpoint: string
  messageCount: number
  receiverRunning: boolean
  notificationMode: NotificationMode
  notificationPosition: NotificationPosition
  directPasteEnabled: boolean
  relayEnabled: boolean
  relayRunning: boolean
  relayBaseUrl: string
  relaySecret: string
  senderDevices: SenderDevice[]
}

export type NotificationPosition = 'bottomRight' | 'bottomLeft' | 'topRight' | 'topLeft' | 'topCenter'
export type NotificationMode = 'all' | 'verification' | 'off'

export interface SenderDevice {
  id: string
  name: string
  deviceId: string
}

export interface RelaySettings {
  enabled: boolean
  baseUrl: string
  secret: string
}

export interface PlatformInfo {
  os: string
  isMacos: boolean
  isWindows: boolean
}
