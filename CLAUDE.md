# ShenhePlayer — 项目上下文

给 AI 助手（以及接手的人）看的项目笔记：架构、关键决策、以及**已经踩过的坑**。
改代码前先扫一遍「踩过的坑」那一节，里面每一条都是实际调试出来的，重复踩很浪费时间。

用户可见的版本变化记在 [CHANGELOG.md](CHANGELOG.md)。

---

## 这是什么

Windows 平台的 Emby 客户端。目标：界面好看、启动快、占用低、播放不打折。
安装包 2.0 MB，主程序 5.6 MB，前端 JS 96 KB（gzip 36 KB）。

## 技术栈与选型理由

| 层 | 选型 | 为什么 |
| --- | --- | --- |
| 外壳 | Tauri v2（Rust + WebView2） | 不打包 Chromium，复用系统运行时；Electron 体积和内存都差一个量级 |
| 界面 | Vue 3 + TS + Vite，手写 CSS | 不引重型 UI 库；设计令牌在 `src/styles/tokens.css` |
| 网络 | Rust `reqwest`（SChannel TLS） | 绕开 CORS 与混合内容限制，token 不落在前端，JSON 解析更快 |
| 解码渲染 | mpv 子进程 + `--wid` 嵌入 + JSON IPC | WebView2 只能播 H.264/VP9 的 MP4，媒体库里大量 MKV/HEVC/DTS 放不了 |

## 目录结构

```
src/                        前端
  api/                      Tauri 命令封装（index.ts）+ 图片 URL 拼装（images.ts）
  components/               通用组件
    player/                 播放控制层，跑在独立的 overlay 窗口里
  stores/                   Pinia：session / settings / player
  views/                    页面
  styles/tokens.css         设计令牌（含亮色主题覆盖）
  styles/base.css           排版阶梯、按钮、全局工具类
src-tauri/src/
  emby/                     Emby REST 客户端（client.rs）、数据模型（models.rs）、宽容反序列化（de.rs）
  player/mpv.rs             mpv 进程管理与 JSON IPC
  player/external.rs        外置播放器探测与启动
  win/host.rs               Win32 视频宿主子窗口
  commands/                 暴露给前端的命令，按 auth / library / playback / system 分文件
  update.rs                 检查 GitHub Releases
scripts/                    验证用的 PowerShell 脚本，见下文
```

## 窗口结构（重要）

播放器是**两个独立的顶层窗口**，都不依附浏览窗口：

```
main 窗口（不透明）                浏览界面
player 窗口（不透明、无边框）      全屏/最大化/关闭作用在它身上
 ├─ WebView2                      闲置，被视频完全盖住
 └─ ShenheVideoHost（置顶子窗口）   mpv 在这里渲染
overlay 窗口（透明，owner=player） 播放控制条，DWM 合成到视频之上
```

控制条为什么要单独一个窗口，见「踩过的坑」第 2 条。

`player` 窗口尺寸一变，`Resized` 事件里同时做两件事：`host.fit_to_parent()` 和 `sync_overlay()`。
漏掉任何一个都会在全屏时露出底层。

---

## 踩过的坑

### 1. mpv 0.38+ 的 `loadfile` 多了一个 index 参数

`loadfile <url> [<flags> [<index> [<options>]]]`。把 options 直接放第四位会被当成 index，
命令**静默失效**（`file_loaded` 一直是 false，画面不出，没有任何报错）。

现在起播位置和标题走属性设置（`set_property start` / `force-media-title`），跨版本都一致。
见 `src-tauri/src/player/mpv.rs` 的 `load_file`。

### 2. Tauri 透明窗口里子窗口不参与合成

Tauri 的 `transparent(true)` 会带上 `WS_EX_NOREDIRECTIONBITMAP`，窗口没有重定向表面，
**子窗口（HWND）完全不参与 DWM 合成**。所以「视频子窗口垫在透明 WebView2 底下」这条路走不通——
实测看到的是穿透过去的桌面/后面的窗口，视频根本不显示。

结论：视频子窗口必须置于 WebView2 **之上**（盖住它），控制条另开一个 owner 是播放窗口的透明窗口。
owned 窗口永远显示在 owner 之上、跟着 owner 最小化、不占任务栏。

### 3. Emby 同一字段的类型在不同服务器上不一样

典型是 `ProviderIds`：`{"Tmdb": 757}` 是数字，`{"Imdb": "tt0120737"}` 是字符串。
`Studios[].Id`、`People[].Id`、`ImageTags` 也有同样情况。严格按 String 解析会让**整部影片加载失败**
（报 `invalid type: integer '757', expected a string`）。

所有 id / tag / 文本字段统一走 `src-tauri/src/emby/de.rs` 里的宽容反序列化器。
加新字段时照抄现有写法，别直接用裸 `String`。

### 4. Vue scoped CSS 挡不住全局工具类

组件里的局部类名如果和 `base.css` 的全局工具类同名，**全局那份照样生效**。
已经踩过两次：
- `MediaRow` 用了 `.row`，撞上 `.row { display:flex }`，整行标题竖排
- `PosterCard` 用了 `.card`，撞上 `.card { background/border }`，亮色主题下每张海报后面多个灰框

