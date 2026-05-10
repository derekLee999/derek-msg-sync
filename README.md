# 验证码接收器

`derek-msg-sync` 是一个 Windows 桌面端验证码接收工具。它可以接收 iPhone 快捷指令转发的短信内容，自动提取验证码，复制到 Windows 剪贴板，并可选模拟逐字符输入验证码。

项目使用 Tauri 2 + Vue 3 开发，当前版本为 `0.0.2`。

## 功能

- 局域网接收：在 Windows 本机启动 HTTP 接收服务，iPhone 快捷指令通过局域网 IP 发送消息。
- 云端接入：配合独立的 `derek-relay` 云端中转服务，iPhone 和 Windows 不在同一局域网时也能接收。
- 多设备识别：最多配置 5 台 iPhone，每台设备有独立设备 ID。
- 自动提取验证码：从短信正文中提取 4-8 位数字验证码。
- 自动复制：收到消息后自动复制验证码；未提取到验证码时复制原始文本。
- 直接输入：可选开启，复制后向当前光标位置逐个字符输入验证码。
- 消息收件台：本地持久化保存最近消息。
- 应用通知：使用 Tauri 窗口模拟 Windows 通知，支持通知位置和提示音。
- 托盘支持：关闭窗口默认隐藏到托盘，托盘菜单可退出程序。
- 开机自启、端口设置、窗口置顶、自定义标题栏。

## 接收方式

### 局域网接入

适用于 iPhone 和 Windows 在同一 Wi-Fi / 局域网内。

1. 打开应用设置。
2. 在“局域网接入”中复制接收地址，例如：

```text
http://192.168.31.43:17866/otp
```

3. 在 iPhone 快捷指令中把 URL 改为这个地址。
4. POST JSON 示例：

```json
{
  "text": "您的验证码是 123456，5 分钟内有效",
  "id": "1234567"
}
```

`id` 需要填写应用设置中对应设备的“设备 ID”。

### 云端接入

适用于 iPhone 和 Windows 不在同一局域网内。

云端接入依赖独立服务：

[derek-relay](https://github.com/derekLee999/derek-relay)

链路如下：

```text
iPhone 快捷指令
→ derek-relay 云端服务
→ Windows 客户端主动轮询云端
→ 复制验证码 / 通知 / 直接输入
```

Windows 不需要公网 IP，也不需要开放家庭路由器端口。

设置项：

```text
云端服务地址：http://服务器公网IP:18080
云端密钥：与 derek-relay 的 RELAY_SECRET 保持一致
```

iPhone 快捷指令 URL：

```text
http://服务器公网IP:18080/api/messages
```

POST JSON 示例：

```json
{
  "text": "您的验证码是 123456，5 分钟内有效",
  "id": "1234567",
  "secret": "你的云端密钥"
}
```

如果没有域名和 HTTPS，消息会通过公网 HTTP 明文传输。个人临时使用可以先跑通，长期使用建议配置 HTTPS 或后续增加消息体加密。

## iPhone 快捷指令配置

建议使用“自动化”触发：

1. 打开 iPhone 快捷指令 App。
2. 进入“自动化”。
3. 新建自动化，选择“信息”或“收到短信”。
4. 条件按需选择发件人或包含关键字。
5. 添加动作“获取 URL 内容”。
6. 方法选择 `POST`。
7. 请求体选择 `JSON`。
8. 填入 `text` 和 `id`；云端接入时额外填写 `secret`。

应用设置中提供了快捷指令示例链接和截图预览。

## 云端服务部署示例

在云服务器部署 `derek-relay` 后，应用即可云端接收。

推荐直接参考：

[derek-relay README](https://github.com/derekLee999/derek-relay)

典型 Docker 部署参数：

```bash
RELAY_SECRET=一串足够长的随机密钥
RELAY_LISTEN=:18080
```

服务检查：

```bash
curl http://服务器公网IP:18080/api/health
```

鉴权检查：

```bash
curl -H "Authorization: Bearer 你的云端密钥" \
  http://服务器公网IP:18080/api/verify
```

## 开发

环境要求：

- Windows 10 / 11
- Node.js
- Rust
- Tauri 2 依赖环境

安装依赖：

```powershell
npm install
```

启动开发模式：

```powershell
npm run tauri dev
```

前端构建检查：

```powershell
npm run build
```

Rust 检查：

```powershell
cd src-tauri
cargo check
```

## 构建安装包

```powershell
npm run tauri build
```

常见产物位置：

```text
src-tauri/target/release/bundle/nsis/
src-tauri/target/release/bundle/msi/
```

## 安全说明

- 设备 ID 用于区分设备，不应视为强密码。
- 云端接入必须配置足够长的云端密钥。
- 没有 HTTPS 时，公网 HTTP 传输存在明文风险。
- “直接输入”会向当前焦点窗口输入验证码，开启前请确认当前使用场景可信。

## License

Apache-2.0
