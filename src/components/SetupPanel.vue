<script setup lang="ts">
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { openUrl } from '@tauri-apps/plugin-opener'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import { computed, onMounted, shallowRef, watch } from 'vue'
import type { NotificationPosition, SenderDevice } from '../types'

const MAX_SENDER_DEVICES = 5
const SHORTCUT_EXAMPLE_URL = 'https://www.icloud.com/shortcuts/d02d0af4323b403d8c4269019bb6f11f'
const NOTIFICATION_POSITION_OPTIONS: Array<{ label: string; value: NotificationPosition }> = [
  { label: '右下', value: 'bottomRight' },
  { label: '左下', value: 'bottomLeft' },
  { label: '右上', value: 'topRight' },
  { label: '左上', value: 'topLeft' },
  { label: '中上', value: 'topCenter' },
]

const props = defineProps<{
  endpoint: string
  senderDevices: SenderDevice[]
  notificationEnabled: boolean
  notificationPosition: NotificationPosition
  directPasteEnabled: boolean
  port: number
}>()

const emit = defineEmits<{
  setNotificationEnabled: [enabled: boolean]
  setNotificationPosition: [position: NotificationPosition]
  setDirectPasteEnabled: [enabled: boolean]
  showToast: [text: string]
  updateSenderDevices: [devices: SenderDevice[]]
  requestPortChange: [port: number]
}>()

type CopiedTarget = '' | 'endpoint' | 'json' | 'curl'

const copied = shallowRef<CopiedTarget>('')
const autostartEnabled = shallowRef(false)
const autostartBusy = shallowRef(false)
const portInput = shallowRef(String(props.port))

const parsedPort = computed(() => Number.parseInt(portInput.value, 10))
const portInvalid = computed(() => {
  const port = parsedPort.value
  return !Number.isInteger(port) || port < 1024 || port > 65535
})
const portChanged = computed(() => !portInvalid.value && parsedPort.value !== props.port)
const firstDevice = computed(() => props.senderDevices.find((device) => device.deviceId.trim()) ?? null)
const canRemoveDevice = computed(() => props.senderDevices.length > 1)
const senderDeviceInvalid = computed(() => props.senderDevices.some((device) => !device.name.trim()))
const senderDeviceHint = computed(() => {
  if (senderDeviceInvalid.value) {
    return '请填写设备名称'
  }

  return '设备 ID 由系统生成且不可修改，快捷指令只需发送对应 ID'
})

const copiedTip = computed(() => {
  const messages: Record<Exclude<CopiedTarget, ''>, string> = {
    endpoint: '接收地址已复制',
    json: 'JSON 示例已复制',
    curl: 'curl 请求已复制',
  }

  return copied.value ? messages[copied.value] : ''
})
const examplePayload = computed(() => {
  const payload: Record<string, string> = {
    text: '您的验证码是 123456，5 分钟内有效',
  }

  if (firstDevice.value) {
    payload.id = firstDevice.value.deviceId.trim()
  } else {
    payload.id = '自动生成'
  }

  return payload
})
const exampleJson = computed(() => JSON.stringify(examplePayload.value, null, 2))
const curlCommand = computed(() => {
  const endpoint = escapePowerShellSingleQuoted(props.endpoint)
  const payload = escapePowerShellSingleQuoted(JSON.stringify(examplePayload.value))

  return `curl.exe -X POST '${endpoint}' -H 'Content-Type: application/json' -d '${payload}'`
})

watch(
  () => props.port,
  (port) => {
    portInput.value = String(port)
  },
)

async function copyEndpoint() {
  await writeText(props.endpoint)
  copied.value = 'endpoint'
}

async function copyJson() {
  await writeText(exampleJson.value)
  copied.value = 'json'
}

async function copyCurl() {
  await writeText(curlCommand.value)
  copied.value = 'curl'
}

