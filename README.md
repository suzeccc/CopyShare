# LAN Clipboard Sync MVP

这是一个局域网文本和图片剪贴板同步工具。当前实现遵循 `clipboard-sync-design.md`，只使用 Python 标准库，不需要安装第三方依赖。

## 功能
- 监听本机文本和图片剪贴板。
- 通过 `ws://` WebSocket 发送剪贴板更新。
- 默认监听端口 `8765`。
- 通过 `--peer` 手动连接其他设备，UI 内会显示为“设备地址 / IP”。
- 使用消息 ID 和内容 hash 去重，避免循环同步。
- 对不可用的设备连接自动重连。

## 运行要求
- Python 3.14 或兼容的 Python 3 运行时。
- 当前 MVP 的系统剪贴板适配器仅支持 Windows：文本使用 PowerShell 的 `Get-Clipboard` / `Set-Clipboard`，图片使用 Windows 剪贴板 API 读写 PNG。
- 两台设备需要在同一个局域网内互相可达。Windows 防火墙可能需要放行 TCP `8765` 端口。

## 运行方式

### 桌面 UI

打开 A+C 混合版桌面控制台：

```powershell
python -m lan_clipboard_sync.ui
```

带设备 ID 和对端设备启动：

```powershell
python -m lan_clipboard_sync.ui --device-id device-a --peer 192.168.1.20
```

桌面 UI 使用 Python 标准库 `tkinter` 实现，当前是 **UI UX Pro Max / Dark Tray Console Glass** 深色毛玻璃主题：窗口默认 `1120x680`，使用 Windows Mica/深色玻璃面板、绿色主操作、绿色在线状态和紧凑卡片间距。整窗保持不透明以避免背景文字干扰阅读，玻璃感由系统背景和面板层级承担。字体系统使用 `Microsoft YaHei UI` 承担中文界面，`Cascadia Mono` 承担日志数字。

当前 UI 已按 `clipboard_ui_optimization.md` 重排为四个页面：

- **总览**：同步状态、开始/停止同步主按钮、已连接设备、手动连接和连接/剪贴日志。
- **设备连接**：手动输入“设备地址 / IP”，添加、移除或清空设备。
- **剪贴历史**：查看、清空和复制连接与剪贴同步记录。
- **设置**：调整设备 ID、监听端口和紧凑状态入口。

“自动发现”和“扫码配对”按钮会给出后续版本提示；当前不会伪装成已实现，仍需要手动输入设备 IP。真正系统托盘会作为后续增强。

如果只想看可交互 UI 预览，可以打开本地预览页：

```text
http://localhost:61237/
```

预览页用于查看和点击 UI 交互，不承担真实剪贴板同步；真实同步请运行上面的 `python -m lan_clipboard_sync.ui`。预览页与桌面 UI 使用同一套 Pro Max 深色毛玻璃主题方向，其中浏览器预览页使用真实 CSS `backdrop-filter` 毛玻璃效果，桌面 Tkinter 版使用 Windows Mica 加深色玻璃面板来接近同一视觉风格。

### 命令行模式

在设备 B 上启动监听：

```powershell
python -m lan_clipboard_sync --device-id device-b
```

在设备 A 上连接设备 B：

```powershell
python -m lan_clipboard_sync --device-id device-a --peer 192.168.1.20
```

把 `192.168.1.20` 换成设备 B 的局域网 IPv4 地址。可以在设备 B 的 PowerShell 里运行下面的命令查看：

```powershell
ipconfig
```

`--peer` 支持裸 IP、`host:port`，也支持完整的 `ws://` URL：

```powershell
python -m lan_clipboard_sync --peer 192.168.1.20 --peer ws://192.168.1.21:8765/
```

常用参数：

```text
--host 0.0.0.0            服务端绑定地址。
--port 8765               服务端 WebSocket 端口。
--poll-interval 0.1       剪贴板轮询间隔，单位秒。
--reconnect-delay 2.0     对端设备断开后的重连间隔，单位秒。
--device-id NAME          当前设备在同步消息中的稳定标识。
--verbose                 启用调试日志。
```

## 测试

```powershell
python -m unittest discover -v
python -m lan_clipboard_sync --help
python -m lan_clipboard_sync.ui --help
```

## 打包为 Windows 程序

项目已提供 Windows 打包脚本，使用 PyInstaller 生成可双击运行的桌面程序：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package_windows.ps1
```

如果旧版 `dist\LanClipboardSync.exe` 正在运行，先关闭它，或使用：

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package_windows.ps1 -StopRunning
```

打包产物：

```text
dist\LanClipboardSync.exe
```

已验证该 `.exe` 可以启动 Tkinter 桌面控制台。首次在两台设备之间同步时，Windows 防火墙可能会询问是否允许局域网访问；需要允许 TCP `8765` 端口或当前程序访问局域网。

## 通信协议

剪贴板消息是 JSON 文本帧：

```json
{
  "type": "clipboard",
  "id": "uuid",
  "deviceId": "device-A",
  "timestamp": 1710000000,
  "format": "text",
  "content": "hello world"
}
```

`format` 当前支持 `text` 和 `image`。图片会在本机剪贴板读取后转成 PNG，并以 base64 字符串放入 `content` 字段同步到对端。

程序也接受：

```json
{ "type": "ping" }
{ "type": "pong" }
```

## MVP 限制
- 支持文本和图片剪贴板。
- 暂不支持文件剪贴板。
- 不支持云端中继或跨公网同步。
- 不支持自动设备发现。
- 暂不支持加密或 PIN 配对。
- 当前系统剪贴板适配器仅支持 Windows。
