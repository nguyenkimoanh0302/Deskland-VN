//! Cửa sổ Cài đặt giống hệt bản gốc: mở chính trang `prototype/settings.html` trong WebView2.
//! WebView2 CHỈ chạy khi cửa sổ đang mở; đóng cửa sổ là đóng controller -> các tiến trình
//! msedgewebview2 thoát, RAM về lại như cũ. Thông điệp trang gửi lên được xếp hàng (`take_inbox`),
//! vòng lặp chính xử lý rồi trả lời bằng `post`.

use crate::desktop::wide;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{CoTaskMemPWSTR, CreateCoreWebView2ControllerCompletedHandler, CreateCoreWebView2EnvironmentCompletedHandler, WebMessageReceivedEventHandler};
use windows::core::{Interface, HSTRING, PCWSTR, PWSTR};
use windows_sys::Win32::Foundation as F;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLASS: &str = "DesklandRsWebSettings";

/// `--webview-debug` (chỉ để phát triển / kiểm thử): mở cổng DevTools 9222 cho trang cài đặt.
pub static DEBUG_PORT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

struct Web {
    hwnd: F::HWND,
    controller: ICoreWebView2Controller,
    webview: ICoreWebView2,
}

thread_local! {
    static WEB: RefCell<Option<Web>> = const { RefCell::new(None) };
    static INBOX: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static COM_READY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Tìm thư mục `prototype` (có `settings.html` + `themes.json`): cạnh exe, hoặc đi ngược lên từ exe
/// (chạy từ `deskland-rs/target/release` trong repo).
pub fn find_prototype() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent();
    while let Some(d) = dir {
        for cand in [d.join("prototype"), d.to_path_buf()] {
            if cand.join("settings.html").is_file() && cand.join("themes.json").is_file() {
                return Some(cand);
            }
        }
        dir = d.parent();
    }
    None
}

pub fn is_open() -> bool {
    WEB.with(|w| w.try_borrow().map(|w| w.is_some()).unwrap_or(true))
}

pub fn take_inbox() -> Vec<String> {
    INBOX.with(|i| std::mem::take(&mut *i.borrow_mut()))
}

pub fn post(json: &str) {
    WEB.with(|w| {
        if let Ok(w) = w.try_borrow() {
            if let Some(w) = w.as_ref() {
                let s = HSTRING::from(json);
                let _ = unsafe { w.webview.PostWebMessageAsString(PCWSTR(s.as_ptr())) };
            }
        }
    });
}

fn light_theme() -> bool {
    use windows_sys::Win32::System::Registry::HKEY_CURRENT_USER;
    let k = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let n = wide("AppsUseLightTheme");
    let mut v = 1u32;
    let mut sz = 4u32;
    unsafe {
        windows_sys::Win32::System::Registry::RegGetValueW(
            HKEY_CURRENT_USER,
            k.as_ptr(),
            n.as_ptr(),
            windows_sys::Win32::System::Registry::RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut v as *mut u32 as *mut _,
            &mut sz,
        );
    }
    v != 0
}

/// Thanh tiêu đề theo chế độ sáng / tối của Windows (như bản C#).
fn theme_title_bar(hwnd: F::HWND) {
    use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
    let light = light_theme();
    let dark: i32 = if light { 0 } else { 1 };
    let color: u32 = if light { 0x00FF_FFFF } else { 0x0017_1717 };
    unsafe {
        DwmSetWindowAttribute(hwnd, 20, &dark as *const i32 as *const _, 4);
        DwmSetWindowAttribute(hwnd, 35, &color as *const u32 as *const _, 4);
    }
}

fn resize(hwnd: F::HWND) {
    WEB.with(|w| {
        if let Ok(w) = w.try_borrow() {
            if let Some(w) = w.as_ref() {
                let mut r: F::RECT = unsafe { std::mem::zeroed() };
                unsafe { GetClientRect(hwnd, &mut r) };
                let rr = windows::Win32::Foundation::RECT { left: 0, top: 0, right: r.right, bottom: r.bottom };
                let _ = unsafe { w.controller.SetBounds(rr) };
            }
        }
    });
}