function escapePowerShellSingleQuoted(value: string) {
  return value.replace(/'/g, "''")
}

async function openShortcutExample() {
  try {
    await openUrl(SHORTCUT_EXAMPLE_URL)
  } catch {
    emit('showToast', '快捷指令链接打开失败')
  }
}

async function copyDeviceId(deviceId: string) {
  const normalizedDeviceId = deviceId.trim()
  if (!normalizedDeviceId) {
    emit('showToast', '设备 ID 会自动生成')
    return
  }

  await writeText(normalizedDeviceId)
  emit('showToast', '设备 ID 已复制')
}

async function refreshAutostart() {
  autostartEnabled.value = await isEnabled()
}

async function toggleAutostart() {
  autostartBusy.value = true
  try {
    if (autostartEnabled.value) {
      await disable()
      autostartEnabled.value = false
    } else {
      await enable()
      autostartEnabled.value = true
    }
  } finally {
    autostartBusy.value = false
  }
}

function submitPortChange() {
  if (!portChanged.value) {
    return
  }

  emit('requestPortChange', parsedPort.value)
}

function addSenderDevice() {
  if (props.senderDevices.length >= MAX_SENDER_DEVICES) {
    emit('showToast', `最多只能添加 ${MAX_SENDER_DEVICES} 个设备`)
    return
  }

  emit('updateSenderDevices', [
    ...props.senderDevices,
    {
      id: crypto.randomUUID(),
      name: `iPhone ${props.senderDevices.length + 1}`,
      deviceId: '',
    },
  ])
}

function updateSenderDevice(id: string, value: string) {
  emit(
    'updateSenderDevices',
    props.senderDevices.map((device) =>
      device.id === id
        ? {
            ...device,
            name: value,
          }
        : device,
    ),
  )
}

function removeSenderDevice(id: string) {
  if (!canRemoveDevice.value) {
    return
  }

  emit(
    'updateSenderDevices',
    props.senderDevices.filter((device) => device.id !== id),
  )
}

onMounted(() => {
  refreshAutostart()
})
</script>

<template>
  <aside class="setup-panel">
    <section class="settings-section autostart-section">
      <div class="setting-row">
        <div>
          <span>开机自启</span>
          <small>登录 Windows 后自动启动接收器</small>
        </div>
        <button
          type="button"
          :class="['switch-button', { active: autostartEnabled }]"
          :disabled="autostartBusy"
          role="switch"
          :aria-checked="autostartEnabled"
          @click="toggleAutostart"
        >
          <span></span>
        </button>
      </div>
      <div class="setting-row">
        <div>
          <span>接收通知</span>
          <small>收到 iPhone 消息后显示浮动通知</small>
        </div>
        <button
          type="button"
          :class="['switch-button', { active: notificationEnabled }]"
          role="switch"
          :aria-checked="notificationEnabled"
          @click="emit('setNotificationEnabled', !notificationEnabled)"
        >
          <span></span>
        </button>
      </div>
      <div v-if="notificationEnabled" class="notification-position-row">
        <span>通知位置</span>
        <div class="segmented-control" role="group" aria-label="通知显示位置">
          <button
            v-for="option in NOTIFICATION_POSITION_OPTIONS"
            :key="option.value"
            type="button"
            :class="{ active: notificationPosition === option.value }"
            @click="emit('setNotificationPosition', option.value)"
          >
            {{ option.label }}
          </button>
        </div>
      </div>
      <div class="setting-row">
        <div>
          <span>直接输入</span>
          <small>收到验证码后复制到剪贴板，并向当前光标位置逐个字符输入验证码</small>
        </div>
        <button
          type="button"
          :class="['switch-button', { active: directPasteEnabled }]"
          role="switch"
          :aria-checked="directPasteEnabled"
          @click="emit('setDirectPasteEnabled', !directPasteEnabled)"
        >
          <span></span>
        </button>
      </div>
    </section>

    <section class="settings-section">
      <div class="section-head">
        <div>
          <p class="eyebrow">iPhone 快捷指令</p>
          <h2>局域网接入</h2>
        </div>
        <button type="button" class="small-button" @click="openShortcutExample">快捷指令示例</button>
      </div>

      <div class="shortcut-notice">
        <div>
          <b>快捷指令需要修改</b>
          <span>请在iPhone快捷指令中把接收地址改为本机 IP，并把 JSON 里的 id 改为当前设备 ID。</span>
        </div>
        <div class="preview-tip">
          <button type="button" class="preview-trigger" aria-label="查看快捷指令截图">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 17v-6" />
              <path d="M12 8h.01" />
              <path d="M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20Z" />
            </svg>
          </button>
          <div class="preview-popover" role="tooltip">
            <img src="/auto-example.jpg" alt="快捷指令自动化示例截图" />
          </div>
        </div>
      </div>

      <label class="field">
        <span>接收地址</span>
        <div class="control-row">
          <input :value="endpoint" readonly />
          <button type="button" class="icon-button" title="复制接收地址" @click="copyEndpoint">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M8 8h10v10H8z" />
              <path d="M6 14H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h8a1 1 0 0 1 1 1v1" />
            </svg>
          </button>
        </div>
      </label>

      <label class="field">
        <span>服务端口</span>
        <div class="control-row">
          <input
            v-model="portInput"
            inputmode="numeric"
            type="number"
            min="1024"
            max="65535"
            placeholder="17866"
          />
          <button
            type="button"
            class="text-button"
            :disabled="!portChanged"
            @click="submitPortChange"
          >
            重启
          </button>
        </div>
        <small v-if="portInvalid" class="field-hint error">端口号需在 1024-65535 之间</small>
        <small v-else class="field-hint">修改端口后会重启接收服务并更新接收地址</small>
      </label>
    </section>

    <section class="settings-section">
      <div class="section-head compact">
        <h3>设备识别</h3>
        <button
          type="button"
          class="small-button"
          :title="`最多 ${MAX_SENDER_DEVICES} 个设备`"
          @click="addSenderDevice"
        >
          添加设备
        </button>
      </div>

      <div v-if="senderDevices.length" class="device-list">
        <div v-for="device in senderDevices" :key="device.id" class="device-row">
          <label>
            <span>设备名称</span>
            <input
              :value="device.name"
              placeholder="Derek iPhone"
              @input="updateSenderDevice(device.id, ($event.target as HTMLInputElement).value)"
            />
          </label>
          <label>
            <span>设备 ID</span>
            <input
              class="device-id-input"
              :value="device.deviceId || '自动生成'"
              readonly
              title="点击复制设备 ID"
              @click="copyDeviceId(device.deviceId)"
            />
          </label>
          <button
            type="button"
            class="danger-button"
            :disabled="!canRemoveDevice"
            :title="canRemoveDevice ? '删除设备' : '至少保留一个设备'"
            @click="removeSenderDevice(device.id)"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M5 12h14" />
            </svg>
          </button>
        </div>
      </div>

      <div class="section-actions">
        <small :class="['field-hint', { error: senderDeviceInvalid }]">
          {{ senderDeviceHint }}
        </small>
      </div>
    </section>

    <section class="settings-section">
      <div class="shortcut-title">
        <span>POST JSON 示例</span>
        <div class="shortcut-actions">
          <button type="button" class="icon-button quiet" title="复制 JSON 示例" @click="copyJson">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M8 8h10v10H8z" />
              <path d="M6 14H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h8a1 1 0 0 1 1 1v1" />
            </svg>
          </button>
          <button type="button" class="icon-button quiet" title="复制 curl 请求" @click="copyCurl">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="m4 7 5 5-5 5" />
              <path d="M12 17h8" />
            </svg>
          </button>
        </div>
      </div>
      <pre>{{ exampleJson }}</pre>
    </section>

    <section class="steps">
      <div class="step">
        <b>1</b>
        <span>快捷指令自动化选择 Message / 收到短信。</span>
      </div>
      <div class="step">
        <b>2</b>
        <span>动作选择 Get Contents of URL，方法为 POST。</span>
      </div>
      <div class="step">
        <b>3</b>
        <span>多设备建议发送 text、id；Windows 会根据 ID 自动识别设备。</span>
      </div>
    </section>

    <p v-if="copiedTip" class="copied-tip">{{ copiedTip }}</p>
  </aside>
</template>

<style scoped>
.setup-panel {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px;
  background: #ffffff;
}

.settings-section {
  min-width: 0;
  display: grid;
  gap: 12px;
  padding: 12px;
  border: 1px solid #e7ebf1;
  border-radius: 8px;
  background: #fbfcfe;
}

.autostart-section {
  background: #f6f9ff;
}

.section-head,
.shortcut-title,
.control-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.section-head,
.shortcut-title {
  justify-content: space-between;
}

.section-head.compact {
  min-height: 24px;
}

.eyebrow {
  margin: 0 0 3px;
  color: #667085;
  font-size: 12px;
}

h2,
h3 {
  margin: 0;
  color: #17202f;
}

h2 {
  font-size: 19px;
  line-height: 25px;
}

h3 {
  font-size: 14px;
  line-height: 20px;
}

.id-badge {
  flex: 0 0 auto;
  padding: 4px 8px;
  border-radius: 999px;
  color: #0b7a56;
  background: #dff8ed;
  font-size: 12px;
}

.field {
  display: grid;
  gap: 7px;
  min-width: 0;
  color: #465160;
  font-size: 13px;
}

.field-hint {
  margin-top: -2px;
  color: #667085;
  font-size: 12px;
  line-height: 18px;
}

.field-hint.error {
  color: #b42318;
}

.shortcut-notice {
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 28px;
  align-items: start;
  gap: 8px;
  padding: 9px 10px;
  border: 1px solid #f7d68a;
  border-radius: 7px;
  color: #7a4f01;
  background: #fff8e6;
  font-size: 12px;
  line-height: 18px;
}

.shortcut-notice b {
  display: block;
  color: #5f3b00;
  font-size: 13px;
}

.preview-tip {
  position: relative;
  display: grid;
  place-items: center;
}

.preview-trigger {
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  color: #7a4f01;
  background: #ffefbd;
}

.preview-trigger:hover,
.preview-trigger:focus-visible {
  color: #5f3b00;
  background: #ffe39a;
}

.preview-trigger svg {
  width: 16px;
  height: 16px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.preview-popover {
  position: fixed;
  top: 50%;
  left: 50%;
  z-index: 30;
  width: min(420px, calc(100vw - 72px));
  max-height: min(620px, calc(100vh - 96px));
  overflow: auto;
  padding: 8px;
  border: 1px solid rgba(28, 39, 54, 0.14);
  border-radius: 8px;
  background: #ffffff;
  box-shadow: 0 24px 70px rgba(28, 39, 54, 0.3);
  opacity: 0;
  pointer-events: none;
  transform: translate(-50%, -48%);
  transition:
    opacity 0.14s ease,
    transform 0.14s ease;
}

.preview-popover img {
  display: block;
  width: 100%;
  height: auto;
  border-radius: 5px;
}

.preview-tip:hover .preview-popover,
.preview-tip:focus-within .preview-popover {
  opacity: 1;
  pointer-events: auto;
  transform: translate(-50%, -50%);
}

.section-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}

.setting-row span,
.setting-row small {
  display: block;
}

.setting-row span {
  color: #17202f;
  font-size: 14px;
  font-weight: 700;
}

.setting-row small {
  margin-top: 2px;
  color: #667085;
  font-size: 12px;
}

.notification-position-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-top: 2px;
}

