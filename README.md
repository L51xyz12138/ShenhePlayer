# ShenhePlayer

Windows 平台的 Emby 客户端。目标是：**界面好看、启动快、占用低、播放不打折**。

- **安装包 2.0 MB，主程序 5.6 MB**（实测）。不打包 Chromium，复用系统自带的 WebView2 运行时
- 前端 JS 96 KB（gzip 36 KB），路由级代码分割，海报懒加载
- 内置播放器基于 **mpv**：D3D11 硬件解码，HEVC / AV1 / MKV / DTS / TrueHD / ASS 字幕全部原生直连播放，服务器不转码
- 也可以把播放交给 **PotPlayer / mpv / VLC / MPC-HC** 等已装的播放器
- 低端机可切「性能优先」画质档 + 低性能模式，关掉毛玻璃与动效
- 亮色 / 暗色 / 跟随系统三种外观；播放控制层始终保持深色
- 可保存多台 Emby 服务器随时切换，未连接时主界面照常可用

## 技术选型

| 层 | 选型 | 原因 |
| --- | --- | --- |
| 外壳 | Tauri v2（Rust + WebView2） | 体积/内存远小于 Electron，系统已自带 WebView2 |
| 界面 | Vue 3 + TypeScript + Vite | 手写 CSS 设计系统，不引重型 UI 库，首屏 JS gzip 约 36 KB |
| 网络 | Rust 侧 `reqwest`（SChannel TLS） | 无 CORS 限制、无混合内容问题，JSON 解析比 JS 快，token 不落在前端 |
| 解码渲染 | mpv 子进程 + `--wid` 窗口嵌入 + JSON IPC | WebView2 只能播 H.264/VP9 的 MP4，媒体库里大量文件放不了 |

### 播放画面是怎么显示出来的

播放器是**独立的窗口**，和浏览界面互不干扰，可以一边播一边翻媒体库。

```
player 窗口（不透明、无边框）        全屏 / 最大化 / 关闭都作用在它身上
 ├─ WebView2                        闲置，被视频完全盖住
 └─ ShenheVideoHost（置顶子窗口）    mpv 在这里渲染

overlay 窗口（透明，归属于 player）  播放控制条，由 DWM 合成到视频之上
```

为什么控制条要单独开一个窗口：Tauri 的透明窗口带 `WS_EX_NOREDIRECTIONBITMAP`，
没有重定向表面，子窗口根本不参与合成 —— 把视频放在透明 WebView2 底下的话画面
不显示。所以让视频盖住 WebView2，控制条用一个归属于播放窗口的透明窗口叠上去。
owned 窗口永远在 owner 之上、跟着 owner 最小化，也不会在任务栏多出一项。

两个窗口都挂在播放器自己身上（而不是浏览窗口），播放窗口尺寸一变就同时对齐
视频宿主和控制层，所以全屏时不会露出底下的东西。

浏览窗口保持不透明，文字走 ClearType 渲染，清晰且没有额外的合成开销。

## 环境要求

- Windows 10 1809+ / Windows 11
- [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)（Win11 自带；Win10 多数已随 Edge 安装）
- 内置播放器需要 `mpv.exe`。程序会按此顺序查找：设置里指定的路径 → 程序目录下的 `mpv/mpv.exe` → PATH 与常见安装目录

## 开发

```bash
npm install
npm run app:dev      # 开发模式（热重载）
npm run app:build    # 打包出 NSIS 安装程序
```

其它脚本：

```bash
npm run build                  # 只做前端类型检查 + 构建
node scripts/gen-icons.mjs     # 重新生成应用图标
```

自检（不连服务器验证内置播放器）：

```bash
SHENHE_SELFTEST=1 npm run app:dev
```

设置页里的「测试」按钮做同样的事：让 mpv 播一段合成画面，用来确认播放器能启动、能硬解、画面能正确嵌入窗口。

## 目录结构

```
src/                      前端
  api/                    Tauri 命令封装 + 图片 URL 拼装
  components/             通用组件
    player/               播放控制层（跑在独立的透明窗口里）
  stores/                 Pinia：会话 / 设置 / 播放器
  views/                  页面
  styles/                 设计令牌与基础样式
src-tauri/src/            后端
  emby/                   Emby REST 客户端与数据模型
  player/                 mpv 嵌入（mpv.rs）与外置播放器（external.rs）
  win/                    Win32 视频宿主窗口
  commands/               暴露给前端的命令
```

## 配置文件

`%APPDATA%\ShenhePlayer\settings.json`，包含服务器列表、访问令牌与播放器偏好。

## 快捷键（播放时）

| 键 | 作用 |
| --- | --- |
| `空格` / `K` | 播放 / 暂停 |
| `←` / `→` | 后退 / 快进 10 秒（按住 `Shift` 为 60 秒） |
| `↑` / `↓` | 音量 |
| `F` | 全屏切换 |
| `M` | 静音 |
| `Esc` | 退出全屏 / 返回媒体库 |
