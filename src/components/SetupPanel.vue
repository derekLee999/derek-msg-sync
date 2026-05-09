<script setup lang="ts">
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import { computed, onMounted, shallowRef, watch } from 'vue'

const props = defineProps<{
  endpoint: string
  defaultSender: string
  token: string
  isTokenEnabled: boolean
  notificationEnabled: boolean
  port: number
}>()

const emit = defineEmits<{
  saveDefaultSender: []
  saveToken: []
  setNotificationEnabled: [enabled: boolean]
  updateDefaultSender: [value: string]
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

const exampleJson = computed(() => {
  const payload: Record<string, string> = {
    sender: props.defaultSender.trim() || 'iPhone',
    text: '您的验证码是 123456，5 分钟内有效',
  }

  if (props.token.trim()) {
    payload.token = props.token.trim()
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
        <h3>安全校验</h3>
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
        <span>发送 sender、text、token；Windows 会自动复制验证码。</span>
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
</style>