.notification-position-row > span {
  flex: 0 0 auto;
  color: #465160;
  font-size: 13px;
}

.segmented-control {
  min-width: 0;
  display: grid;
  grid-template-columns: repeat(5, minmax(38px, 1fr));
  gap: 3px;
  padding: 3px;
  border: 1px solid #d7dde7;
  border-radius: 7px;
  background: #ffffff;
}

.segmented-control button {
  height: 28px;
  padding: 0 9px;
  border-radius: 5px;
  color: #465160;
  background: transparent;
  font-size: 12px;
  font-weight: 700;
}

.segmented-control button:hover {
  color: #1769e0;
  background: #edf4ff;
}

.segmented-control button.active {
  color: #ffffff;
  background: #1769e0;
}

.switch-button {
  width: 46px;
  height: 26px;
  flex: 0 0 auto;
  padding: 3px;
  border-radius: 999px;
  background: #d7dde7;
}

.switch-button span {
  display: block;
  width: 20px;
  height: 20px;
  border-radius: 999px;
  background: #ffffff;
  box-shadow: 0 2px 5px rgba(28, 39, 54, 0.2);
  transition: transform 0.16s ease;
}

.switch-button.active {
  background: #1769e0;
}

.switch-button.active span {
  transform: translateX(20px);
}

