<div align="center">

<img src="src-tauri/icons/icon.ico" width="96" height="96" alt="CopyShare" />

# CopyShare

**局域网剪贴板同步工具**

在同一局域网内同步文本、视频、图片和文件剪贴板内容。无需公网服务器，不上传云端。

<p>
  <img alt="版本" src="https://img.shields.io/static/v1?label=%E7%89%88%E6%9C%AC&message=v3.0.0&color=22c55e&style=flat-square">
  <img alt="许可证" src="https://img.shields.io/static/v1?label=%E8%AE%B8%E5%8F%AF%E8%AF%81&message=MIT&color=0ea5e9&style=flat-square">
  <img alt="平台" src="https://img.shields.io/static/v1?label=%E5%B9%B3%E5%8F%B0&message=Windows&color=2563eb&style=flat-square">
  <img alt="Tauri" src="https://img.shields.io/static/v1?label=Tauri&message=2&color=24c8db&style=flat-square">
  <img alt="Vue" src="https://img.shields.io/static/v1?label=Vue&message=3&color=42b883&style=flat-square">
</p>

</div>

## 简介

CopyShare 面向多设备局域网办公场景：两台电脑互相信任后，可自动同步剪贴板；手机可通过二维码临时连接，用来发送或查看剪贴板内容。

适合：

- 多台电脑之间频繁复制文本、截图、图片或文件。
- 不希望剪贴板内容经过公网或第三方云端。
- 需要在局域网内临时用手机给电脑发送文字。
- 办公室、宿舍、家庭等同网段设备协作。

## 界面预览

### 桌面同步控制台

<p align="center">
  <img src="docs/images/1.png" alt="CopyShare 桌面同步控制台" width="900">
</p>

### 最近同步内容

<p align="center">
  <img src="docs/images/2-2.png" alt="CopyShare 最近同步内容列表" width="900">
</p>

### 全部剪贴内容

<p align="center">
  <img src="docs/images/2-1.png" alt="CopyShare 全部剪贴内容弹窗" width="900">
</p>



### 设备连接

<p align="center">
  <img src="docs/images/3.png" alt="CopyShare 设备连接页面" width="900">
</p>

### 桌面浮窗

<p align="center">
  <img src="docs/images/4.png" alt="CopyShare 桌面浮窗" width="720">
</p>

## 主要功能

### 剪贴板同步

- 支持文本、截图、图片同步。
- 支持文件类剪贴板内容复制与下载。
- 最近历史可保留并展示多条同步记录。
- 支持按类型筛选：全部、文本、图片、链接、文件。

### 设备连接

- 自动发现同一局域网内正在运行的 CopyShare 设备。
- 支持手动输入 IPv4 地址和端口连接。
- 双方确认信任后才开始同步内容。
- 支持查看设备在线状态、连接状态和延迟。
- 已信任设备重新连接时自动恢复可信同步状态。

### 手机临时连接

- 电脑端生成二维码。
- 手机扫码后可临时查看电脑剪贴板内容。
- 手机可提交文本到电脑剪贴板。
- 会话关闭后自动失效。

### 文件保存

- 支持修改文件保存位置。
- 支持保存后自动打开文件夹。
- 支持一键打开下载文件夹。
- 支持恢复默认保存位置。

### 翻译功能

- 内置翻译页面，可输入或粘贴文本并选择目标语言。
- 默认使用 Google 免费翻译，无需额外配置。
- 支持在设置中切换 AI 翻译，并配置 API 地址、API Key 和模型。
- 支持中文、英语、日语、韩语、法语、德语、西班牙语等目标语言。
- 翻译结果可一键复制，页面切换后会保留当前输入和结果。

### 外观主题

- Win11 深色
- 午夜玻璃
- 石墨白雾
- 清雅茶绿



## 快速开始

1. 在两台电脑上打开 CopyShare，并确保处于同一局域网。
2. 进入「设备连接」，等待自动发现或手动输入对方 IP 和端口。
3. 双方确认信任设备。
4. 在任意一台电脑复制文本、截图、图片或文件，另一台设备会自动接收。
5. 可在「剪贴板」页面查看最近同步内容，或通过浮窗快速复制。
6. 可进入「翻译」页面输入文本并选择目标语言，默认使用 Google 翻译；如需 AI 翻译，可在「设置」中切换并填写自有 API 配置。

## 常见问题

### 搜不到设备怎么办？

- 确认两台电脑连接到同一个 Wi-Fi 或局域网。
- 确认对方已经打开 CopyShare。
- 检查 Windows 防火墙是否允许 CopyShare 访问专用网络。
- 如果存在 VPN、虚拟网卡或网络隔离，建议先关闭后重试。
- 可手动输入对方 IP 和端口连接。

### 连接成功但不同步怎么办？

- 确认双方都已经信任设备。
- 检查首页状态是否为「同步中」。
- 确认对方设备仍在线。
- 检查防火墙是否拦截局域网连接。

### 文件保存在哪里？

默认保存到系统下载目录下的 `CopyShare` 文件夹。也可以在「设置」里的「下载位置」中修改。

### 手机扫码打不开怎么办？

- 确认手机和电脑在同一局域网。
- 确认手机浏览器可以访问电脑的局域网 IP。
- 检查 Windows 防火墙是否允许 CopyShare。

## 安全说明

- CopyShare 面向局域网使用，不需要公网服务器。
- 剪贴板内容不会上传到云端。
- 未信任设备不能接收剪贴板内容。
- 剪贴板可能包含密码、验证码或隐私内容，请只信任自己的设备。

## 开发

```bash
npm install
npm run dev
npm run build
npm run tauri:build
```

常用命令：

- `npm run dev`：启动前端开发服务
- `npm run tauri:dev`：启动桌面开发模式
- `npm run build`：构建前端
- `npm run build:exe`：生成单个 exe
- `npm run tauri:build`：生成安装包

## 版本

当前版本：`v3.0.0`
