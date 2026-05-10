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
  notificationEnabled: boolean
  notificationPosition: NotificationPosition
  directPasteEnabled: boolean
  relayEnabled: boolean
  relayRunning: boolean
  relayBaseUrl: string
  relaySecret: string
  senderDevices: SenderDevice[]
}

export type NotificationPosition = 'bottomRight' | 'bottomLeft' | 'topRight' | 'topLeft' | 'topCenter'

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