.control-row input {
  min-width: 0;
  width: 100%;
  height: 36px;
  padding: 0 10px;
  border: 1px solid #d7dde7;
  border-radius: 7px;
  color: #17202f;
  background: #ffffff;
  font: inherit;
}

.control-row input:focus {
  border-color: #1769e0;
  outline: 2px solid rgba(23, 105, 224, 0.12);
}

.device-list {
  display: grid;
  gap: 8px;
}

.device-row {
  display: grid;
  grid-template-columns: minmax(116px, 0.82fr) minmax(160px, 1.18fr) 32px;
  align-items: end;
  gap: 8px;
  padding: 10px;
  border: 1px solid #e7ebf1;
  border-radius: 8px;
  background: #ffffff;
}

.device-row label {
  min-width: 0;
  display: grid;
  gap: 5px;
  color: #465160;
  font-size: 12px;
}

.device-row input {
  min-width: 0;
  width: 100%;
  height: 34px;
  padding: 0 9px;
  border: 1px solid #d7dde7;
  border-radius: 7px;
  color: #17202f;
  background: #ffffff;
  font: inherit;
}

.device-row input:focus {
  border-color: #1769e0;
  outline: 2px solid rgba(23, 105, 224, 0.12);
}

.device-row .device-id-input {
  cursor: pointer;
  color: #0b1b31;
  background: #f8fafc;
}

