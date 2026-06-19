# SVG 资源说明

总计：216 个 SVG。

## 目录总览

| 本地目录 | SVG 数量 | 编辑器位置 | 主要作用 |
|---|---:|---|---|
| `Canvas/TrackHeader` | 20 | 时间线左侧轨道头 | 轨道类型、显示/隐藏、锁定、静音、独奏等按钮图标 |
| `ToolBar` | 82 | 时间线上方工具栏 | 分割、删除、撤销、缩放、吸附、录音、标记、波形、智能粗剪等操作按钮 |
| `Canvas` | 72 | 时间线画布区域 | 片段状态、关键帧、模板、字幕、转场、组合、加载、错误、拖拽辅助等图标 |
| `Cursor` | 21 | 时间线交互光标/拖拽状态 | 裁剪、选择、移动、轨道头展开收起、吸附拖拽等鼠标状态图标 |
| `Template` | 7 | 模板时间线/模板片段区域 | 模板入口、添加、收起、替换、警告等图标 |
| `segment` | 5 | 时间线片段内部状态 | 片段锁定/解锁、冻结帧、文件夹、视频图表等图标 |
| `MaterialReplacement` | 5 | 素材替换流程 | 播放、暂停、多视频、音量、卡点高亮等图标 |
| `Drag` | 4 | 拖入时间线时的素材类型 | 拖拽音频、视频、图片、文件夹时的类型提示图标 |

## 轨道头图标

左侧轨道列表的图标。

| 文件 | 编辑器位置 | 作用 |
|---|---|---|
| `Canvas/TrackHeader/video.svg` | 时间线左侧视频轨道头 | 表示视频轨道 |
| `Canvas/TrackHeader/audio.svg` | 时间线左侧音频轨道头 | 表示音频轨道 |
| `Canvas/TrackHeader/text.svg` | 时间线左侧文本轨道头 | 表示文字轨道 |
| `Canvas/TrackHeader/text_flower.svg` | 时间线左侧文本/花字轨道头 | 表示花字或特殊文本轨道 |
| `Canvas/TrackHeader/sticker.svg` | 时间线左侧贴纸轨道头 | 表示贴纸轨道 |
| `Canvas/TrackHeader/effect.svg` | 时间线左侧特效轨道头 | 表示特效轨道 |
| `Canvas/TrackHeader/filter.svg` | 时间线左侧滤镜轨道头 | 表示滤镜轨道 |
| `Canvas/TrackHeader/adjust.svg` | 时间线左侧调节轨道头 | 表示调节轨道 |
| `Canvas/TrackHeader/hideOn.svg` | 轨道头眼睛按钮 | 显示/隐藏轨道的状态图标 |
| `Canvas/TrackHeader/hideOff.svg` | 轨道头眼睛按钮 | 显示/隐藏轨道的另一状态图标 |
| `Canvas/TrackHeader/lockOn.svg` | 轨道头锁按钮 | 锁定轨道状态 |
| `Canvas/TrackHeader/lockOff.svg` | 轨道头锁按钮 | 未锁定轨道状态 |
| `Canvas/TrackHeader/muteOn.svg` | 音频轨道头喇叭按钮 | 静音状态 |
| `Canvas/TrackHeader/muteOff.svg` | 音频轨道头喇叭按钮 | 非静音状态 |
| `Canvas/TrackHeader/solo_on.svg` | 音频轨道头 `S` 按钮 | 独奏/仅播放该轨状态 |
| `Canvas/TrackHeader/solo_off.svg` | 音频轨道头 `S` 按钮 | 未独奏状态 |
| `Canvas/TrackHeader/btncoverEdit.svg` | 轨道头封面相关按钮 | 编辑封面 |
| `Canvas/TrackHeader/btncoverDel.svg` | 轨道头封面相关按钮 | 删除封面 |
| `Canvas/TrackHeader/ai_clipper.svg` | 轨道头 AI 相关入口 | AI 剪辑/智能剪辑入口 |
| `Canvas/TrackHeader/ai_clipper_grey.svg` | 轨道头 AI 相关入口 | AI 剪辑不可用或弱化状态 |

## 时间线工具栏

