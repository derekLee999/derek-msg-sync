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
  senderDevices: SenderDevice[]
}

export interface SenderDevice {
  id: string
  name: string
  deviceId: string
}