unsafe extern "system" fn proc_(h: F::HWND, m: u32, wp: F::WPARAM, lp: F::LPARAM) -> F::LRESULT {
    match m {
        WM_SIZE => {
            resize(h);
            0
        }
        WM_GETMINMAXINFO => {
            let s = GetDpiForWindow(h).max(96) as f32 / 96.0;
            let mi = &mut *(lp as *mut MINMAXINFO);
            mi.ptMinTrackSize.x = (940.0 * s) as i32;
            mi.ptMinTrackSize.y = (680.0 * s) as i32;
            0
        }
        WM_SETTINGCHANGE => {
            theme_title_bar(h);
            DefWindowProcW(h, m, wp, lp)
        }
        WM_CLOSE => {
            DestroyWindow(h);
            0
        }
        WM_DESTROY => {
            if let Some(w) = WEB.with(|x| x.try_borrow_mut().ok().and_then(|mut g| g.take())) {
                // Close() dừng WebView2 -> các tiến trình msedgewebview2 của cửa sổ này thoát
                let _ = w.controller.Close();
            }
            crate::log!("Đóng cửa sổ cài đặt (WebView2 đã dừng)");
            0
        }
        _ => DefWindowProcW(h, m, wp, lp),
    }
}

/// Mở (hoặc đưa lên trước) cửa sổ cài đặt WebView2. Lỗi (không có WebView2 Runtime, không thấy
/// thư mục prototype...) trả về Err để bên gọi dùng cửa sổ Win32 dự phòng.
pub fn open(proto: &Path) -> Result<(), String> {
    if let Some(h) = WEB.with(|w| w.borrow().as_ref().map(|w| w.hwnd)) {
        unsafe {
            if IsIconic(h) != 0 {
                ShowWindow(h, SW_RESTORE);
            }
            SetForegroundWindow(h);
        }
        return Ok(());
    }
    let t0 = std::time::Instant::now();
    unsafe {
        if !COM_READY.with(|c| c.get()) {
            let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED);
            COM_READY.with(|c| c.set(true));
        }
        let cls = wide(CLASS);
        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(proc_);
        wc.hInstance = GetModuleHandleW(std::ptr::null());
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        wc.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
        wc.lpszClassName = cls.as_ptr();
        RegisterClassExW(&wc);

        let s = windows_sys::Win32::UI::HiDpi::GetDpiForSystem().max(96) as f32 / 96.0;
        let (w, h) = ((1120.0 * s) as i32, (800.0 * s) as i32);
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let hwnd = CreateWindowExW(
            0,
            cls.as_ptr(),
            wide("Deskland Preview").as_ptr(),
            WS_OVERLAPPEDWINDOW,
            ((sw - w) / 2).max(0),
            ((sh - h) / 2).max(0),
            w.min(sw),
            h.min(sh),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return Err(format!("CreateWindowEx lỗi {}", F::GetLastError()));
        }
        theme_title_bar(hwnd);
        // Nền trùng màu trang trong lúc WebView2 khởi động (khỏi chớp trắng ở chế độ tối)
        ShowWindow(hwnd, SW_SHOWNORMAL);

        match create(hwnd, proto) {
            Ok(web) => {
                WEB.with(|x| *x.borrow_mut() = Some(web));
                resize(hwnd);
                SetForegroundWindow(hwnd);
                crate::log!("Mở cửa sổ cài đặt WebView2 sau {:.0} ms ({})", t0.elapsed().as_secs_f64() * 1000.0, proto.display());
                Ok(())
            }
            Err(e) => {
                DestroyWindow(hwnd);
                Err(e)
            }
        }
    }
}

