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
  tokenRequired: boolean
  receiverRunning: boolean
  notificationEnabled: boolean
}