| 文件示例 | 编辑器位置 | 作用 |
|---|---|---|
| `ToolBar/addTimeline.svg` | 时间线工具栏 | 添加时间线/轨道相关入口 |
| `ToolBar/cutting.svg` | 时间线工具栏 | 分割/切割工具 |
| `ToolBar/cutoff.svg` | 时间线工具栏 | 截断/切断相关操作 |
| `ToolBar/trim_clip.svg` | 时间线工具栏 | 裁剪片段 |
| `ToolBar/prune.svg` | 时间线工具栏 | 修剪/精剪相关操作 |
| `ToolBar/del.svg` | 时间线工具栏 | 删除 |
| `ToolBar/undo.svg` | 时间线工具栏 | 撤销 |
| `ToolBar/redo.svg` | 时间线工具栏 | 重做 |
| `ToolBar/btnZoomIn.svg` | 时间线工具栏 | 放大时间线 |
| `ToolBar/btnZoomOutN.svg` | 时间线工具栏 | 缩小时间线 |
| `ToolBar/btnZoomFitAdjust.svg` | 时间线工具栏 | 缩放到合适视图 |
| `ToolBar/btnOpenAdsorb.svg` | 时间线工具栏 | 开启吸附 |
| `ToolBar/btnCloseAdsorb.svg` | 时间线工具栏 | 关闭吸附 |
| `ToolBar/btnOpenMainTrackAdsorb.svg` | 时间线工具栏 | 开启主轨吸附 |
| `ToolBar/btnCloseMainTrackAdsorb.svg` | 时间线工具栏 | 关闭主轨吸附 |
| `ToolBar/buttonLinkage_on.svg` | 时间线工具栏 | 联动开启 |
| `ToolBar/buttonLinkage_off.svg` | 时间线工具栏 | 联动关闭 |
| `ToolBar/btnRecord.svg` | 时间线工具栏 | 录音入口 |
| `ToolBar/btnRecordStart.svg` | 时间线工具栏 | 开始录音 |
| `ToolBar/btnRecordStop.svg` | 时间线工具栏 | 停止录音 |
| `ToolBar/record_countdown_1.svg` | 录音倒计时 | 倒计时 1 |
| `ToolBar/record_countdown_2.svg` | 录音倒计时 | 倒计时 2 |
| `ToolBar/record_countdown_3.svg` | 录音倒计时 | 倒计时 3 |
| `ToolBar/mic_status_warning.svg` | 录音/麦克风状态 | 麦克风异常或提示 |
| `ToolBar/mic_status_help.svg` | 录音/麦克风状态 | 麦克风帮助提示 |
| `ToolBar/volume.svg` | 时间线工具栏 | 音量入口 |
| `ToolBar/wave_30_normal.svg` | 时间线工具栏 | 波形显示比例 30，普通状态 |
| `ToolBar/wave_30_selected.svg` | 时间线工具栏 | 波形显示比例 30，选中状态 |
| `ToolBar/wave_60_normal.svg` | 时间线工具栏 | 波形显示比例 60，普通状态 |
| `ToolBar/wave_60_selected.svg` | 时间线工具栏 | 波形显示比例 60，选中状态 |
| `ToolBar/wave_100_normal.svg` | 时间线工具栏 | 波形显示比例 100，普通状态 |
| `ToolBar/wave_100_selected.svg` | 时间线工具栏 | 波形显示比例 100，选中状态 |
| `ToolBar/btnAddMark.svg` | 时间线工具栏 | 添加标记 |
| `ToolBar/btnRemoveMark.svg` | 时间线工具栏 | 移除标记 |
| `ToolBar/btnCleanMarkN.svg` | 时间线工具栏 | 清除标记 |
| `ToolBar/btnAddAiMarks.svg` | 时间线工具栏 | 添加 AI 标记 |
| `ToolBar/quickCutLeft.svg` | 时间线工具栏 | 快速裁左侧 |
| `ToolBar/quickCutRight.svg` | 时间线工具栏 | 快速裁右侧 |
| `ToolBar/freeze.svg` | 时间线工具栏 | 冻结帧 |
| `ToolBar/reverse.svg` | 时间线工具栏 | 倒放 |
| `ToolBar/mirror.svg` | 时间线工具栏 | 镜像 |
| `ToolBar/rotate.svg` | 时间线工具栏 | 旋转 |
| `ToolBar/smart_split.svg` | 时间线工具栏 | 智能分割 |
| `ToolBar/smart_rough_cut.svg` | 时间线工具栏 | 智能粗剪 |
| `ToolBar/script_rough_cut_icon.svg` | 时间线工具栏 | 文稿粗剪入口 |
| `ToolBar/intelligent_extend.svg` | 时间线工具栏 | 智能扩展 |
| `ToolBar/agent_entry.svg` | 时间线工具栏 | AI Agent 入口 |

## 时间线画布

| 文件示例 | 编辑器位置 | 作用 |
|---|---|---|
| `Canvas/text.svg` | 时间线片段/轨道内容 | 文字片段图标 |
| `Canvas/text_template.svg` | 时间线片段/轨道内容 | 文本模板图标 |
| `Canvas/subtitleIcon.svg` | 时间线片段/轨道内容 | 字幕片段图标 |
| `Canvas/lyricsIcon.svg` | 时间线片段/轨道内容 | 歌词片段图标 |
| `Canvas/ttsIcon.svg` | 时间线片段/轨道内容 | 文本朗读/TTS 图标 |
| `Canvas/speedIcon.svg` | 时间线片段/轨道内容 | 变速图标 |
| `Canvas/transition.svg` | 时间线片段之间 | 转场图标 |
| `Canvas/composition.svg` | 时间线片段/轨道内容 | 组合片段图标 |
| `Canvas/template_play.svg` | 模板时间线片段 | 模板播放图标 |
| `Canvas/template_lock.svg` | 模板时间线片段 | 模板锁定图标 |
| `Canvas/timeline_loading.svg` | 时间线画布 | 加载状态 |
| `Canvas/loading.svg` | 时间线画布 | 通用加载图标 |
| `Canvas/retry.svg` | 时间线画布 | 重试按钮 |
| `Canvas/stickerDefault.svg` | 时间线贴纸片段 | 默认贴纸状态 |
| `Canvas/stickerError.svg` | 时间线贴纸片段 | 贴纸异常状态 |
| `Canvas/keyframe_line_normal.svg` | 关键帧/曲线编辑 | 线性关键帧普通状态 |
| `Canvas/keyframe_line_select.svg` | 关键帧/曲线编辑 | 线性关键帧选中状态 |
| `Canvas/keyframe_line_disable.svg` | 关键帧/曲线编辑 | 线性关键帧禁用状态 |
| `Canvas/keyframe_curved_select.svg` | 关键帧/曲线编辑 | 曲线关键帧选中状态 |
| `Canvas/keyframe_curved_disable.svg` | 关键帧/曲线编辑 | 曲线关键帧禁用状态 |
| `Canvas/keyframe_free_curved_normal.svg` | 关键帧/曲线编辑 | 自由曲线关键帧普通状态 |
| `Canvas/keyframe_free_curved_disable.svg` | 关键帧/曲线编辑 | 自由曲线关键帧禁用状态 |
| `Canvas/keyframe_group_select.svg` | 关键帧/曲线编辑 | 关键帧组选中状态 |

