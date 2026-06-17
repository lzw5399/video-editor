# 开拍公式兼容推进方案

日期：2026-06-17

## 当前结论

这条线应该按 GSD spike 推进，而不是直接进入功能实现。原因是它属于边缘兼容链路，不是当前桌面编辑器核心；真正要验证的是边界：我们能不能在不接开拍实时 API、不依赖 Android worker 渲染的情况下，承载已经生成好的开拍公式、素材引用和证据，然后转成自己的 `.veproj/project.json` 草稿。

结论是：技术上可行，但第一步必须限定为“离线公式包输入 + 受支持子集 + 兼容报告”。不要承诺开拍/剪映 native 效果 100% 还原。

## 目标体验

dcoin 原链路的问题是：用户选模板后直接进入 Android App 黑盒导出，等待几分钟后才看到 MP4，中间不能预览也不能编辑。

Video Editor 这边要做的是：

```text
开拍公式包
  -> 资源本地化
  -> 适配成 .veproj 草稿
  -> 桌面端预览
  -> 用户编辑文字/贴纸/PIP/音频等
  -> 自己的 Rust/Render Graph/FFmpeg 链路导出
```

Android worker 后续只适合当 oracle 和校准来源，不应该成为产品运行时依赖。

## 已确认的 dcoin 证据

关键路径：

- `/Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker/src/worker-task-executor.js`
- `/Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker/src/kaipai-app-template.js`
- `/Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker/android-entry/src/com/meitu/action/dcoin/reverse/KpReverseRecognizerStore.java`
- `/Users/zhiwen/code/dcoin/src/editor-render-engine`

证据：

- `prepareKaipaiSmartEditFormula()` 在 Android 导出前已经拿到 `formula`。
- 公式生成需要 `recipe_id`、源视频信息、`VoiceResult.word_list`、`safe_area`、模板开关等输入。
- `safe_area` 当前来自 patched App 的人脸检测窗口，是公式生成证据，不是时间线渲染语义。
- editor-render-engine 的 fixture 已经证明字体、PIP 视频、源视频等资源可以用本地路径和 sha256 manifest 记录。
- editor-render-engine 更适合作为 Android oracle / pixel diff / fixture 方法参考，不应被当成当前 Video Editor 的 runtime。

## safe_area 放哪一层

`safe_area` 不应该放进 Rust 核心库的 `draft_model` 作为核心编辑语义，也不应该由 `engine_core` 或 `render_graph` 去计算。

建议分层：

| 情况 | 放置位置 | 原因 |
|---|---|---|
| 已经拿到公式 | `KaipaiFormulaBundle.safeArea` / provenance | 只用于说明这个公式怎么来的 |
| 还需要生成公式 | `SafeAreaProvider` / preprocess service | 这是 provider 输入证据，可替换 App、人脸检测、默认策略 |
| 公式映射后影响画面 | 映射成具体草稿语义，例如画布调整、片段 transform、裁剪 | Rust 核心只关心最终可编辑/可渲染的语义 |
| 用户可编辑安全框 | 未来作为单独工具或素材分析结果 | 不和开拍公式证据混在一起 |

所以当前答案是：`safe_area` 检测不放在 Rust 草稿核心。它最多由 adapter/preprocess 层生成和记录；核心只消费 adapter 产出的草稿语义。

## 建议新增边界

先不要接开拍 API。先定义并验证一个离线公式包：

```text
KaipaiFormulaBundle
  ├─ formula
  ├─ templateId / recipeId / formulaTaskId / formulaRequestId
  ├─ sourceMedia
  ├─ recognizerResult.word_list
  ├─ safeArea evidence
  ├─ directMaterials
  └─ resources
```

后续模块边界：

```text
adapter_kaipai
  ├─ FormulaBundle 读取和校验
  ├─ ResourceLocalizer 资源下载/复制/sha256/相对路径
  ├─ DraftMapper 受支持字段转 .veproj
  └─ CompatibilityReport 记录支持/降级/不支持/缺失资源/native 效果
```

不要让 UI、Render Graph、FFmpeg Compiler 直接理解开拍原始 JSON。

## 当前 schema 缺口

当前 v1 `.veproj` 已经支持基础编辑：

- 草稿、素材、轨道、片段
- source/target time range
- 主轨磁吸
- 基础文字
- 基础音量
- keyframe/filter/transition 占位

