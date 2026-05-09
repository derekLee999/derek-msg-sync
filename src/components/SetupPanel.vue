<script setup lang="ts">
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import { computed, onMounted, shallowRef, watch } from 'vue'
import type { SenderDevice } from '../types'

const props = defineProps<{
  endpoint: string
  defaultSender: string
  senderDevices: SenderDevice[]
  token: string
  isTokenEnabled: boolean
  notificationEnabled: boolean
  port: number
}>()

const emit = defineEmits<{
  saveDefaultSender: []
  saveSenderDevices: []
  saveToken: []
  setNotificationEnabled: [enabled: boolean]
  updateDefaultSender: [value: string]
  updateSenderDevices: [devices: SenderDevice[]]
  updateToken: [value: string]
  requestPortChange: [port: number]
}>()

const copied = shallowRef('')
const autostartEnabled = shallowRef(false)
const autostartBusy = shallowRef(false)
const portInput = shallowRef(String(props.port))

const parsedPort = computed(() => Number.parseInt(portInput.value, 10))
const portInvalid = computed(() => {
  const port = parsedPort.value
  return !Number.isInteger(port) || port < 1024 || port > 65535
})
const portChanged = computed(() => !portInvalid.value && parsedPort.value !== props.port)
const firstDevice = computed(() => props.senderDevices.find((device) => device.token.trim()) ?? null)
const senderDeviceInvalid = computed(() =>
  props.senderDevices.some((device) => !device.name.trim() || !device.token.trim()) || hasDuplicateDeviceToken.value,
)
const hasDuplicateDeviceToken = computed(() => {
  const tokens = props.senderDevices.map((device) => device.token.trim()).filter(Boolean)
  return new Set(tokens).size !== tokens.length
})
const senderDeviceHint = computed(() => {
  if (hasDuplicateDeviceToken.value) {
    return '设备 Token 不能重复'
  }

  if (senderDeviceInvalid.value) {
    return '请补全设备名称和 Token 后保存'
  }

  return '快捷指令只需发送对应 Token，即可自动区分多部 iPhone'
})

const exampleJson = computed(() => {
  const payload: Record<string, string> = {
    text: '您的验证码是 123456，5 分钟内有效',
  }

  if (firstDevice.value) {
    payload.token = firstDevice.value.token.trim()
  } else {
    payload.sender = props.defaultSender.trim() || 'iPhone'
    if (props.token.trim()) {
      payload.token = props.token.trim()
    }
  }

  return JSON.stringify(payload, null, 2)
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
  emit('updateSenderDevices', [
    ...props.senderDevices,
    {
      id: crypto.randomUUID(),
      name: `iPhone ${props.senderDevices.length + 1}`,
      token: '',
    },
  ])
}

function updateSenderDevice(id: string, field: 'name' | 'token', value: string) {
  emit(
    'updateSenderDevices',
    props.senderDevices.map((device) =>
      device.id === id
        ? {
            ...device,
            [field]: value,
          }
        : device,
    ),
  )
}

function removeSenderDevice(id: string) {
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
          <small>收到 iPhone 消息后在右下角显示通知</small>
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
    </section>

    <section class="settings-section">
      <div class="section-head">
        <div>
          <p class="eyebrow">iPhone 快捷指令</p>
          <h2>局域网接入</h2>
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

      <label class="field">
        <span>发送方名称</span>
        <div class="control-row">
          <input
            :value="defaultSender"
            placeholder="iPhone"
            @input="emit('updateDefaultSender', ($event.target as HTMLInputElement).value)"
          />
          <button type="button" class="text-button" @click="emit('saveDefaultSender')">保存</button>
        </div>
        <small class="field-hint">快捷指令未发送 sender 时，消息将使用此名称</small>
      </label>
    </section>

    <section class="settings-section">
      <div class="section-head compact">
        <h3>设备识别</h3>
        <button type="button" class="small-button" @click="addSenderDevice">添加设备</button>
      </div>

      <div v-if="senderDevices.length" class="device-list">
        <div v-for="device in senderDevices" :key="device.id" class="device-row">
          <label>
            <span>设备名称</span>
            <input
              :value="device.name"
              placeholder="Derek iPhone"
              @input="updateSenderDevice(device.id, 'name', ($event.target as HTMLInputElement).value)"
            />
          </label>
          <label>
            <span>设备 Token</span>
            <input
              :value="device.token"
              placeholder="每部 iPhone 使用不同 Token"
              @input="updateSenderDevice(device.id, 'token', ($event.target as HTMLInputElement).value)"
            />
          </label>
          <button type="button" class="danger-button" title="删除设备" @click="removeSenderDevice(device.id)">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M5 12h14" />
            </svg>
          </button>
        </div>
      </div>

      <p v-else class="empty-devices">添加设备后，Windows 会根据 Token 自动显示对应 iPhone 名称。</p>

      <div class="section-actions">
        <small :class="['field-hint', { error: senderDeviceInvalid }]">
          {{ senderDeviceHint }}
        </small>
        <button
          type="button"
          class="text-button"
          :disabled="senderDeviceInvalid"
          @click="emit('saveSenderDevices')"
        >
          保存设备
        </button>
      </div>
    </section>

    <section class="settings-section">
      <div class="section-head compact">
        <h3>通用校验</h3>
        <span v-if="isTokenEnabled" class="token-badge">Token 已开启</span>
      </div>

      <label class="field">
        <span>本地密钥</span>
        <div class="control-row">
          <input
            :value="token"
            placeholder="留空则不校验"
            @input="emit('updateToken', ($event.target as HTMLInputElement).value)"
          />
          <button type="button" class="text-button" @click="emit('saveToken')">保存</button>
        </div>
      </label>
    </section>

    <section class="settings-section">
      <div class="shortcut-title">
        <span>POST JSON 示例</span>
        <button type="button" class="icon-button quiet" title="复制 JSON 示例" @click="copyJson">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M8 8h10v10H8z" />
            <path d="M6 14H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h8a1 1 0 0 1 1 1v1" />
          </svg>
        </button>
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
        <span>多设备建议发送 text、token；Windows 会根据 Token 自动识别设备。</span>
      </div>
    </section>

    <p v-if="copied" class="copied-tip">{{ copied === 'endpoint' ? '接收地址已复制' : 'JSON 示例已复制' }}</p>
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

.token-badge {
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
