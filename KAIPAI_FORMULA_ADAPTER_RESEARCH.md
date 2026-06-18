# 开拍网感模板公式适配调研

日期：2026-06-17

## 背景

dcoin 现有网感模板链路的问题是：用户选择开拍模板后，只能等待 Android App 黑盒导出结果，中间没有本地预览，也不能编辑。实际体验会变成选择模板后等待三四分钟，拿到 MP4 才知道效果是否合适。

Video Editor 的目标不是复用 Android worker 渲染，而是把开拍模板公式转成自己的 `.veproj/project.json` 草稿，让桌面编辑器可以立刻预览、调整文字/贴纸/片段/音频，再通过自己的 Rust/FFmpeg 渲染链路导出。

## 调研结论

结论：技术上可行，但必须按“受支持子集 + 兼容报告”推进，不能把全量开拍/剪映 native 效果一次性承诺为完全还原。

关键判断：

- Kaipai 公式在 Android 导出前已经拿到，不是只能从 Android MP4 结果反推。
- Android worker 后半段主要负责把公式、识别结果、源视频路径注入 patched App，然后等待 App 原生保存 MP4。
- 我们可以在“拿到公式和素材依赖”之后切出自己的 adapter：资源本地化、公式到 `.veproj` 映射、本地预览、本地编辑、本地导出。
- Android worker 后续更适合作为 oracle 和校准工具，不应该成为 Video Editor 产品运行时依赖。
- 当前 `.veproj` v1 schema 是 MVP 基础剪辑 schema，不是完整模板/特效 schema；Kaipai 模板适配需要补齐画布、视觉变换、贴纸、字体资源、资源包和兼容报告。

## dcoin 现有链路

相关路径：

- `/Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker`
- `/Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker/src/worker-task-executor.js`
- `/Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker/src/kaipai-app-api.js`
- `/Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker/src/kaipai-app-template.js`
- `/Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker/scripts/reverse-app-export-runner.js`
- `/Users/zhiwen/code/dcoin/src/editor-render-engine`

现有 reverse worker 大致流程：

1. 下载或准备源视频，读取素材元数据。
2. 获取或复用 App 形态的 `VoiceResult.word_list`。
3. 获取 App 人脸安全区 `safe_area`。
4. 选择 For You 或指定模板，拿到 `template id` 和 `recipe id`。
5. 调开拍 App API：
   - `/vg/smart_edit/hot_category_list.json`
   - `/vg/smart_edit_recipe/get_direct_material_list.json`
   - `/vg/smart_edit_recipe/formula.json`
   - `/vg/smart_edit_recipe/get_formula_result.json`
6. 拿到 `formulaResult.formula`。
7. 根据模板开关后处理公式，例如关闭 BGM、人物抠像、PIP、在线贴纸、movement。
8. 把公式交给 patched Android App 的本地 `/jobs/export` 保存导出。
9. 拉取 App 产出的 MP4 并上传。

这说明：公式是可截获、可保存、可转译的。旧链路慢和不可编辑的原因，不在于公式拿不到，而在于公式之后直接进入 Android App 黑盒保存。

## 公式能否转成 `.veproj`

可以做，但要分层：

1. 开拍接口层只负责拿模板、公式和素材依赖。
2. 资源本地化层负责下载或缓存公式引用的视频、图片、字体、贴纸、音乐等资源。
3. `adapter_kaipai` 负责把受支持字段转成内部草稿。
4. 兼容报告记录 unsupported/degraded 字段。
5. 预览和导出都走 Video Editor 自己的 render graph 与 FFmpeg runtime。

不要让 UI、render graph 或 ffmpeg compiler 直接理解 Kaipai 原始 JSON。Kaipai 原始字段应该是外部来源和兼容信息，不应该成为内部渲染语义。

## 当前 `.veproj` schema 状态

当前 v1 schema 已经能覆盖 MVP 基础编辑语义：

- `Draft`
- `Material`
- `Track`
- `Segment`
- `SourceTimerange`
- `TargetTimerange`
- `MainTrackMagnet`
- `Keyframe`
- `Filter`
- `Transition`
- 基础 `TextSegment`
- 基础音量 `SegmentVolume`
- `video/image/audio/text/sticker` 素材类型
- `video/audio/text/sticker/filter` 轨道类型