fn create(hwnd: F::HWND, proto: &Path) -> Result<Web, String> {
    let parent = windows::Win32::Foundation::HWND(hwnd as _);
    let data = HSTRING::from(crate::lock::data_dir().join("WebView2").to_string_lossy().as_ref());
    let opts = webview2_com::CoreWebView2EnvironmentOptions::default();
    if DEBUG_PORT.load(std::sync::atomic::Ordering::Relaxed) {
        unsafe { opts.set_additional_browser_arguments("--remote-debugging-port=9222".into()) };
    }
    let opts: ICoreWebView2EnvironmentOptions = opts.into();
    let environment = {
        let (tx, rx) = std::sync::mpsc::channel();
        CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
            Box::new(move |handler| unsafe {
                CreateCoreWebView2EnvironmentWithOptions(PCWSTR::null(), PCWSTR(data.as_ptr()), &opts, &handler)
                    .map_err(webview2_com::Error::WindowsError)
            }),
            Box::new(move |err, env| {
                err?;
                let _ = tx.send(env.ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_POINTER)));
                Ok(())
            }),
        )
        .map_err(|e| format!("Không khởi tạo được WebView2 (có cài WebView2 Runtime chưa?): {e:?}"))?;
        rx.recv().map_err(|e| e.to_string())?.map_err(|e| e.to_string())?
    };
    let controller = {
        let (tx, rx) = std::sync::mpsc::channel();
        CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
            Box::new(move |handler| unsafe { environment.CreateCoreWebView2Controller(parent, &handler).map_err(webview2_com::Error::WindowsError) }),
            Box::new(move |err, c| {
                err?;
                let _ = tx.send(c.ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_POINTER)));
                Ok(())
            }),
        )
        .map_err(|e| format!("Tạo WebView2 controller lỗi: {e:?}"))?;
        rx.recv().map_err(|e| e.to_string())?.map_err(|e| e.to_string())?
    };
    unsafe {
        if let Ok(c2) = controller.cast::<ICoreWebView2Controller2>() {
            let bg = if light_theme() { COREWEBVIEW2_COLOR { A: 255, R: 255, G: 255, B: 255 } } else { COREWEBVIEW2_COLOR { A: 255, R: 23, G: 23, B: 23 } };
            let _ = c2.SetDefaultBackgroundColor(bg);
        }
        let _ = controller.SetIsVisible(true);
        let webview = controller.CoreWebView2().map_err(|e| e.to_string())?;
        let st = webview.Settings().map_err(|e| e.to_string())?;
        let _ = st.SetAreDefaultContextMenusEnabled(false);
        let _ = st.SetIsStatusBarEnabled(false);
        let _ = st.SetIsZoomControlEnabled(false);
        let _ = st.SetAreDevToolsEnabled(cfg!(debug_assertions));
        let w3 = webview.cast::<ICoreWebView2_3>().map_err(|e| format!("WebView2 Runtime quá cũ: {e}"))?;
        let host = HSTRING::from("app.local");
        let dir = HSTRING::from(proto.to_string_lossy().as_ref());
        w3.SetVirtualHostNameToFolderMapping(PCWSTR(host.as_ptr()), PCWSTR(dir.as_ptr()), COREWEBVIEW2_HOST_RESOURCE_ACCESS_KIND_ALLOW).map_err(|e| e.to_string())?;
        let mut token = 0i64;
        webview
            .add_WebMessageReceived(
                &WebMessageReceivedEventHandler::create(Box::new(|_wv, args| {
                    if let Some(args) = args {
                        let mut p = PWSTR(std::ptr::null_mut());
                        if args.TryGetWebMessageAsString(&mut p).is_ok() {
                            let s = CoTaskMemPWSTR::from(p).to_string();
                            INBOX.with(|i| i.borrow_mut().push(s));
                        }
                    }
                    Ok(())
                })),
                &mut token,
            )
            .map_err(|e| e.to_string())?;
        let url = HSTRING::from(format!("https://app.local/settings.html?v={}", env!("CARGO_PKG_VERSION")));
        webview.Navigate(PCWSTR(url.as_ptr())).map_err(|e| e.to_string())?;
        Ok(Web { hwnd, controller, webview })
    }
}
