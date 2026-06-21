# Copy-Sharer

Copy-Sharer 是一款局域网剪贴板共享桌面工具。当前主架构已经重构为 **Rust + Tauri 2 + Vue 3 + TypeScript**：Vue 负责界面，Rust 负责剪贴板、WebSocket、设备状态、历史摘要、托盘和系统集成。

## MVP 范围

第一版聚焦两台电脑在同一局域网内同步文本剪贴板：

- 手动输入对方 IP 和端口连接设备。
- 监听本机文本剪贴板变化。
- 通过 WebSocket 发送文本剪贴板消息。
- 收到远端文本后写入本机剪贴板。
- 通过 `message_id`、`source_device_id`、`content_hash` 防止同步死循环。
- 显示总览、设备连接、同步历史和设置页面。
- 历史记录只保存摘要，不保存完整敏感剪贴板内容。
- 支持系统托盘和开机自启设置。

暂不做图片同步、文件同步、云端同步、二维码配对、手机端和公网穿透。

## 技术结构

```text
src/
  Vue 3 + TypeScript UI
  Pinia stores
  Vue Router pages
  Tauri invoke/event 封装

src-tauri/
  Rust Tauri 2 后端
  commands / state / sync / network / clipboard
  config / history / tray / autostart / security
```

核心原则：

```text
UI 只发命令
Rust 执行逻辑
Rust 通过事件通知 UI
```

## 开发命令

安装依赖：

```powershell
npm.cmd install
```

启动前端开发服务器：

```powershell
npm.cmd run dev
```

启动 Tauri 桌面开发模式：

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
npm.cmd run tauri:dev
```

构建前端：

```powershell
npm.cmd run build
```

运行 Rust 测试：

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cd src-tauri
cargo test
```

打包桌面应用：

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
npm.cmd run tauri:build
```

## 构建产物

成功打包后会生成：

```text
src-tauri\target\release\copy-sharer.exe
src-tauri\target\release\bundle\msi\Copy-Sharer_1.0.0_x64_en-US.msi
src-tauri\target\release\bundle\nsis\Copy-Sharer_1.0.0_x64-setup.exe
```

## 使用方式

1. 在两台同一局域网内的电脑上启动 Copy-Sharer。
2. 打开“设备连接”页面。
3. 在其中一台电脑输入另一台电脑的局域网 IPv4 地址，例如 `192.168.1.20`。
4. 使用默认端口 `8765`，或输入对方设置中的监听端口。
5. 点击“连接”。
6. 回到“总览”页面，点击“开始同步”。

连接后，在任一设备复制文本，另一台设备会自动写入剪贴板。

## 安全说明

- 本工具默认面向局域网使用，请勿在不可信网络中开放监听端口。
- MVP 只同步文本，后续再扩展图片和文件。
- 历史页面只保存文本摘要，避免长期明文保存敏感剪贴板内容。
- 首次连接陌生设备应通过“信任设备”流程确认。

## 验证记录

本次重构和清理后已通过：

- `npm.cmd ci --prefer-offline`
- `npm.cmd run build`
- `cargo test`