相关项目路径：

- `schemas/draft.schema.json`
- `schemas/command.schema.json`
- `crates/draft_model/src/draft.rs`
- `crates/draft_model/src/material.rs`
- `crates/draft_model/src/timeline.rs`
- `apps/desktop-electron/src/generated/Draft.ts`

但 Kaipai 网感模板需要的下面这些语义还没有：

- 草稿级 canvas：宽高、fps、背景、比例、适配模式、坐标系统。
- 结构化 transform：位置、锚点、缩放、旋转、透明度、翻转、裁剪、fit/crop、z-order、blend mode。
- 贴纸模型：图片贴纸、视频贴纸、动图贴纸、贴纸资源包、贴纸入出场动画、边界盒。
- 字体资源：字体素材类型、字体路径、字体族、fallback、字重、行距、字距。
- 更完整文字贴纸：多段文字、描边/阴影单位、发光、背景、文字模板、气泡、花字、文字动画。
- 资源 bundle：远程 URL、本地缓存路径、sha256、来源、外部 proprietary id、授权/下载状态。
- 兼容报告：supported、degraded、unsupported、missing-resource、needs-native-effect 等分类。
- adapter provenance：保留 Kaipai template id、recipe id、formula task id、原始公式摘要。

因此，v1 schema 可以继续服务当前 MVP 编辑器，但不应该把 Kaipai 原始 JSON 强塞进 v1 的 `Filter.parameters` 或 `Keyframe.value` 字符串里。

## dcoin editor-render-engine 的价值

`/Users/zhiwen/code/dcoin/src/editor-render-engine` 不是完整本地 Kaipai 渲染器。当前核心 renderer 仍有 unsupported 行为，只有很窄的 pure-video / crop-placement 实验路径。

它的主要价值是测试方法和样例数据：

- `input/project.json` 捕获了 App 风格的 `androidProject`。
- `input/resources.json` 证明字体、PIP 视频、部分图片资源可以本地化并 sha256 校验。
- `android-oracle/output.mp4` 和固定时间点 PNG 帧可以作为对照。
- fixture import、ffprobe、抽帧、PNG diff 的方法可以复用到 Video Editor 的 adapter 测试。

已观察到的可映射字段：

- `videoCanvasConfig.width/height/frameRate/videoBitrate`：可映射到 canvas/output 设置。
- `videoClipList[].originalFilePath/startAtMs/endAtMs/durationMsWithSpeed/scale/rotate/alpha/volume`：可映射到主轨视频片段。
- `pipList[].start/duration/level/firstImgPath/videoClip.*`：可映射到 PIP/覆盖视频轨道。
- `stickerList[].start/duration/relativeCenterX/relativeCenterY/scale/rotate/alpha/level/textEditInfoList`：可映射到文字贴纸或普通贴纸。
- `textEditInfoList[].text/fontName/fontPath/textColor/textStrokeColor/textStrokeWidth/showShadow/shadow*`：可映射到文字贴纸样式。
- `resources[].kind=font` 和 `resources[].kind=video`：证明字体和 PIP 视频本地化路径可行。

限制：

- 现有图片贴纸样例没有完成 Android oracle 闭环，不能直接宣称图片贴纸语义已完全理解。
- 音乐/BGM 样例证据不足。
- native 特效、抠像、美颜、复杂转场、气泡、花字、场景特效仍需要逐项校准。

## 建议的 adapter 分层

建议新增独立模块，而不是混进 UI 或 FFmpeg runtime：

```text
adapter_kaipai
  ├─ KaipaiTemplateProvider
  │   ├─ 推荐模板 / 指定模板
  │   ├─ direct material list
  │   └─ formula submit / poll
  ├─ KaipaiFormulaBundle
  │   ├─ formula
  │   ├─ template id / recipe id
  │   ├─ word_list / safe_area
  │   ├─ direct materials
  │   └─ provenance
  ├─ KaipaiResourceLocalizer
  │   ├─ 下载视频、图片、字体、音乐、贴纸资源
  │   ├─ sha256 校验
  │   ├─ 写入 .veproj/resources/
  │   └─ 缺失资源诊断
  ├─ KaipaiDraftMapper
  │   ├─ 受支持字段转 Draft/Material/Track/Segment
  │   └─ 不支持字段转 CompatibilityReport
  └─ KaipaiCompatibilityReport
      ├─ supported
      ├─ degraded
      ├─ unsupported
      ├─ missingResource
      └─ needsNativeEffect
```