局部类名不要用 `row / card / stack / item / spacer / dim / num` 这类通用词。
`.row` 和 `.stack` 已经从全局工具类里删掉了。

### 5. scoped 样式不会穿透到子组件内部

父组件的 `.glyph` 样式管不到 `TrackMenu` 内部的按钮（scoped 只给子组件的**根元素**加作用域属性）。
`TrackMenu` 因此自带一份和 `PlayerRoot` 一致的按钮样式。改播放器按钮样式时两处都要动。

### 6. Tauri capability 缺权限会让窗口永远不出现

窗口默认 `visible: false`，等前端渲染完再 `show()`。但 `core:window:allow-show` 没配的话，
`show()` 直接抛异常 → 进程在跑、窗口永远看不见，而且没有任何提示。

`src-tauri/capabilities/default.json` 里 `windows` 数组要包含所有窗口标签（`main` / `player` / `overlay`），
用到的窗口操作都要显式列权限。另外 `lib.rs` 里有个 3 秒兜底：前端没按时就绪就强制显示窗口。

### 7. 路由守卫跑在会话恢复之前

启动时 `session.restore()` 是异步的，路由的首次导航早就跑完了。
所以「恢复成功后」要么主动导航，要么用 `watch(() => session.isAuthed, ...)` 重新取数据。
`HomeView` 用的是后者（`immediate: true`），否则会出现「侧边栏有媒体库、内容区空白」。

### 8. 点击打开的菜单不能靠 pointerleave 关闭

按钮和弹出菜单之间有间隙，鼠标穿过时会同时离开两者，菜单当场消失，手再快也过不去。
现在菜单开合由 `PlayerRoot` 统一管理（三个菜单互斥），只在**选中项 / 点外部 / Esc / 控制条隐藏**时关闭，
间隙用一层透明 `.bridge` 补上。菜单开着时控制条不自动隐藏。

### 9. 照片/视频上的文字不能用会跟随主题的颜色令牌

`.dim` / `.dim-3` 在亮色主题下会翻成深灰，叠在大图上直接看不清。
Hero、详情页顶部、播放控制层里的文字颜色都在组件内固定成浅色系。
遮罩同理：暗色下可以淡入 `var(--bg)`，亮色下淡入白色会让白字消失，需要单独给一套深色遮罩。

### 10. 截图验证的注意事项

- `PrintWindow` **抓不到 mpv 的 GPU 画面**，返回全黑。要验证视频是否真的出画，得用真实屏幕采样。
- 采样前必须确认前台窗口属于本进程，否则会拍到用户屏幕上的其它内容（`scripts/shot-live.ps1` 里有这个校验，
  不满足就直接放弃截图）。
- DPI：PowerShell 进程默认 DPI 不感知，`GetClientRect` 返回虚拟化后的逻辑像素，截出来是裁切的。
  脚本里都调了 `SetProcessDPIAware()`。
- **`.ps1` 文件保持纯 ASCII**：Windows PowerShell 5.1 会用系统 ANSI 代码页读无 BOM 的 UTF-8 文件，
  中文注释会把解析搞坏。

---

## 开发与验证

```bash
npm install
npm run app:dev          # 开发模式（热重载）
npm run app:build        # 打包 NSIS 安装程序
npm run build            # 只做前端类型检查 + 构建
cd src-tauri && cargo test --lib
```

### 冒烟自检（不需要 Emby 服务器）

```bash
SHENHE_SELFTEST=1 npm run app:dev
```

启动 3 秒后自动用 mpv 的合成测试源走一遍完整播放链路，日志里会打印：
外置播放器探测结果、是否载入成功、分辨率、帧率、解码方式、播放位置。
`自检：通过` 说明 mpv 启动、窗口嵌入、IPC、属性回流全部正常。

设置页的「测试」按钮做同样的事，是给用户排查用的。

### 验证脚本（scripts/）

| 脚本 | 用途 |
| --- | --- |
| `shot-any.ps1` | 按标题截某个窗口（PrintWindow，抓不到视频画面） |
| `shot-live.ps1` | 真实屏幕采样，带前台归属校验；`-Wake` 会先动一下鼠标唤出控制条 |
| `check-geometry.ps1` | 比对播放窗口客户区与视频宿主子窗口的矩形，验证有没有缝隙 |
| `poke-menu.ps1` | 真实点击播放器的菜单按钮并把光标移进菜单，验证 hover 行为 |
| `crop.ps1` | 裁剪/放大截图局部 |
| `gen-icons.mjs` | 重新生成应用图标（纯 Node 手写 PNG/ICO 编码） |

## 代码约定

- **注释用中文，只写「为什么」**，不复述代码在做什么。有反直觉的取舍就写清楚原因。
- Rust 命令统一返回 `Result<T, AppError>`，`AppError` 序列化成中文错误串直接给用户看。
- 前端类型（`src/types.ts`）和 Rust 的 serde 输出一一对应；Rust 侧反序列化用 PascalCase（Emby 的格式），
  序列化给前端用 camelCase。
- 新增 Tauri 命令要同时改三处：`commands/*.rs`、`lib.rs` 的 `invoke_handler`、`src/api/index.ts`。
- 改了用户可见的行为，往 `CHANGELOG.md` 的「未发布」段落加一条。
- 踩到新的坑，往本文件「踩过的坑」加一节。