## 片段状态

| 文件 | 编辑器位置 | 作用 |
|---|---|---|
| `segment/locked.svg` | 时间线片段 | 片段锁定状态 |
| `segment/unlocked.svg` | 时间线片段 | 片段未锁定/可替换状态 |
| `segment/freeze.svg` | 时间线片段 | 冻结帧状态 |
| `segment/folder.svg` | 时间线片段 | 文件夹/组合类资源提示 |
| `segment/videochart.svg` | 时间线片段 | 视频图表/分析类状态提示 |

## 光标和拖拽状态

| 文件示例 | 编辑器位置 | 作用 |
|---|---|---|
| `Cursor/cut.svg` | 时间线光标 | 切割工具光标 |
| `Cursor/cutLeft.svg` | 时间线光标 | 左侧裁剪光标 |
| `Cursor/cutRight.svg` | 时间线光标 | 右侧裁剪光标 |
| `Cursor/multi_cutLeft.svg` | 时间线光标 | 多选左侧裁剪光标 |
| `Cursor/multi_cutRight.svg` | 时间线光标 | 多选右侧裁剪光标 |
| `Cursor/forwardSelectPointer.svg` | 时间线光标 | 向前选择指针 |
| `Cursor/backwardSelectPointer.svg` | 时间线光标 | 向后选择指针 |
| `Cursor/movable_cursor.svg` | 时间线光标 | 可移动状态 |
| `Cursor/movable_vertical.svg` | 时间线光标 | 垂直移动状态 |
| `Cursor/video_extend_left_cursor.svg` | 时间线光标 | 视频左侧延展 |
| `Cursor/video_extend_right_cursor.svg` | 时间线光标 | 视频右侧延展 |
| `Cursor/trackHeaderOpen.svg` | 轨道头 | 展开轨道头 |
| `Cursor/trackHeaderClose.svg` | 轨道头 | 收起轨道头 |
| `Drag/audio.svg` | 拖入时间线 | 音频素材拖拽提示 |
| `Drag/video.svg` | 拖入时间线 | 视频素材拖拽提示 |
| `Drag/photo.svg` | 拖入时间线 | 图片素材拖拽提示 |
| `Drag/folder.svg` | 拖入时间线 | 文件夹拖拽提示 |

## 素材替换和模板

| 文件 | 编辑器位置 | 作用 |
|---|---|---|
| `MaterialReplacement/play.svg` | 素材替换预览 | 播放 |
| `MaterialReplacement/pause.svg` | 素材替换预览 | 暂停 |
| `MaterialReplacement/volumn.svg` | 素材替换预览 | 音量 |
| `MaterialReplacement/multi_video_btn.svg` | 素材替换预览 | 多视频切换/多素材入口 |
| `MaterialReplacement/match_beats_highlight.svg` | 素材替换/卡点 | 卡点高亮提示 |
| `Template/entrance.svg` | 模板时间线 | 模板入口 |
| `Template/default_add.svg` | 模板时间线 | 默认添加 |
| `Template/hover_add.svg` | 模板时间线 | 悬停添加 |
| `Template/hover_add_new.svg` | 模板时间线 | 新版悬停添加 |
| `Template/put_away.svg` | 模板时间线 | 收起 |
| `Template/replacedIcon.svg` | 模板时间线 | 已替换提示 |
| `Template/warning.svg` | 模板时间线 | 警告提示 |



## 完整 SVG 索引

下表逐个记录当前 `~/Downloads/icons` 目录下的全部 SVG。数量应与本地扫描结果一致：216 个。