但开拍公式适配至少还缺：

- `Draft.canvas`：宽高、fps、背景、比例/坐标策略。
- 字体素材：字体文件、本地路径、字体族、fallback。
- 资源 manifest：远程来源、本地相对路径、sha256、下载状态、资源类型。
- 画面调节/transform：位置、锚点、缩放、旋转、透明度、裁剪、适配、翻转、层级、混合模式。
- 贴纸和文字贴纸：图片/视频/GIF 贴纸、文字贴纸、字体、描边、阴影、气泡、花字、动画降级。
- 兼容报告：supported、degraded、unsupported、missing-resource、needs-native-effect。
- 外部来源证明：template id、recipe id、formula task id、raw formula digest。

这些不是开拍专属。它们也是剪映草稿导入、手动贴纸、字幕、移动端、服务端渲染都会用到的模板语义基础。

## 可单独推进的功能

### 1. 公式样本和 fixture corpus

目标：先收集真实公式，不凭想象设计 schema。

交付：

- 脱敏后的 formula bundle 样本。
- direct material list 样本。
- recognizer `word_list` 样本。
- safe_area evidence 样本。
- 资源清单和 sha256。
- Android oracle 输出只作为对照，不作为 runtime。

### 2. CompatibilityReport

目标：让 adapter 不会误报能力。

交付：

- report schema。
- supported/degraded/unsupported/missing-resource/needs-native-effect 分类。
- adapter snapshot 测试。
- 后续 UI 展示入口。

### 3. 资源 bundle / 本地化

目标：让 `.veproj` 可以稳定携带模板依赖。

交付：

- `.veproj/resources/` 约定。
- 资源 manifest。
- 远程 URL 下载或本地复制。
- sha256 校验。
- 相对路径保存。
- 缺失资源诊断。

### 4. Draft v2 模板语义基础

目标：补齐预览和导出模板所需的通用语义。

交付：

- `Draft.canvas`
- 字体素材
- 资源引用
- 画面调节/transform
- sticker/text sticker segment
- 兼容报告基础类型
- v1 -> v2 migration

### 5. 离线 Kaipai adapter POC

目标：给定一个 formula bundle，可以生成 `.veproj/project.json` 和兼容报告。

第一版支持：

- 主视频片段。
- 画布。
- PIP/视频覆盖层。
- 基础文字贴纸。
- 图片贴纸。
- 字体资源引用。
- unsupported/native effect 报告。

### 6. safe_area / ASR 替代 spike

目标：以后如果要脱离 Android worker 生成公式，需要替代 App evidence。

交付：

- `VoiceResult.word_list` 兼容器。
- 我们自己的 ASR 输出到 word_list 的转换。
- safe_area 空值/默认值/人脸检测策略实验。
- 开拍公式接口对 safe_area 的容忍度测试。

这件事不应该阻塞“离线公式导入”。

### 7. Oracle 测试工具迁移

目标：复用 dcoin editor-render-engine 的测试方法。

交付：

- ffprobe 元数据对比。
- ffmpeg 抽帧。
- PNG diff / heatmap。
- fixture manifest 校验。
- Android oracle 对照报告。

## 推荐推进顺序

当前不要打断桌面编辑器主线。建议并行插入一条兼容准备线：

1. 先做 fixture corpus 和 compatibility report。
2. 再做资源 bundle/localizer。
3. 再做 Draft v2 模板语义基础。
4. 再做离线 Kaipai adapter POC。
5. 最后再考虑 Kaipai provider/API 接入。

最小 POC 验收：

- 给定源视频和公式包，可以生成 `.veproj/project.json`。
- 受支持资源都进入本地资源目录，并用相对路径引用。
- 草稿通过 schema/Rust validation。
- 桌面端能打开草稿。
- 预览能显示主视频和至少一种覆盖层。
- 用户能编辑至少一个模板字段，例如文字内容、贴纸位置或 PIP 显隐。
- 导出不依赖 Android worker。
- 输出兼容报告，明确列出暂不支持的 native 效果。

## GSD 记录

本次已创建 spike：

- `.planning/spikes/001-kaipai-formula-bundle-boundary/README.md`

Spike 结论：离线公式包边界成立，`safe_area` 属于 provider/preprocess/provenance，不属于 Rust 核心草稿语义；下一步应先做样本、兼容报告、资源本地化和 Draft v2 模板语义基础。
