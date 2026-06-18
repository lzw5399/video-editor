<p align="right">
  <a href="./README.md"><kbd>English</kbd></a>
</p>

# Video Editor

Video Editor 是一个桌面优先、体验接近剪映/CapCut 的视频编辑器，同时也是一个完全自研的 Rust 视频编辑与渲染引擎。

Electron 桌面端是第一个已经落地的客户端，但这个项目真正重要的价值在底层引擎：统一的草稿模型、统一的时间线命令模型、统一的渲染图和统一的 FFmpeg 编译路径。移动端、服务端渲染，以及外部草稿兼容适配，都可以复用同一套编辑语义。

## 架构

![Video Editor architecture](./docs/assets/architecture.png)

架构里有两条不同的链路：

1. 主编辑与渲染链路：
   应用端调用 API/Binding 层，Rust Core 负责草稿和时间线语义，图形/组合层把编辑状态解析成渲染意图，编码编译层把渲染图转换成 FFmpeg 执行计划，运行时层负责预览和导出任务。
2. 兼容适配链路：
   外部剪映/CapCut 草稿先进入 Adapter，Adapter 只负责把外部工程映射成 Video Editor 自己的 `.veproj/project.json`，并输出兼容性报告。桌面端正常编辑时不会经过适配层。

## 分层说明

| 层 | 职责 | 当前形态 |
| --- | --- | --- |
| 应用层 | 产品交互、文件选择、工作区布局、命令派发 | `apps/desktop-electron` 已作为第一个客户端实现；移动端和服务端是可接入的扩展位 |
| API / Binding 层 | 稳定命令信封、面向客户端的服务 API | `bindings_node` 把 Rust 拥有的契约暴露给 Electron；移动端/服务端 Binding 可以复用同一核心 |
| 兼容适配层 | 把外部工程格式映射成内部规范项目，并报告不兼容能力 | `adapter_jianying` 是接下来的兼容实践方向；它把剪映/CapCut 草稿导入 `.veproj`，不会成为内部渲染语义 |
| 核心语义层 | 草稿、素材、轨道、片段、关键帧、滤镜、转场语义，整数时间模型，命令校验，撤销/重做 | `draft_model`、`draft_commands`、`engine_core` 负责编辑模型 |
| 图形 / 组合层 | 把时间线状态解析成类型化渲染意图，与 FFmpeg 细节隔离 | `render_graph` 和 `engine_core` 保持组合决策独立于进程执行 |
| 编码编译层 | 把渲染图编译成 FFmpeg 输入、滤镜脚本、字幕和编码参数 | `ffmpeg_compiler` 负责生成 FFmpeg 执行计划；UI 不直接拼 FFmpeg 命令 |
| 媒体运行时层 | 发现并执行 ffmpeg/ffprobe，回传进度、错误和取消边界 | `media_runtime` 定义运行时 trait；`media_runtime_desktop` 提供桌面端实现 |
| 项目与派生产物层 | 持久化规范项目，把生成产物排除在语义源之外 | `.veproj/project.json` 是唯一规范源；渲染图、FFmpeg 脚本、缩略图、波形、预览缓存、代理文件、导出文件都是派生产物 |

## 当前状态

| 模块 | 状态 |
| --- | --- |
| Rust workspace 和工具链 | 已实现 |
| 草稿、素材、项目包、Schema 模型 | 已实现 |
| 时间线命令核心、吸附、主轨磁吸、撤销/重做 | 已实现 |
| Electron 桌面工作区 | 已作为第一个客户端实现 |
| 预览与导出流水线 | 开发中 |
| 打包、FFmpeg 分发合规、发布加固 | 计划中 |
| 剪映/CapCut 兼容适配器 | 下一项明确的适配实践 |
| 移动端 App 和服务端渲染客户端 | 架构扩展位，不是当前产品承诺 |

## 仓库结构

```text
apps/desktop-electron/      Electron + React + TypeScript 桌面编辑器
crates/draft_model/         规范草稿/素材/时间线 schema 与时间模型
crates/draft_commands/      时间线编辑命令、吸附、撤销/重做
crates/engine_core/         草稿归一化与帧/时间线求值
crates/render_graph/        类型化渲染图与渲染意图模型
crates/ffmpeg_compiler/     渲染图到 FFmpeg 执行计划
crates/media_runtime/       ffmpeg/ffprobe trait、发现、任务、错误
crates/media_runtime_desktop/ 桌面端 FFmpeg 进程实现
crates/project_store/       .veproj 打开、保存、自动保存、相对路径处理
crates/preview_service/     预览、缩略图、波形和缓存边界
crates/bindings_node/       面向 Electron 的 Node-API 层
crates/testkit/             Fixture、Golden、渲染冒烟测试工具
schemas/                    生成的 JSON Schema
fixtures/                   正向和负向草稿/项目 fixture
docs/                       架构与运行时边界文档
```

## 快速开始

环境要求：

- Rust `1.95.0`
- Node.js `24.12.0`
- pnpm `10.32.1`
- `just`，仅用于可选的根命令 recipe
- 运行时和渲染冒烟测试需要 `PATH` 里有 FFmpeg/ffprobe，或配置
  `VE_FFMPEG_PATH` 和 `VE_FFPROBE_PATH`

```bash
nvm use
corepack enable
pnpm start
```

`pnpm start` 会按锁文件安装依赖、构建 Electron 桌面端，然后启动编辑器。
如果你习惯用 `just`，也可以运行 `just start`。

构建：

```bash
just build
```

运行完整本地检查：

```bash
just test
```

常用的局部检查：

```bash
pnpm run test:rust
pnpm run test:desktop
pnpm run test:bindings
pnpm run test:render-smoke
```

## 项目格式

Video Editor 项目以 `.veproj` 包保存。唯一规范语义源是：

```text
.veproj/project.json
```

以下内容都是派生产物，不能写成持久化语义：

- 渲染图
- FFmpeg 脚本
- 缩略图
- 波形数据
- 代理文件
- 预览缓存
- 导出视频
- 原始 ffprobe JSON

## 开发边界

- UI 只派发命令；Rust 拥有项目和时间线语义。
- UI 代码不能直接构造 FFmpeg 命令。
- 持久化语义里的时间计算使用整数微秒、帧索引或有理数帧率。
- 模型、IPC、UI、测试和文档优先使用剪映风格术语：草稿、素材、轨道、片段、关键帧、滤镜、转场。
- 外部草稿 ID 只是兼容引用，不会变成内部渲染语义。
- Kdenlive、MLT、pyJianYingDraft 只是概念参考。本项目不复制 GPL 代码、资产、XML 定义、预设或 UI 实现。

## 兼容适配方向

剪映/CapCut Adapter 是映射层，不是引擎本身。它负责读取外部草稿结构，把可兼容的子集转换成 `.veproj/project.json`，必要时保留外部引用，并为不支持的特效、转场、滤镜、素材或时间特性输出兼容性报告。

映射完成后，导入项目会和 Video Editor 原生草稿一样，进入同一条核心引擎链路。

## License

Video Editor 使用 [MIT License](./LICENSE) 开源。

如果后续版本分发 FFmpeg 二进制文件，发布前还需要审查 LGPL/GPL/nonfree 构建选项、notice、源码提供义务和商业分发约束。