| # | 文件 | 编辑器位置 | 作用 |
|---:|---|---|---|
| 1 | `Canvas/GIFIcon.svg` | 时间线画布/片段区域 | 时间线画布内片段、状态或控制图标 |
| 2 | `Canvas/L-L.svg` | 时间线画布/片段区域 | 左侧连接/对齐状态图标 |
| 3 | `Canvas/L-M.svg` | 时间线画布/片段区域 | 左侧连接/对齐状态图标 |
| 4 | `Canvas/L-S.svg` | 时间线画布/片段区域 | 左侧连接/对齐状态图标 |
| 5 | `Canvas/PolygonR.svg` | 时间线画布/片段区域 | 多边形/图形右侧控制图标 |
| 6 | `Canvas/R-L.svg` | 时间线画布/片段区域 | 右侧连接/对齐状态图标 |
| 7 | `Canvas/R-M.svg` | 时间线画布/片段区域 | 右侧连接/对齐状态图标 |
| 8 | `Canvas/R-S.svg` | 时间线画布/片段区域 | 右侧连接/对齐状态图标 |
| 9 | `Canvas/TrackHeader/adjust.svg` | 时间线左侧轨道头 | 调节轨道类型图标 |
| 10 | `Canvas/TrackHeader/ai_clipper.svg` | 时间线左侧轨道头 | AI 剪辑入口图标 |
| 11 | `Canvas/TrackHeader/ai_clipper_grey.svg` | 时间线左侧轨道头 | AI 剪辑弱化/不可用状态图标 |
| 12 | `Canvas/TrackHeader/audio.svg` | 时间线左侧轨道头 | 音频轨道类型图标 |
| 13 | `Canvas/TrackHeader/btncoverDel.svg` | 时间线左侧轨道头 | 轨道封面删除按钮 |
| 14 | `Canvas/TrackHeader/btncoverEdit.svg` | 时间线左侧轨道头 | 轨道封面编辑按钮 |
| 15 | `Canvas/TrackHeader/effect.svg` | 时间线左侧轨道头 | 特效轨道类型图标 |
| 16 | `Canvas/TrackHeader/filter.svg` | 时间线左侧轨道头 | 滤镜轨道类型图标 |
| 17 | `Canvas/TrackHeader/hideOff.svg` | 时间线左侧轨道头 | 轨道显示/隐藏按钮另一状态图标 |
| 18 | `Canvas/TrackHeader/hideOn.svg` | 时间线左侧轨道头 | 轨道显示/隐藏按钮状态图标 |
| 19 | `Canvas/TrackHeader/lockOff.svg` | 时间线左侧轨道头 | 轨道未锁定状态图标 |
| 20 | `Canvas/TrackHeader/lockOn.svg` | 时间线左侧轨道头 | 轨道锁定状态图标 |
| 21 | `Canvas/TrackHeader/muteOff.svg` | 时间线左侧轨道头 | 音频轨道非静音状态图标 |
| 22 | `Canvas/TrackHeader/muteOn.svg` | 时间线左侧轨道头 | 音频轨道静音状态图标 |
| 23 | `Canvas/TrackHeader/solo_off.svg` | 时间线左侧轨道头 | 音频轨道非独奏状态图标 |
| 24 | `Canvas/TrackHeader/solo_on.svg` | 时间线左侧轨道头 | 音频轨道独奏状态图标 |
| 25 | `Canvas/TrackHeader/sticker.svg` | 时间线左侧轨道头 | 贴纸轨道类型图标 |
| 26 | `Canvas/TrackHeader/text.svg` | 时间线左侧轨道头 | 文字轨道类型图标 |
| 27 | `Canvas/TrackHeader/text_flower.svg` | 时间线左侧轨道头 | 花字/特殊文本轨道类型图标 |
| 28 | `Canvas/TrackHeader/video.svg` | 时间线左侧轨道头 | 视频轨道类型图标 |
| 29 | `Canvas/add.svg` | 时间线画布/片段区域 | 添加按钮图标 |
| 30 | `Canvas/adjoinRectangleLeft.svg` | 时间线画布/片段区域 | 相邻矩形左侧控制图标 |
| 31 | `Canvas/adjoinRectangleRight.svg` | 时间线画布/片段区域 | 相邻矩形右侧控制图标 |
| 32 | `Canvas/adjustIcon.svg` | 时间线画布/片段区域 | 时间线画布内片段、状态或控制图标 |
| 33 | `Canvas/ai_generate_tag.svg` | 时间线画布/片段区域 | AI 生成标签图标 |
| 34 | `Canvas/animationArrowL.svg` | 时间线画布/片段区域 | 动画左向箭头 |
| 35 | `Canvas/animationArrowR.svg` | 时间线画布/片段区域 | 动画右向箭头 |
| 36 | `Canvas/arrowMoveR.svg` | 时间线画布/片段区域 | 右移箭头 |
| 37 | `Canvas/bussiness_warning.svg` | 时间线画布/片段区域 | 商业/版权提示图标 |
| 38 | `Canvas/combination.svg` | 时间线画布/片段区域 | 时间线画布内片段、状态或控制图标 |
| 39 | `Canvas/composition.svg` | 时间线画布/片段区域 | 组合片段图标 |
| 40 | `Canvas/defaultCover.svg` | 时间线画布/片段区域 | 默认封面图标 |
| 41 | `Canvas/draging.svg` | 时间线画布/片段区域 | 拖拽中状态图标 |
| 42 | `Canvas/effectError.svg` | 时间线画布/片段区域 | 异常/错误状态图标 |
| 43 | `Canvas/freeHorizon.svg` | 时间线画布/片段区域 | 水平自由移动光标 |
| 44 | `Canvas/fundamentalShapeIcon.svg` | 时间线画布/片段区域 | 基础图形片段图标 |
| 45 | `Canvas/icTLGroup.svg` | 时间线画布/片段区域 | 时间线分组图标 |
| 46 | `Canvas/icoclipColorEffect.svg` | 时间线画布/片段区域 | 片段颜色特效图标 |
| 47 | `Canvas/icoclipColorFilter.svg` | 时间线画布/片段区域 | 片段颜色滤镜图标 |
| 48 | `Canvas/keyframe_curved_disable.svg` | 时间线画布/片段区域 | 曲线关键帧禁用状态 |
| 49 | `Canvas/keyframe_curved_normal.svg` | 时间线画布/片段区域 | 关键帧相关图标 |
| 50 | `Canvas/keyframe_curved_select.svg` | 时间线画布/片段区域 | 曲线关键帧选中状态 |
| 51 | `Canvas/keyframe_down.svg` | 时间线画布/片段区域 | 关键帧下拉/向下状态图标 |
| 52 | `Canvas/keyframe_free_curved_disable.svg` | 时间线画布/片段区域 | 自由曲线关键帧禁用状态 |
| 53 | `Canvas/keyframe_free_curved_normal.svg` | 时间线画布/片段区域 | 自由曲线关键帧普通状态 |
| 54 | `Canvas/keyframe_free_curved_select.svg` | 时间线画布/片段区域 | 关键帧相关图标 |
| 55 | `Canvas/keyframe_group_select.svg` | 时间线画布/片段区域 | 关键帧组选中状态 |
| 56 | `Canvas/keyframe_line_disable.svg` | 时间线画布/片段区域 | 线性关键帧禁用状态 |
| 57 | `Canvas/keyframe_line_normal.svg` | 时间线画布/片段区域 | 线性关键帧普通状态 |
| 58 | `Canvas/keyframe_line_select.svg` | 时间线画布/片段区域 | 线性关键帧选中状态 |
| 59 | `Canvas/keyframe_up.svg` | 时间线画布/片段区域 | 关键帧相关图标 |
| 60 | `Canvas/leftArray.svg` | 时间线画布/片段区域 | 左箭头/左侧指示图标 |
| 61 | `Canvas/leftControl.svg` | 时间线画布/片段区域 | 左侧控制柄图标 |
| 62 | `Canvas/loading.svg` | 时间线画布/片段区域 | 加载状态图标 |
| 63 | `Canvas/lyricsIcon.svg` | 时间线画布/片段区域 | 歌词片段图标 |
| 64 | `Canvas/optionsArrowR.svg` | 时间线画布/片段区域 | 选项右箭头 |
| 65 | `Canvas/replaceSegment.svg` | 时间线画布/片段区域 | 替换入口图标 |
| 66 | `Canvas/retry.svg` | 时间线画布/片段区域 | 重试按钮图标 |
| 67 | `Canvas/rightArrow.svg` | 时间线画布/片段区域 | 右箭头图标 |
| 68 | `Canvas/rightControl.svg` | 时间线画布/片段区域 | 右侧控制柄图标 |
| 69 | `Canvas/select_range_cancel_rect.svg` | 时间线画布/片段区域 | 时间线画布内片段、状态或控制图标 |
| 70 | `Canvas/shapeIcon.svg` | 时间线画布/片段区域 | 时间线画布内片段、状态或控制图标 |
| 71 | `Canvas/speedIcon.svg` | 时间线画布/片段区域 | 变速片段图标 |
| 72 | `Canvas/stickerDefault.svg` | 时间线画布/片段区域 | 默认状态图标 |
| 73 | `Canvas/stickerError.svg` | 时间线画布/片段区域 | 异常/错误状态图标 |
| 74 | `Canvas/subtitleIcon.svg` | 时间线画布/片段区域 | 字幕片段图标 |
| 75 | `Canvas/template_combination.svg` | 时间线画布/片段区域 | 模板组合图标 |
| 76 | `Canvas/template_lock.svg` | 时间线画布/片段区域 | 模板锁定图标 |
| 77 | `Canvas/template_play.svg` | 时间线画布/片段区域 | 模板播放图标 |
| 78 | `Canvas/text.svg` | 时间线画布/片段区域 | 文字片段图标 |
| 79 | `Canvas/textTemplateIcon.svg` | 时间线画布/片段区域 | 时间线画布内片段、状态或控制图标 |
| 80 | `Canvas/textTimeline.svg` | 时间线画布/片段区域 | 时间线画布内片段、状态或控制图标 |
| 81 | `Canvas/text_template.svg` | 时间线画布/片段区域 | 文本模板片段图标 |
| 82 | `Canvas/timeline_loading.svg` | 时间线画布/片段区域 | 时间线加载状态图标 |
| 83 | `Canvas/timeline_tab_more.svg` | 时间线画布/片段区域 | 时间线画布内片段、状态或控制图标 |
| 84 | `Canvas/timeline_tab_pin.svg` | 时间线画布/片段区域 | 时间线标签固定图标 |
| 85 | `Canvas/timeline_tab_pin_2.svg` | 时间线画布/片段区域 | 时间线标签固定状态图标 |
| 86 | `Canvas/tracking.svg` | 时间线画布/片段区域 | 跟踪相关图标 |
| 87 | `Canvas/transition.svg` | 时间线画布/片段区域 | 转场图标 |
| 88 | `Canvas/ttsIcon.svg` | 时间线画布/片段区域 | 文本朗读/TTS 图标 |
| 89 | `Canvas/video_extend_bar_close.svg` | 时间线画布/片段区域 | 视频延展关闭按钮 |
| 90 | `Canvas/video_extend_bar_input.svg` | 时间线画布/片段区域 | 视频延展输入栏图标 |
| 91 | `Canvas/video_extend_bar_start.svg` | 时间线画布/片段区域 | 视频延展起点图标 |
| 92 | `Canvas/vipIcon.svg` | 时间线画布/片段区域 | 会员/VIP 标识 |
| 93 | `Cursor/adjoincut.svg` | 时间线光标与拖拽交互 | 切割工具光标 |
| 94 | `Cursor/backwardSelectPointer.svg` | 时间线光标与拖拽交互 | 指针/选择工具图标 |
| 95 | `Cursor/cut.svg` | 时间线光标与拖拽交互 | 切割工具光标 |
| 96 | `Cursor/cutLeft.svg` | 时间线光标与拖拽交互 | 左侧裁剪光标 |
| 97 | `Cursor/cutRight.svg` | 时间线光标与拖拽交互 | 右侧裁剪光标 |
| 98 | `Cursor/edit_text.svg` | 时间线光标与拖拽交互 | 文字片段图标 |
| 99 | `Cursor/forwardSelectPointer.svg` | 时间线光标与拖拽交互 | 指针/选择工具图标 |
| 100 | `Cursor/freeHorizon.svg` | 时间线光标与拖拽交互 | 水平自由移动光标 |
| 101 | `Cursor/header_adsorb.svg` | 时间线光标与拖拽交互 | 轨道头吸附状态光标 |
| 102 | `Cursor/header_adsorb_pressed.svg` | 时间线光标与拖拽交互 | 轨道头吸附按下状态光标 |
| 103 | `Cursor/header_disable.svg` | 时间线光标与拖拽交互 | 轨道头禁用状态光标 |
| 104 | `Cursor/header_normal.svg` | 时间线光标与拖拽交互 | 轨道头普通状态光标 |
| 105 | `Cursor/header_normal_pressed.svg` | 时间线光标与拖拽交互 | 轨道头普通按下状态光标 |
| 106 | `Cursor/movable_cursor.svg` | 时间线光标与拖拽交互 | 可移动状态光标 |
| 107 | `Cursor/movable_vertical.svg` | 时间线光标与拖拽交互 | 垂直移动光标 |
| 108 | `Cursor/multi_cutLeft.svg` | 时间线光标与拖拽交互 | 多选左侧裁剪光标 |
| 109 | `Cursor/multi_cutRight.svg` | 时间线光标与拖拽交互 | 多选右侧裁剪光标 |
| 110 | `Cursor/trackHeaderClose.svg` | 时间线光标与拖拽交互 | 轨道头收起图标 |
| 111 | `Cursor/trackHeaderOpen.svg` | 时间线光标与拖拽交互 | 轨道头展开图标 |
| 112 | `Cursor/video_extend_left_cursor.svg` | 时间线光标与拖拽交互 | 视频左侧延展光标 |
| 113 | `Cursor/video_extend_right_cursor.svg` | 时间线光标与拖拽交互 | 视频右侧延展光标 |
| 114 | `Drag/audio.svg` | 拖入时间线提示 | 拖拽音频素材提示 |
| 115 | `Drag/folder.svg` | 拖入时间线提示 | 拖拽文件夹素材提示 |
| 116 | `Drag/photo.svg` | 拖入时间线提示 | 拖拽图片素材提示 |
| 117 | `Drag/video.svg` | 拖入时间线提示 | 拖拽视频素材提示 |
| 118 | `MaterialReplacement/match_beats_highlight.svg` | 素材替换预览 | 卡点匹配高亮图标 |
| 119 | `MaterialReplacement/multi_video_btn.svg` | 素材替换预览 | 多视频切换按钮 |
| 120 | `MaterialReplacement/pause.svg` | 素材替换预览 | 暂停按钮 |
| 121 | `MaterialReplacement/play.svg` | 素材替换预览 | 播放按钮 |
| 122 | `MaterialReplacement/volumn.svg` | 素材替换预览 | 音量按钮 |
| 123 | `Template/default_add.svg` | 模板时间线 | 添加按钮图标 |
| 124 | `Template/entrance.svg` | 模板时间线 | 模板入口图标 |
| 125 | `Template/hover_add.svg` | 模板时间线 | 添加按钮图标 |
| 126 | `Template/hover_add_new.svg` | 模板时间线 | 新版悬停添加按钮 |
| 127 | `Template/put_away.svg` | 模板时间线 | 收起按钮 |
| 128 | `Template/replacedIcon.svg` | 模板时间线 | 替换入口图标 |
| 129 | `Template/warning.svg` | 模板时间线 | 警告提示图标 |
| 130 | `ToolBar/Slider.svg` | 时间线上方工具栏 | 滑杆/缩放条图标 |
| 131 | `ToolBar/addTimeline.svg` | 时间线上方工具栏 | 添加时间线/轨道入口图标 |
| 132 | `ToolBar/add_keyframe_guide.svg` | 时间线上方工具栏 | 添加关键帧引导图标 |
| 133 | `ToolBar/agent_entry.svg` | 时间线上方工具栏 | AI Agent 入口图标 |
| 134 | `ToolBar/arrow_down.svg` | 时间线上方工具栏 | 向下箭头 |
| 135 | `ToolBar/arrow_down_big.svg` | 时间线上方工具栏 | 大号向下箭头 |
| 136 | `ToolBar/arrow_up.svg` | 时间线上方工具栏 | 向上箭头 |
| 137 | `ToolBar/arrow_up_big.svg` | 时间线上方工具栏 | 大号向上箭头 |
| 138 | `ToolBar/btnAddAiMarks.svg` | 时间线上方工具栏 | AI 标记按钮 |
| 139 | `ToolBar/btnAddMark.svg` | 时间线上方工具栏 | 添加标记按钮 |
| 140 | `ToolBar/btnAutoMarkN.svg` | 时间线上方工具栏 | 标记相关按钮 |
| 141 | `ToolBar/btnBackwardSelectIcon.svg` | 时间线上方工具栏 | 向后选择工具图标 |
| 142 | `ToolBar/btnCleanMarkN.svg` | 时间线上方工具栏 | 清除标记按钮 |
| 143 | `ToolBar/btnCloseAdsorb.svg` | 时间线上方工具栏 | 时间线吸附开关图标 |
| 144 | `ToolBar/btnCloseMainTrackAdsorb.svg` | 时间线上方工具栏 | 时间线吸附开关图标 |
| 145 | `ToolBar/btnClosePreviewAxis.svg` | 时间线上方工具栏 | 预览轴开关图标 |
| 146 | `ToolBar/btnForwardSelectIcon.svg` | 时间线上方工具栏 | 向前选择工具图标 |
| 147 | `ToolBar/btnMarkDelN.svg` | 时间线上方工具栏 | 删除标记按钮 |
| 148 | `ToolBar/btnMarkN.svg` | 时间线上方工具栏 | 标记相关按钮 |
| 149 | `ToolBar/btnOpenAdsorb.svg` | 时间线上方工具栏 | 时间线吸附开关图标 |
| 150 | `ToolBar/btnOpenMainTrackAdsorb.svg` | 时间线上方工具栏 | 时间线吸附开关图标 |
| 151 | `ToolBar/btnOpenPreviewAxis.svg` | 时间线上方工具栏 | 预览轴开关图标 |
| 152 | `ToolBar/btnRecord.svg` | 时间线上方工具栏 | 录音入口按钮 |
| 153 | `ToolBar/btnRecordStart.svg` | 时间线上方工具栏 | 开始录音状态按钮 |
| 154 | `ToolBar/btnRecordStarting.svg` | 时间线上方工具栏 | 开始录音状态按钮 |
| 155 | `ToolBar/btnRecordStop.svg` | 时间线上方工具栏 | 停止录音状态按钮 |
| 156 | `ToolBar/btnRecordStopping.svg` | 时间线上方工具栏 | 停止录音状态按钮 |
| 157 | `ToolBar/btnRemoveMark.svg` | 时间线上方工具栏 | 移除标记按钮 |
| 158 | `ToolBar/btnStyleCut.svg` | 时间线上方工具栏 | 时间线工具栏按钮或状态图标 |
| 159 | `ToolBar/btnStyleCutIcon.svg` | 时间线上方工具栏 | 时间线工具栏按钮或状态图标 |
| 160 | `ToolBar/btnStylePointer.svg` | 时间线上方工具栏 | 指针/选择工具图标 |
| 161 | `ToolBar/btnZoomFitAdjust.svg` | 时间线上方工具栏 | 缩放到合适视图按钮 |
| 162 | `ToolBar/btnZoomIn.svg` | 时间线上方工具栏 | 放大时间线按钮 |
| 163 | `ToolBar/btnZoomOutN.svg` | 时间线上方工具栏 | 缩小时间线按钮 |
| 164 | `ToolBar/buttonLinkage_off.svg` | 时间线上方工具栏 | 联动开关图标 |
| 165 | `ToolBar/buttonLinkage_on.svg` | 时间线上方工具栏 | 联动开关图标 |
| 166 | `ToolBar/caption_text_icon.svg` | 时间线上方工具栏 | 字幕/文本入口图标 |
| 167 | `ToolBar/check.svg` | 时间线上方工具栏 | 勾选状态图标 |
| 168 | `ToolBar/combobox_down.svg` | 时间线上方工具栏 | 下拉框向下箭头 |
| 169 | `ToolBar/combobox_up.svg` | 时间线上方工具栏 | 下拉框向上箭头 |
| 170 | `ToolBar/cutoff.svg` | 时间线上方工具栏 | 切断/截断工具按钮 |
| 171 | `ToolBar/cutting.svg` | 时间线上方工具栏 | 切割/分割工具按钮 |
| 172 | `ToolBar/del.svg` | 时间线上方工具栏 | 删除按钮 |
| 173 | `ToolBar/freeze.svg` | 时间线上方工具栏 | 冻结帧按钮 |
| 174 | `ToolBar/iconReplace.svg` | 时间线上方工具栏 | 替换入口图标 |
| 175 | `ToolBar/intelligent_extend.svg` | 时间线上方工具栏 | 智能扩展入口图标 |
| 176 | `ToolBar/leftArray.svg` | 时间线上方工具栏 | 左箭头/左侧指示图标 |
| 177 | `ToolBar/mic_status_help.svg` | 时间线上方工具栏 | 麦克风帮助提示图标 |
| 178 | `ToolBar/mic_status_warning.svg` | 时间线上方工具栏 | 麦克风异常提示图标 |
| 179 | `ToolBar/mirror.svg` | 时间线上方工具栏 | 镜像按钮 |
| 180 | `ToolBar/more_timeline_2.svg` | 时间线上方工具栏 | 时间线更多菜单图标 |
| 181 | `ToolBar/more_timeline_3.svg` | 时间线上方工具栏 | 时间线更多菜单图标 |
| 182 | `ToolBar/more_timeline_4.svg` | 时间线上方工具栏 | 时间线更多菜单图标 |
| 183 | `ToolBar/more_timeline_5.svg` | 时间线上方工具栏 | 时间线更多菜单图标 |
| 184 | `ToolBar/more_timeline_6.svg` | 时间线上方工具栏 | 时间线更多菜单图标 |
| 185 | `ToolBar/previewAxisDisabled.svg` | 时间线上方工具栏 | 时间线工具栏按钮或状态图标 |
| 186 | `ToolBar/previewAxisEnabled.svg` | 时间线上方工具栏 | 时间线工具栏按钮或状态图标 |
| 187 | `ToolBar/prune.svg` | 时间线上方工具栏 | 修剪/精剪按钮 |
| 188 | `ToolBar/quickCutLeft.svg` | 时间线上方工具栏 | 快速裁剪左侧按钮 |
| 189 | `ToolBar/quickCutRight.svg` | 时间线上方工具栏 | 快速裁剪右侧按钮 |
| 190 | `ToolBar/record_countdown_1.svg` | 时间线上方工具栏 | 录音倒计时图标 |
| 191 | `ToolBar/record_countdown_2.svg` | 时间线上方工具栏 | 录音倒计时图标 |
| 192 | `ToolBar/record_countdown_3.svg` | 时间线上方工具栏 | 录音倒计时图标 |
| 193 | `ToolBar/redo.svg` | 时间线上方工具栏 | 重做按钮 |
| 194 | `ToolBar/replace.svg` | 时间线上方工具栏 | 替换入口图标 |
| 195 | `ToolBar/reverse.svg` | 时间线上方工具栏 | 倒放按钮 |
| 196 | `ToolBar/rotate.svg` | 时间线上方工具栏 | 旋转按钮 |
| 197 | `ToolBar/script_rough_cut_icon.svg` | 时间线上方工具栏 | 文稿粗剪入口图标 |
| 198 | `ToolBar/smart_rough_cut.svg` | 时间线上方工具栏 | 智能粗剪入口图标 |
| 199 | `ToolBar/smart_split.svg` | 时间线上方工具栏 | 智能分割入口图标 |
| 200 | `ToolBar/timeline_drop_1.svg` | 时间线上方工具栏 | 时间线下拉/拖放状态图标 |
| 201 | `ToolBar/timeline_drop_2.svg` | 时间线上方工具栏 | 时间线下拉/拖放状态图标 |
| 202 | `ToolBar/tracks_adjust.svg` | 时间线上方工具栏 | 时间线工具栏按钮或状态图标 |
| 203 | `ToolBar/trim_clip.svg` | 时间线上方工具栏 | 片段裁剪按钮 |
| 204 | `ToolBar/undo.svg` | 时间线上方工具栏 | 撤销按钮 |
| 205 | `ToolBar/volume.svg` | 时间线上方工具栏 | 音量按钮 |
| 206 | `ToolBar/wave_100_normal.svg` | 时间线上方工具栏 | 波形显示比例状态图标 |
| 207 | `ToolBar/wave_100_selected.svg` | 时间线上方工具栏 | 波形显示比例状态图标 |
| 208 | `ToolBar/wave_30_normal.svg` | 时间线上方工具栏 | 波形显示比例状态图标 |
| 209 | `ToolBar/wave_30_selected.svg` | 时间线上方工具栏 | 波形显示比例状态图标 |
| 210 | `ToolBar/wave_60_normal.svg` | 时间线上方工具栏 | 波形显示比例状态图标 |
| 211 | `ToolBar/wave_60_selected.svg` | 时间线上方工具栏 | 波形显示比例状态图标 |
| 212 | `segment/folder.svg` | 时间线片段状态 | 文件夹/组合类片段状态图标 |
| 213 | `segment/freeze.svg` | 时间线片段状态 | 冻结帧片段状态图标 |
| 214 | `segment/locked.svg` | 时间线片段状态 | 片段锁定状态图标 |
| 215 | `segment/unlocked.svg` | 时间线片段状态 | 片段未锁定/可替换状态图标 |
| 216 | `segment/videochart.svg` | 时间线片段状态 | 视频图表/分析类片段状态图标 |