运行时目标：

```text
Kaipai API
  -> formula bundle
  -> resource localizer
  -> adapter_kaipai
  -> .veproj/project.json
  -> Video Editor preview
  -> user edits
  -> render graph
  -> FFmpeg export
```

## 第一批可支持功能

适合先支持这些，因为它们能直接解决“选模板后立刻预览并可编辑”的体验问题：

1. 主视频片段
   - source/target 时间段
   - 速度
   - 音量
   - 基础 fit/crop

2. 画布
   - 宽高
   - fps
   - 背景色
   - 竖屏比例

3. PIP / 视频覆盖层
   - start/duration
   - scale
   - position
   - opacity
   - z-order

4. 文字贴纸
   - 文本内容
   - 字体文件
   - 字号
   - 颜色
   - 描边
   - 阴影
   - 基础位置/旋转/缩放

5. 图片贴纸
   - 本地图片资源
   - start/duration
   - position
   - scale
   - rotate
   - opacity

6. 兼容报告
   - 先让用户和测试知道哪些效果被保留、哪些被降级、哪些暂不支持。

## 暂不建议承诺的功能

这些应该先进兼容报告，而不是作为 POC 必达目标：

- native 人物抠像
- 美颜/瘦脸/磨皮
- AR/Scene 特效
- 复杂 movement
- 花字模板
- 气泡模板
- 复杂文字动画
- 复杂转场
- 专有滤镜精确还原
- App native ImageKit/TextFilter/ARKernel 效果完全像素级还原

## 可以单独推进的功能

下面这些功能可以不等完整 Kaipai adapter，全都可以拆成独立阶段或独立 plan 推进。

### 1. Draft v2 视觉语义基础

目标：补齐模板预览和导出的最低通用语义。

可独立交付：

- `Draft.canvas`
- `Segment.transform`
- `Segment.layer`
- `MaterialKind::Font`
- `StickerSegment`
- 字体资源引用
- 兼容报告基础类型
- v1 -> v2 migration

为什么可独立：这些不是 Kaipai 专属，后续剪映导入、手动贴纸、字幕、移动端和服务端渲染都要用。

### 2. 资源 bundle / 本地化系统

目标：让 `.veproj` 可以稳定携带远程模板依赖。

可独立交付：

- `.veproj/resources/` 目录约定
- 资源 manifest
- 远程 URL 下载
- sha256 校验
- 缺失资源诊断
- 相对路径保存
- 断网打开验证

为什么可独立：不依赖完整渲染，也不依赖 UI。只要能把资源下载成本地引用，就能先解决模板草稿可复现问题。

### 3. CompatibilityReport

目标：外部模板导入时明确告诉系统和用户哪些能力被保留、降级或丢弃。

可独立交付：

- report schema
- supported/degraded/unsupported 分类
- missing-resource 分类
- needs-native-effect 分类
- adapter 输出快照测试
- UI 后续展示入口

为什么可独立：不需要先实现所有效果，反而能保护 adapter 不误报能力。

### 4. Adapter fixture corpus

目标：先积累真实样本，避免凭想象设计 schema。

可独立交付：

- 保存 Kaipai formula 样本
- 保存 direct material list 样本
- 保存 recognizer word_list 和 safe_area 样本
- 保存资源清单
- 隐私和 token 清理脚本
- golden snapshot

为什么可独立：它是后续 mapper、schema、render parity 的输入，不需要 UI。

### 5. Kaipai provider 边界

目标：把“调用 Kaipai 接口”封装为可替换能力。

可独立交付：

- `fetchHotSmartEditCategories`
- `fetchDirectMaterialList`
- `submitFormula`
- `pollFormulaResult`
- timeout/retry/error 分类
- mock provider 测试

为什么可独立：可以先复用 dcoin 经验，但不要把 dcoin worker 业务流程带进 Video Editor。

### 6. safe_area / ASR 替代方案 spike

目标：降低对 Android App 的前置依赖。

可独立交付：

