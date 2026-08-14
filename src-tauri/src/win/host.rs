//! 视频宿主窗口。
//!
//! mpv 需要一个 HWND 来渲染（`--wid=<hwnd>`）。直接把主窗口句柄丢给 mpv
//! 会让它接管整个客户区，无法控制层级和可见性；所以这里自己建一个子窗口：
//!
//! ```text
//! 主窗口 (Tauri, 无边框)
//!  ├─ WebView2 子窗口   ← 浏览界面
//!  └─ ShenheVideoHost   ← 播放时置于 WebView2 之上，mpv 在里面渲染
//! ```
//!
//! 控制条不画在这里，而是用一个独立的透明置顶窗口（overlay）叠加，
//! 这样 DWM 会正确合成，不需要和 WebView2 抢 alpha 通道。

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::CreateSolidBrush;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

const CLASS_NAME: PCWSTR = w!("ShenheVideoHost");

/// 视频宿主窗口句柄。跨线程传递时存 isize，使用前转回 HWND。
#[derive(Debug, Clone, Copy, Default)]
pub struct VideoHost {
    hwnd: isize,
    parent: isize,
}

unsafe impl Send for VideoHost {}
unsafe impl Sync for VideoHost {}

impl VideoHost {
    pub fn is_valid(&self) -> bool {
        self.hwnd != 0
    }

    pub fn hwnd(&self) -> HWND {
        HWND(self.hwnd as *mut core::ffi::c_void)
    }

    /// mpv `--wid` 需要的十进制句柄
    pub fn wid(&self) -> isize {
        self.hwnd
    }

    /// 在父窗口客户区内创建一个铺满的黑色子窗口。必须在主线程调用。
    pub fn create(parent: HWND) -> Result<Self, String> {
        unsafe {
            register_class()?;

            let mut rc = RECT::default();
            GetClientRect(parent, &mut rc).map_err(|e| e.to_string())?;

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                CLASS_NAME,
                PCWSTR::null(),
                WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
                0,
                0,
                rc.right - rc.left,
                rc.bottom - rc.top,
                Some(parent),
                None,
                None,
                None,
            )
            .map_err(|e| format!("创建视频窗口失败: {e}"))?;

            Ok(Self {
                hwnd: hwnd.0 as isize,
                parent: parent.0 as isize,
            })
        }
    }

    /// 显示并铺满父窗口客户区，层级置于 WebView2 之上。
    ///
    /// 不能反过来（把视频放在透明 WebView2 底下）：Tauri 的透明窗口带
    /// WS_EX_NOREDIRECTIONBITMAP，没有重定向表面，子窗口根本不参与合成，
    /// 视频不会显示。所以视频盖住 WebView2，控制条另开一个透明窗口叠加。
    pub fn show(&self) {
        if !self.is_valid() {
            return;
        }
        unsafe {
            self.fit_to_parent();
            let _ = ShowWindow(self.hwnd(), SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                self.hwnd(),
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    pub fn hide(&self) {
        if !self.is_valid() {
            return;
        }
        unsafe {
            let _ = ShowWindow(self.hwnd(), SW_HIDE);
        }
    }

    /// 父窗口尺寸变化时同步。mpv 会自己跟随宿主窗口的大小。
    pub fn fit_to_parent(&self) {
        if !self.is_valid() {
            return;
        }
        unsafe {
            let parent = HWND(self.parent as *mut core::ffi::c_void);
            let mut rc = RECT::default();
            if GetClientRect(parent, &mut rc).is_err() {
                return;
            }
            let _ = SetWindowPos(
                self.hwnd(),
                None,
                0,
                0,
                rc.right - rc.left,
                rc.bottom - rc.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    pub fn destroy(&mut self) {
        if !self.is_valid() {
            return;
        }
        unsafe {
            let _ = DestroyWindow(self.hwnd());
        }
        self.hwnd = 0;
    }
}

unsafe fn register_class() -> Result<(), String> {
    use std::sync::OnceLock;
    static REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();

    REGISTERED
        .get_or_init(|| unsafe {
            let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;

            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
                lpfnWndProc: Some(host_wnd_proc),
                hInstance: instance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                // 纯黑背景：seek / 切集时不会露出上一帧或白底
                hbrBackground: CreateSolidBrush(COLORREF(0)),
                lpszClassName: CLASS_NAME,
                ..Default::default()
            };

            if RegisterClassW(&wc) == 0 {
                let err = windows::core::Error::from_win32();
                // ERROR_CLASS_ALREADY_EXISTS 不算失败
                if err.code().0 as u32 != 0x8007_0582 {
                    return Err(format!("注册窗口类失败: {err}"));
                }
            }
            Ok(())
        })
        .clone()
}

extern "system" fn host_wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            // 交给 mpv 的子窗口去画，避免自己擦背景造成闪烁
            WM_ERASEBKGND => LRESULT(1),
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}