.device-row .device-id-input:hover {
  border-color: #b8c2d1;
  background: #f1f6ff;
}

.device-row .device-id-input:focus {
  border-color: #d7dde7;
  outline: 0;
  box-shadow: none;
}

.empty-devices {
  margin: 0;
  padding: 10px;
  border: 1px dashed #d7dde7;
  border-radius: 8px;
  color: #667085;
  background: #ffffff;
  font-size: 13px;
  line-height: 18px;
}

button {
  height: 36px;
  border: 0;
  border-radius: 7px;
  color: #ffffff;
  background: #1769e0;
  cursor: pointer;
  font: inherit;
}

button:disabled {
  cursor: default;
  opacity: 0.48;
}

.icon-button {
  width: 36px;
  flex: 0 0 auto;
  display: grid;
  place-items: center;
}

.icon-button svg {
  width: 17px;
  height: 17px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.text-button {
  flex: 0 0 auto;
  padding: 0 14px;
  font-weight: 700;
}

.small-button {
  height: 30px;
  flex: 0 0 auto;
  padding: 0 10px;
  color: #1769e0;
  background: #e8f1ff;
  font-size: 12px;
  font-weight: 700;
}

.danger-button {
  width: 32px;
  height: 34px;
  display: grid;
  place-items: center;
  color: #b42318;
  background: #fee4e2;
}

.danger-button svg {
  width: 15px;
  height: 15px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2.4;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.quiet {
  width: 30px;
  height: 30px;
  color: #1769e0;
  background: #e8f1ff;
}

.shortcut-title {
  color: #17202f;
  font-size: 13px;
  font-weight: 700;
}

.shortcut-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

pre {
  overflow: auto;
  max-height: 112px;
  margin: 0;
  padding: 10px;
  border-radius: 7px;
  color: #e5e7eb;
  background: #111827;
  font-size: 12px;
  line-height: 18px;
  white-space: pre-wrap;
}

.steps {
  display: grid;
  gap: 8px;
}

.step {
  display: grid;
  grid-template-columns: 22px minmax(0, 1fr);
  align-items: start;
  gap: 8px;
  color: #465160;
  font-size: 13px;
  line-height: 18px;
}

.step b {
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  border-radius: 999px;
  color: #1769e0;
  background: #e8f1ff;
  font-size: 12px;
}

.copied-tip {
  margin: -3px 0 0;
  color: #0b7a56;
  font-size: 13px;
}

@media (max-width: 560px) {
  .device-row {
    grid-template-columns: minmax(0, 1fr) 32px;
  }

  .device-row label {
    grid-column: 1 / -1;
  }

  .danger-button {
    grid-column: 2;
  }

  .section-actions {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
