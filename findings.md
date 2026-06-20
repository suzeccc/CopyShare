# 发现与决策

## 需求
- 文本剪贴板监听。
- 通过 `ws://` 进行 WebSocket 局域网通信。
- 自动同步剪贴板内容。
- 去重与防循环。
- 手动输入对端设备 IP/URL 连接。
- 自动重连。
- 默认端口 `8765`。
- MVP 不包含图片剪贴板、文件同步、云同步、自动发现、历史记录和移动端支持。

## 调研发现
- 当前工作区包含 `clipboard-sync-design.md`，起初没有源代码项目。
- 没有 `.codegraph/` 目录，因此 CodeGraph 工具不适用。
- `git status` 失败，因为当前工作区不是 git 仓库。
- `rg` 存在但在当前环境中被拒绝运行。
- Python 3.14.0 可用。

## 技术决策
| 决策 | 理由 |
|------|------|
| 构建 Python CLI 版 MVP | 在当前空工作区里，这是最快得到可运行工具和测试的路径。 |
| 避免第三方依赖 | 网络受限，且没有现成依赖清单。 |
| 使用 `asyncio` streams 实现 WebSocket 网络层 | 标准库异步 socket 足够支撑 MVP。 |
| 协议消息使用 JSON 字典 | 与设计文档完全一致。 |
| 内容 hash 使用 `format + content` 的 SHA-256 | 符合设计文档，并支持重复内容抑制。 |

## 遇到的问题
| 问题 | 处理方式 |
|------|----------|
| 没有既有 Electron 或 Tauri 项目 | 先实现无依赖 CLI 核心，后续可再封装进 Electron/Tauri。 |
| 剪贴板 API 与平台相关 | 提供剪贴板抽象；Windows 运行时使用 PowerShell，测试使用内存剪贴板。 |
| brainstorming 技能中的 commit 步骤无法执行 | 记录限制；当前没有 `.git` 目录。 |

## 资源
- 源设计文档：`D:/QiLin/Copy share/clipboard-sync-design.md`
- 派生设计文档：`docs/superpowers/specs/2026-06-20-lan-clipboard-sync-design.md`
- 实施计划：`docs/superpowers/plans/2026-06-20-lan-clipboard-sync.md`

## 视觉/浏览器发现
- 本任务没有使用视觉或浏览器资料。