- App `VoiceResult.word_list` 结构兼容器
- 我们自己的 ASR 输出到 word_list 转换
- safe_area 默认/空值实验
- 人脸检测 safe_area 替代方案
- 模板接口对 safe_area 的容忍度测试

为什么可独立：这是“只依赖 Kaipai 接口，不依赖 Android worker”的关键风险点。

### 7. Oracle 测试工具迁移

目标：把 dcoin editor-render-engine 的对照方法迁移成 Video Editor 测试工具。

可独立交付：

- ffprobe 元数据对比
- ffmpeg 抽帧
- PNG diff
- frame heatmap
- fixture manifest 校验
- Android oracle 只作为测试输入

为什么可独立：即使 adapter 未完成，也可以先建立未来效果校准工具。

### 8. PIP / 覆盖视频渲染

目标：支持模板里最常见、最容易产生“网感”的覆盖层。

可独立交付：

- 多视频输入 render graph
- overlay filter graph
- position/scale/opacity
- layer order
- preview/export parity

依赖：需要先有基础 `canvas` 和 `transform`。

### 9. 字体和文字贴纸渲染

目标：让模板字幕和文字贴纸能预览、编辑、导出。

可独立交付：

- 字体文件注册
- 文本布局基础规则
- 描边/阴影
- ASS 或 image overlay 路径取舍
- 文字样式 snapshot

依赖：需要字体资源和 transform 基础。

### 10. 图片贴纸渲染

目标：支持静态 PNG/JPG/WebP 贴纸。

可独立交付：

- image material
- sticker segment
- position/scale/rotate/opacity
- z-order
- missing sticker fallback

依赖：需要资源 bundle、sticker segment 和 transform。

## 建议推进顺序

如果不打断当前 v1 MVP 主线：

1. Phase 4 继续做中文桌面编辑器 UI 和命令链路。
2. Phase 5 继续做 preview/export 主路径。
3. Phase 5.x 插入 `Template Semantics Foundation`：
   - canvas
   - transform
   - resource bundle
   - compatibility report
4. Phase 5.y 插入 `Kaipai Formula Adapter POC`：
   - 真实公式 fixture
   - 资源本地化
   - 公式转 `.veproj`
   - PIP/文字/图片贴纸受支持子集
   - 本地预览
   - 本地导出
5. Phase 6 再做 MVP hardening 和 packaged smoke。

如果业务上想更早验证 Kaipai 体验，可以先做一个不进主线的 spike：

- 输入：一条真实源视频 + 一个真实 formula bundle。
- 输出：一个 `.veproj` 草稿 + compatibility report。
- 验收：桌面端能打开草稿，预览首帧，至少能编辑文字内容和隐藏/移动一个贴纸或 PIP。

## 最小 POC 验收标准

最小 POC 不要求像素级完全还原，但必须证明体验闭环：

1. 给定源视频和 Kaipai formula bundle，可以生成 `.veproj/project.json`。
2. 所有受支持资源都下载到本地 bundle，并使用相对路径。
3. 草稿能通过 schema 和 Rust validation。
4. 桌面编辑器能打开草稿。
5. 预览能显示主视频 + 至少一种覆盖层。
6. 用户能编辑至少一个模板字段，例如文字内容、文字位置、PIP 显隐或贴纸位置。
7. 导出不依赖 Android worker。
8. 输出 compatibility report，明确列出 unsupported effects。

## 关键风险

- `safe_area` 当前依赖 App evidence，完全脱离 Android 前需要替代方案。
- Kaipai formula 里的资源 URL 和 direct material list 字段需要真实样本校准。
- 图片贴纸、BGM、文字模板的真实字段覆盖还不够。
- native 效果不应该在早期被承诺为可还原。
- 如果没有 compatibility report，用户会误以为模板 100% 还原，后续维护成本会很高。

## 当前建议

当前 v1 MVP 仍在推进，建议不要改 Phase 4 的目标。最务实的做法是：

- 短期：继续完成桌面编辑器和 preview/export 主链路。
- 并行：启动资源 bundle、compatibility report、fixture corpus 这三件独立工作。
- 中期：在 Phase 5 后插入模板语义基础和 Kaipai adapter POC。
- 长期：按兼容报告逐步扩大支持范围，而不是追求一次性还原所有开拍 native 效果。
