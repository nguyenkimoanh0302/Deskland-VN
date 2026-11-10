//! deskland-rs: bản Rust native (1 tiến trình, OpenGL) của Deskland - spike đo hiệu năng.
//! Chạy:   deskland-rs.exe [--theme beach|meadow] [--fps 20]
//! Thoát:  deskland-rs.exe --quit   (hoặc menu khay hệ thống)

#![windows_subsystem = "windows"]

mod desktop;
mod engine;
mod lock;
mod settings;
mod web;
mod themes {
    pub mod beach;
    pub mod cat_hill;
    pub mod lake;
    pub mod maldives;
    pub mod meadow;
    pub mod mocchau;
    pub mod ocean;
    pub mod rainforest;
    pub mod sky;
    pub mod tet_sakura;
}
mod wgl;

use desktop::wide;
use engine::{IconBox, Prog, Shared, Theme};
use glow::HasContext;
use std::cell::Cell;
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::Graphics::OpenGL::SwapBuffers;
use windows_sys::Win32::Media::timeBeginPeriod;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::ProcessStatus::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::Accessibility::*;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ---------------------------------------------------------------- Nhật ký
static LOG: Mutex<Option<File>> = Mutex::new(None);

pub fn log_line(s: &str) {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    if let Ok(mut g) = LOG.lock() {
        if let Some(f) = g.as_mut() {
            let _ = writeln!(f, "[{t:.3}] {s}");
            let _ = f.flush();
        }
    }
}

#[macro_export]
macro_rules! log {
    ($($t:tt)*) => { $crate::log_line(&format!($($t)*)) };
}

// ---------------------------------------------------------------- Thông điệp cửa sổ
const MSG_CLASS: &str = "DesklandRsMsg";
const WALL_CLASS: &str = "DesklandRsWall";
const SAVER_CLASS: &str = "DesklandRsSaver";
const WM_TRAY: u32 = WM_APP + 1;
const CMD_LOCK: usize = 120;
const CMD_SAVER: usize = 121;
const CMD_SYNC: usize = 122;
const CMD_WINL: usize = 123;
const CMD_SAVER_NOW: usize = 124;
const CMD_SETTINGS: usize = 125;
const WM_OPEN_SETTINGS: u32 = WM_APP + 2;
const CMD_EXIT: usize = 199;
const HOTKEY_SAVER: i32 = 1;
const WM_WTSSESSION_CHANGE_: u32 = 0x02B1;
const WTS_SESSION_LOCK_: usize = 0x7;
const WTS_SESSION_UNLOCK_: usize = 0x8;

pub const THEMES: &[(&str, &str, usize)] = &[
    ("meadow", "Thảo nguyên", 101),
    ("beach", "Bãi biển", 102),
    ("beach-turquoise", "Đảo Maldives", 103),
    ("lake", "Hồ nước", 104),
    ("sky", "Bầu trời", 105),
    ("ocean", "Đại dương", 106),
    ("rainforest", "Rừng mưa nhiệt đới", 107),
    ("mocchau", "Hoa mận", 108),
    ("tet-sakura", "Hoa anh đào Tết", 109),
    ("cat-hill", "Đồi mèo nắng ấm", 110),
];

pub fn canonical_theme(id: &str) -> Option<&'static str> {
    match id {
        "maldives" => Some("beach-turquoise"),
        "plum-blossoms" | "plum_blossoms" | "plum" => Some("mocchau"),
        "tet_sakura" => Some("tet-sakura"),
        "cat_hill" => Some("cat-hill"),
        other => THEMES.iter().find(|&&(tid, _, _)| tid == other).map(|&(tid, _, _)| tid),
    }
}

pub fn theme_cmd(id: &str) -> usize {
    let cid = canonical_theme(id).unwrap_or("beach");
    THEMES.iter().find(|&&(tid, _, _)| tid == cid).map(|&(_, _, cmd)| cmd).unwrap_or(102)
}

pub fn cmd_theme(cmd: usize) -> Option<&'static str> {
    THEMES.iter().find(|&&(_, _, c)| c == cmd).map(|&(tid, _, _)| tid)
}

thread_local! {
    static PENDING_CMD: Cell<usize> = const { Cell::new(0) };
    static ZORDER_DIRTY: Cell<bool> = const { Cell::new(false) };
    static DISPLAY_CHANGED: Cell<bool> = const { Cell::new(false) };
    static REPAINT: Cell<bool> = const { Cell::new(false) };
    static LOCK_SYNC: Cell<bool> = const { Cell::new(true) };
    static PENDING_THEME_SYNC: Cell<bool> = const { Cell::new(false) };
    static SAVER_START: Cell<Option<Instant>> = const { Cell::new(None) };
    static SAVER_MOUSE: Cell<Option<(i32, i32)>> = const { Cell::new(None) };
    static CURRENT_THEME: Cell<usize> = const { Cell::new(102) };
    /// Phiên Windows đang khóa (WM_WTSSESSION_CHANGE): không ai thấy desktop -> ngừng vẽ.
    static SESSION_LOCKED: Cell<bool> = const { Cell::new(false) };
    static HOTKEY_OK: Cell<bool> = const { Cell::new(false) };
}

unsafe extern "system" fn msg_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        WM_CLOSE => {
            PostQuitMessage(0);
            0
        }
        WM_DISPLAYCHANGE => {
            DISPLAY_CHANGED.with(|c| c.set(true));
            0
        }
        WM_WTSSESSION_CHANGE_ => {
            match w {
                WTS_SESSION_LOCK_ => SESSION_LOCKED.with(|c| c.set(true)),
                WTS_SESSION_UNLOCK_ => SESSION_LOCKED.with(|c| c.set(false)),
                _ => {}
            }
            0
        }
        WM_OPEN_SETTINGS => {
            PENDING_CMD.with(|c| c.set(CMD_SETTINGS));
            0
        }
        WM_HOTKEY if w as i32 == HOTKEY_SAVER => {
            // Nhận WM_HOTKEY cho phép tiến trình đưa cửa sổ lên trước (cần cho SC_SCREENSAVE)
            SetForegroundWindow(h);
            PENDING_CMD.with(|c| c.set(CMD_SAVER_NOW));
            0
        }
        WM_TRAY => {
            let ev = (l as u32) & 0xFFFF;
            if ev == WM_LBUTTONUP {
                PENDING_CMD.with(|c| c.set(CMD_SETTINGS));
            } else if ev == WM_RBUTTONUP || ev == WM_CONTEXTMENU {
                let menu = CreatePopupMenu();
                AppendMenuW(menu, MF_STRING, CMD_SETTINGS, wide("Cài đặt…").as_ptr());
                SetMenuDefaultItem(menu, CMD_SETTINGS as u32, 0);
                AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                let cur = CURRENT_THEME.with(|c| c.get());
                let flag = |id: usize| if cur == id { MF_STRING | MF_CHECKED } else { MF_STRING };
                for &(_, label, cmd_id) in THEMES {
                    AppendMenuW(menu, flag(cmd_id), cmd_id, wide(label).as_ptr());
                }
                AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                // Màn hình khóa: chỉ dùng khóa thật của Windows
                AppendMenuW(menu, MF_STRING, CMD_LOCK, wide("Khóa máy ngay (khóa Windows)").as_ptr());
                let label = if HOTKEY_OK.with(|c| c.get()) { "Khóa với cảnh động\tCtrl+Alt+L" } else { "Khóa với cảnh động" };
                AppendMenuW(menu, MF_STRING, CMD_SAVER_NOW, wide(label).as_ptr());
                let chk = |on: bool| if on { MF_STRING | MF_CHECKED } else { MF_STRING };
                AppendMenuW(menu, chk(lock::saver_installed()), CMD_SAVER, wide("Cảnh động khi rảnh (trình bảo vệ màn hình, khóa khi quay lại)").as_ptr());
                AppendMenuW(menu, chk(LOCK_SYNC.with(|c| c.get())), CMD_SYNC, wide("Dùng cảnh làm ảnh màn khóa Windows").as_ptr());
                if lock::winl_disabled() {
                    AppendMenuW(menu, MF_STRING, CMD_WINL, wide("⚠ Bật lại Win+L của Windows (bản C# cũ đã tắt)").as_ptr());
                }
                AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                AppendMenuW(menu, MF_STRING, CMD_EXIT, wide("Thoát Deskland (Rust)").as_ptr());
                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);
                SetForegroundWindow(h);
                let cmd = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON, pt.x, pt.y, 0, h, std::ptr::null());
                DestroyMenu(menu);
                if cmd > 0 {
                    PENDING_CMD.with(|c| c.set(cmd as usize));
                }
            }
            0
        }
        _ => DefWindowProcW(h, m, w, l),
    }
}

unsafe extern "system" fn wall_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            ValidateRect(h, std::ptr::null());
            REPAINT.with(|c| c.set(true));
            0
        }
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        _ => DefWindowProcW(h, m, w, l),
    }
}

/// Cửa sổ trình bảo vệ màn hình: có bất kỳ thao tác nào thì thoát. Việc khóa / mở khóa do Windows
/// làm (ScreenSaverIsSecure), Deskland không kiểm mật khẩu và không chặn phím nào.
unsafe extern "system" fn saver_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let warm = SAVER_START.with(|c| c.get()).is_some_and(|t| t.elapsed().as_millis() > 800);
    match m {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            ValidateRect(h, std::ptr::null());
            REPAINT.with(|c| c.set(true));
            0
        }
        WM_SETCURSOR => {
            SetCursor(std::ptr::null_mut());
            1
        }
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN | WM_MOUSEWHEEL => {
            if warm {
                PostQuitMessage(0);
            }
            0
        }
        WM_MOUSEMOVE => {
            let (x, y) = ((l & 0xFFFF) as i16 as i32, ((l >> 16) & 0xFFFF) as i16 as i32);
            match SAVER_MOUSE.with(|c| c.get()) {
                None => SAVER_MOUSE.with(|c| c.set(Some((x, y)))),
                Some((x0, y0)) if warm && ((x - x0).abs() > 6 || (y - y0).abs() > 6) => PostQuitMessage(0),
                _ => {}
            }
            0
        }
        WM_ACTIVATEAPP if w == 0 && warm => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(h, m, w, l),
    }
}

unsafe extern "system" fn win_event(_: HWINEVENTHOOK, _ev: u32, _h: HWND, id_object: i32, _: i32, _: u32, _: u32) {
    if id_object == 0 {
        ZORDER_DIRTY.with(|c| c.set(true));
    }
}

fn register_class(name: &str, proc_: WNDPROC, own_dc: bool) {
    unsafe {
        let cls = wide(name);
        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.style = if own_dc { CS_OWNDC } else { 0 };
        wc.lpfnWndProc = proc_;
        wc.hInstance = GetModuleHandleW(std::ptr::null());
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        wc.lpszClassName = cls.as_ptr();
        RegisterClassExW(&wc);
    }
}

// ---------------------------------------------------------------- Màn hình
struct Monitor {
    rect: RECT,
    dpr: f32,
    primary: bool,
}

unsafe extern "system" fn enum_monitor(hm: HMONITOR, _: HDC, _: *mut RECT, l: LPARAM) -> BOOL {
    let out = &mut *(l as *mut Vec<Monitor>);
    let mut mi: MONITORINFO = std::mem::zeroed();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(hm, &mut mi) != 0 {
        let (mut dx, mut dy) = (96u32, 96u32);
        GetDpiForMonitor(hm, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        out.push(Monitor { rect: mi.rcMonitor, dpr: dx as f32 / 96.0, primary: mi.dwFlags & MONITORINFOF_PRIMARY != 0 });
    }
    1
}

fn monitors() -> Vec<Monitor> {
    let mut v: Vec<Monitor> = Vec::new();
    unsafe { EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(enum_monitor), &mut v as *mut Vec<Monitor> as LPARAM) };
    v.sort_by_key(|m| !m.primary);
    v
}

// ---------------------------------------------------------------- Cài đặt (đọc chung với bản C#)
struct Config {
    theme: String,
    params: serde_json::Value,
    /// Đặt ảnh cảnh hiện tại làm ảnh màn khóa Windows (`lockSyncWin`, mặc định bật như bản C#)
    lock_sync: bool,
    /// Số phút rảnh trước khi trình bảo vệ màn hình chạy (`lockIdleMins`)
    lock_idle_mins: u32,
    /// Ngôn ngữ giao diện trang cài đặt: "auto" | "vi" | "en"
    lang: String,
}

fn rust_settings_path() -> std::path::PathBuf {
    lock::data_dir().join("settings.json")
}

/// Đọc `~/.deskland-rs/settings.json` (cài đặt riêng của bản Rust); chưa có thì đọc của bản C#
/// (`%LOCALAPPDATA%/DesklandPreview/settings.json`) để giữ chủ đề / thông số đang dùng.
fn load_config(cli_theme: Option<String>) -> Config {
    let read = |p: &std::path::Path| std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(s.trim_start_matches('\u{feff}')).ok());
    let cs = std::env::var("LOCALAPPDATA").map(|d| std::path::PathBuf::from(d).join("DesklandPreview").join("settings.json")).unwrap_or_default();
    let csv = read(&cs);
    let root = read(&rust_settings_path()).or_else(|| csv.clone()).unwrap_or(serde_json::Value::Null);
    // Ngôn ngữ: bản Rust chưa lưu thì theo lựa chọn ở bản C#
    let lang = root.get("lang").or_else(|| csv.as_ref().and_then(|c| c.get("lang"))).and_then(|v| v.as_str()).unwrap_or("auto").to_string();
    let saved = root.get("theme").and_then(|v| v.as_str()).unwrap_or("beach").to_string();
    let raw_theme = cli_theme.unwrap_or(saved);
    let theme = match canonical_theme(&raw_theme) {
        Some(t) => t.to_string(),
        None => {
            log!("Chủ đề '{raw_theme}' không hợp lệ, dùng 'beach'");
            "beach".into()
        }
    };
    Config {
        theme,
        params: root.get("params").cloned().unwrap_or(serde_json::Value::Null),
        lock_sync: root.get("lockSyncWin").and_then(|v| v.as_bool()).unwrap_or(true),
        lang,
        lock_idle_mins: root.get("lockIdleMins").and_then(|v| v.as_u64()).map(|v| v as u32).filter(|&v| v > 0).unwrap_or(5),
    }
}

/// Lưu cài đặt của bản Rust (không đụng file của bản C#). Không lưu mật khẩu / PIN nào.
fn save_config(cfg: &Config) {
    let v = serde_json::json!({
        "version": 1,
        "theme": cfg.theme,
        "lockSyncWin": LOCK_SYNC.with(|c| c.get()),
        "lockIdleMins": cfg.lock_idle_mins,
        "lang": cfg.lang,
        "params": cfg.params,
    });
    let p = rust_settings_path();
    if let Err(e) = std::fs::write(&p, serde_json::to_string_pretty(&v).unwrap_or_default()) {
        log!("Lưu cài đặt {} lỗi: {e}", p.display());
    }
}

/// Ngôn ngữ thực dùng + ngôn ngữ hệ thống cho trang cài đặt.
fn lang_eff(setting: &str) -> (String, String) {
    let id = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
    let sys = if id & 0x3FF == 0x2A { "vi" } else { "en" }.to_string();
    let eff = if setting == "vi" || setting == "en" { setting.to_string() } else { sys.clone() };
    (eff, sys)
}

/// Màn hình chính trong danh sách cửa sổ hình nền.
fn primary_wall(app: &App) -> Option<&Wallpaper> {
    app.walls.iter().find(|w| w.screen.left == 0 && w.screen.top == 0).or(app.walls.first())
}

/// Thông điệp `init` cho trang cài đặt (cùng định dạng bản C#), kèm `caps` nói rõ bản Rust làm được gì.
fn web_init(app: &App) -> serde_json::Value {
    let (w, h) = primary_wall(app).map(|p| (p.screen.right - p.screen.left, p.screen.bottom - p.screen.top)).unwrap_or((1920, 1080));
    let (eff, sys) = lang_eff(&app.cfg.lang);
    serde_json::json!({
        "t": "init",
        "theme": app.cfg.theme,
        "params": app.cfg.params,
        "screen": { "w": w, "h": h },
        "autostart": settings::autostart_enabled(),
        "autostartWarn": false,
        "lang": app.cfg.lang, "langEff": eff, "langSys": sys,
        "lockCfg": {
            "idleMins": if lock::saver_installed() { app.cfg.lock_idle_mins } else { 0 },
            "syncWin": LOCK_SYNC.with(|c| c.get()),
        },
        "caps": {
            // Chỉ dùng khóa thật của Windows: trang hiện nhóm tùy chọn khóa riêng, không có PIN / chặn Win+L
            "realLock": true,
            "hotkey": HOTKEY_OK.with(|c| c.get()),
            "share": false,
            "themes": [
                "meadow",
                "beach",
                "beach-turquoise",
                "lake",
                "sky",
                "ocean",
                "rainforest",
                "mocchau",
                "tet-sakura",
                "cat-hill"
            ],
            // Chưa port: cây dừa, cậu bé, mèo -> ẩn thanh trượt, xem trước cũng tắt cho giống desktop
            "hide": ["palms", "showBoy", "showCat"],
            "force": { "palms": 0, "showBoy": 0, "showCat": 0 },
        },
    })
}

/// Vị trí biểu tượng trên màn hình chính (toạ độ trong màn hình, điểm ảnh vật lý) như bản C#.
fn icons_json(app: &App) -> String {
    let Some(p) = primary_wall(app) else { return String::new() };
    let s = p.screen;
    let r: Vec<[i32; 4]> = p.last_icons.as_deref().unwrap_or(&[]).iter().map(|r| [r.left - s.left, r.top - s.top, r.right - s.left, r.bottom - s.top]).collect();
    serde_json::json!({ "t": "icons", "vis": !r.is_empty(), "r": r }).to_string()
}

// ---------------------------------------------------------------- Một cửa sổ hình nền / màn hình
struct Wallpaper {
    hwnd: HWND,
    hdc: HDC,
    /// DC dùng cho wglMakeCurrent: chính cửa sổ (chế độ gl) hoặc cửa sổ ẩn (chế độ gdi)
    gl_dc: HDC,
    /// Chế độ gdi: FBO độ phân giải thật (GPU phóng to LINEAR) + PBO đọc về không chặn.
    /// Khung N được đọc vào PBO, đầu khung N+1 (sau ~50 ms, GPU đã xong) mới map và vẽ ra cửa sổ.
    out: Option<(glow::Framebuffer, glow::Texture, glow::Buffer)>,
    pbo_ready: bool,
    screen: RECT,
    dpr: f32,
    /// None ở chế độ trình bảo vệ màn hình (cửa sổ toàn màn hình trên cùng, không gắn vào desktop)
    attach: Option<desktop::Attach>,
    theme: Box<dyn Theme>,
    fbo: glow::Framebuffer,
    fbo_tex: glow::Texture,
    buf_w: i32,
    buf_h: i32,
    phys_w: i32,
    phys_h: i32,
    last_icons: Option<Vec<RECT>>,
    paused: bool,
    needs_settle: bool,
    last_frame: f64,
}

struct App {
    ctx: Option<wgl::GlContext>,
    shared: Option<Shared>,
    vao: Option<glow::VertexArray>,
    walls: Vec<Wallpaper>,
    reader: desktop::IconReader,
    cfg: Config,
    start: Instant,
    frames: u64,
    frame_cpu_ms: f64,
    /// true: OpenGL chỉ vẽ vào FBO, đọc về rồi StretchDIBits lên cửa sổ hình nền (chắc chắn hiện
    /// trên cửa sổ con phân lớp). false: blit + SwapBuffers trực tiếp (nhanh hơn nhưng nhiều driver
    /// không hiển thị OpenGL trên cửa sổ WS_EX_LAYERED).
    gdi: bool,
    dummy: Option<(HWND, HDC)>,
    /// Chạy như trình bảo vệ màn hình Windows (`/s`)
    saver: bool,
}

impl App {
    fn ensure_dummy(&mut self) -> Result<HDC, String> {
        if let Some((_, dc)) = self.dummy {
            return Ok(dc);
        }
        let cls = wide(WALL_CLASS);
        let hwnd = unsafe {
            CreateWindowExW(WS_EX_TOOLWINDOW, cls.as_ptr(), cls.as_ptr(), WS_POPUP, 0, 0, 16, 16, std::ptr::null_mut(), std::ptr::null_mut(), GetModuleHandleW(std::ptr::null()), std::ptr::null())
        };
        if hwnd.is_null() {
            return Err("Không tạo được cửa sổ ẩn cho OpenGL".into());
        }
        let dc = unsafe { GetDC(hwnd) };
        if !wgl::set_pixel_format(dc) {
            return Err("SetPixelFormat (cửa sổ ẩn) thất bại".into());
        }
        self.dummy = Some((hwnd, dc));
        Ok(dc)
    }

    /// Đẩy khung hình đang nằm trong PBO lên cửa sổ hình nền bằng GDI (không phóng to trên CPU).
    /// Ngữ cảnh GL phải đang current.
    fn present_gdi(gl: &glow::Context, wp: &Wallpaper) {
        let Some((_, _, pbo)) = wp.out else { return };
        if !wp.pbo_ready {
            return;
        }
        let n = (wp.phys_w * wp.phys_h * 4) as i32;
        unsafe {
            gl.bind_buffer(glow::PIXEL_PACK_BUFFER, Some(pbo));
            let ptr = gl.map_buffer_range(glow::PIXEL_PACK_BUFFER, 0, n, glow::MAP_READ_BIT);
            if !ptr.is_null() {
                let mut bmi: BITMAPINFO = std::mem::zeroed();
                bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                bmi.bmiHeader.biWidth = wp.phys_w;
                bmi.bmiHeader.biHeight = wp.phys_h; // dương = hàng dưới lên trên, trùng thứ tự OpenGL
                bmi.bmiHeader.biPlanes = 1;
                bmi.bmiHeader.biBitCount = 32;
                bmi.bmiHeader.biCompression = BI_RGB;
                SetDIBitsToDevice(wp.hdc, 0, 0, wp.phys_w as u32, wp.phys_h as u32, 0, 0, 0, wp.phys_h as u32, ptr as *const _, &bmi, DIB_RGB_COLORS);
                gl.unmap_buffer(glow::PIXEL_PACK_BUFFER);
            }
            gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
        }
    }

    fn repaint_all(&self) {
        if !self.gdi {
            return;
        }
        let Some(ctx) = self.ctx.as_ref() else { return };
        for wp in &self.walls {
            ctx.make_current(wp.gl_dc);
            Self::present_gdi(&ctx.gl, wp);
        }
    }

    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    fn seed(&self) -> i32 {
        self.cfg
            .params
            .get(&self.cfg.theme)
            .and_then(|p| p.get("seed"))
            .and_then(|s| s.as_i64())
            .map(|s| s as i32)
            .unwrap_or_else(|| (self.start.elapsed().as_nanos() % 1_000_000) as i32)
    }

    fn make_theme(&self, gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, idx: usize) -> Box<dyn Theme> {
        let seed = self.seed();
        let p = self.cfg.params.get(&self.cfg.theme);
        let now = self.now();
        let rs = 0x5EED_0000 + idx as u64 * 7919 + self.start.elapsed().as_nanos() as u64;
        match self.cfg.theme.as_str() {
            "meadow" => Box::new(themes::meadow::Meadow::new(gl, w, h, buf_w, buf_h, scale, seed, themes::meadow::Params::from_json(p), rs, now)),
            "beach-turquoise" => Box::new(themes::maldives::Maldives::new(gl, w, h, buf_w, buf_h, scale, seed, themes::maldives::Params::from_json(p), rs, now)),
            "lake" => Box::new(themes::lake::Lake::new(gl, w, h, buf_w, buf_h, scale, seed, themes::lake::Params::from_json(p), rs, now)),
            "sky" => Box::new(themes::sky::Sky::new(gl, w, h, buf_w, buf_h, scale, seed, themes::sky::Params::from_json(p), rs, now)),
            "ocean" => Box::new(themes::ocean::Ocean::new(gl, w, h, buf_w, buf_h, scale, seed, themes::ocean::Params::from_json(p), rs, now)),
            "rainforest" => Box::new(themes::rainforest::Rainforest::new(gl, w, h, buf_w, buf_h, scale, seed, themes::rainforest::Params::from_json(p), rs, now)),
            "mocchau" => Box::new(themes::mocchau::PlumBlossoms::new(gl, w, h, buf_w, buf_h, scale, seed, themes::mocchau::Params::from_json(p), rs, now)),
            "tet-sakura" => Box::new(themes::tet_sakura::TetSakura::new(gl, w, h, buf_w, buf_h, scale, seed, themes::tet_sakura::Params::from_json(p), rs, now)),
            "cat-hill" => Box::new(themes::cat_hill::CatHill::new(gl, w, h, buf_w, buf_h, scale, seed, themes::cat_hill::Params::from_json(p), rs, now)),
            _ => Box::new(themes::beach::Beach::new(gl, w, h, buf_w, buf_h, scale, seed, themes::beach::Params::from_json(p), rs, now)),
        }
    }

    fn build_shared(&mut self) -> Result<(), String> {
        let ctx = self.ctx.as_ref().unwrap();
        let gl = &ctx.gl;
        let seed = self.seed();
        let noise = engine::Noise { seed };
        let frag = match self.cfg.theme.as_str() {
            "meadow" => themes::meadow::FRAG,
            "beach-turquoise" => themes::maldives::FRAG,
            "lake" => themes::lake::FRAG,
            "sky" => themes::sky::FRAG,
            "ocean" => themes::ocean::FRAG,
            "rainforest" => themes::rainforest::FRAG,
            "mocchau" => themes::mocchau::FRAG,
            "tet-sakura" => themes::tet_sakura::FRAG,
            "cat-hill" => themes::cat_hill::FRAG,
            _ => themes::beach::FRAG,
        };
        let t0 = Instant::now();
        let prog = Prog::new(gl, frag)?;
        log!("Biên dịch shader '{}' mất {:.0} ms", self.cfg.theme, t0.elapsed().as_secs_f64() * 1000.0);
        let noise_tex = engine::build_noise_tex(gl, &noise);
        self.shared = Some(Shared { noise_tex, seed, prog });
        Ok(())
    }

    fn drop_shared(&mut self) {
        if let (Some(ctx), Some(s)) = (self.ctx.as_ref(), self.shared.take()) {
            unsafe {
                ctx.gl.delete_program(s.prog.prog);
                ctx.gl.delete_texture(s.noise_tex);
            }
        }
    }

    fn create_walls(&mut self) -> Result<(), String> {
        self.ensure_seed();
        let mons = monitors();
        log!("Phát hiện {} màn hình", mons.len());
        let cls = wide(if self.saver { SAVER_CLASS } else { WALL_CLASS });
        let title = wide("DesklandWindow");
        let ex_style = if self.saver { WS_EX_TOPMOST | WS_EX_TOOLWINDOW } else { WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE };
        for (idx, m) in mons.iter().enumerate() {
            let (pw, ph) = (m.rect.right - m.rect.left, m.rect.bottom - m.rect.top);
            let hwnd = unsafe {
                CreateWindowExW(
                    ex_style,
                    cls.as_ptr(),
                    title.as_ptr(),
                    WS_POPUP,
                    m.rect.left,
                    m.rect.top,
                    pw,
                    ph,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    GetModuleHandleW(std::ptr::null()),
                    std::ptr::null(),
                )
            };
            if hwnd.is_null() {
                return Err("CreateWindowExW thất bại".into());
            }
            let attach = if self.saver {
                unsafe {
                    SetWindowPos(hwnd, HWND_TOPMOST, m.rect.left, m.rect.top, pw, ph, SWP_SHOWWINDOW);
                }
                None
            } else {
                Some(desktop::attach(hwnd, m.rect).ok_or("Không gắn được vào lớp hình nền")?)
            };
            let hdc = unsafe { GetDC(hwnd) };
            let gl_dc = if self.gdi {
                self.ensure_dummy()?
            } else {
                if !wgl::set_pixel_format(hdc) {
                    return Err("SetPixelFormat thất bại".into());
                }
                hdc
            };
            if self.ctx.is_none() {
                let ctx = wgl::GlContext::new(gl_dc)?;
                unsafe {
                    log!(
                        "OpenGL: {} | {} | GLSL {}",
                        ctx.gl.get_parameter_string(glow::RENDERER),
                        ctx.gl.get_parameter_string(glow::VERSION),
                        ctx.gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION)
                    );
                    let vao = ctx.gl.create_vertex_array()?;
                    ctx.gl.bind_vertex_array(Some(vao));
                    self.vao = Some(vao);
                }
                self.ctx = Some(ctx);
                self.build_shared()?;
            }
            let ctx = self.ctx.as_ref().unwrap();
            ctx.make_current(gl_dc);
            // Giống bản JS: canvas = kích thước CSS * min(dpr, 0.72), trình duyệt phóng lên toàn màn hình
            let (w, h) = (pw as f32 / m.dpr, ph as f32 / m.dpr);
            let scale = m.dpr.min(0.72);
            let (buf_w, buf_h) = ((w * scale).round() as i32, (h * scale).round() as i32);
            let gl = &ctx.gl;
            let (fbo, fbo_tex) = unsafe {
                let t = gl.create_texture()?;
                gl.bind_texture(glow::TEXTURE_2D, Some(t));
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, buf_w, buf_h, 0, glow::RGBA, glow::UNSIGNED_BYTE, None);
                engine::set_tex_params(gl, glow::LINEAR, glow::CLAMP_TO_EDGE);
                let f = gl.create_framebuffer()?;
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(f));
                gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(t), 0);
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                (f, t)
            };
            let out = if self.gdi {
                unsafe {
                    let t = gl.create_texture()?;
                    gl.bind_texture(glow::TEXTURE_2D, Some(t));
                    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, pw, ph, 0, glow::RGBA, glow::UNSIGNED_BYTE, None);
                    let f = gl.create_framebuffer()?;
                    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(f));
                    gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(t), 0);
                    gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                    let b = gl.create_buffer()?;
                    gl.bind_buffer(glow::PIXEL_PACK_BUFFER, Some(b));
                    gl.buffer_data_size(glow::PIXEL_PACK_BUFFER, pw * ph * 4, glow::STREAM_READ);
                    gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
                    Some((f, t, b))
                }
            } else {
                None
            };
            let theme = self.make_theme(gl, w, h, buf_w, buf_h, scale, idx);
            log!("Màn hình {idx}: {pw}x{ph}, dpr {:.2}, bộ đệm vẽ {buf_w}x{buf_h}", m.dpr);
            self.walls.push(Wallpaper {
                hwnd,
                hdc,
                gl_dc,
                out,
                pbo_ready: false,
                screen: m.rect,
                dpr: m.dpr,
                attach,
                theme,
                fbo,
                fbo_tex,
                buf_w,
                buf_h,
                phys_w: pw,
                phys_h: ph,
                last_icons: None,
                paused: false,
                needs_settle: false,
                last_frame: 0.0,
            });
        }
        Ok(())
    }

    fn destroy_walls(&mut self) {
        let walls = std::mem::take(&mut self.walls);
        for mut wp in walls {
            if let Some(ctx) = self.ctx.as_ref() {
                ctx.make_current(wp.gl_dc);
                wp.theme.destroy(&ctx.gl);
                unsafe {
                    ctx.gl.delete_framebuffer(wp.fbo);
                    ctx.gl.delete_texture(wp.fbo_tex);
                    if let Some((f, t, b)) = wp.out.take() {
                        ctx.gl.delete_framebuffer(f);
                        ctx.gl.delete_texture(t);
                        ctx.gl.delete_buffer(b);
                    }
                }
            }
            if let Some(a) = &wp.attach {
                desktop::detach(wp.hwnd, a);
            }
            unsafe {
                ReleaseDC(wp.hwnd, wp.hdc);
                DestroyWindow(wp.hwnd);
            }
        }
    }

    fn switch_theme(&mut self, theme: &str) {
        if self.cfg.theme == theme {
            return;
        }
        log!("Đổi chủ đề -> {theme}");
        self.cfg.theme = theme.into();
        self.rebuild_theme();
    }

    /// Thông số của chủ đề hiện tại (tạo object nếu chưa có).
    fn theme_params_mut(&mut self) -> &mut serde_json::Map<String, serde_json::Value> {
        if !self.cfg.params.is_object() {
            self.cfg.params = serde_json::json!({});
        }
        let theme = self.cfg.theme.clone();
        let root = self.cfg.params.as_object_mut().unwrap();
        let e = root.entry(theme).or_insert_with(|| serde_json::json!({}));
        if !e.is_object() {
            *e = serde_json::json!({});
        }
        e.as_object_mut().unwrap()
    }

    /// Giữ cố định bố cục khi chỉnh thông số: chưa có seed thì tạo và lưu lại.
    fn ensure_seed(&mut self) {
        let fresh = (self.start.elapsed().as_nanos() % 1_000_000) as i64;
        self.theme_params_mut().entry("seed").or_insert(serde_json::json!(fresh));
    }

    fn reseed(&mut self) {
        let fresh = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) % 1_000_000) as i64;
        self.theme_params_mut().insert("seed".into(), serde_json::json!(fresh));
    }

    fn settings_state(&self) -> settings::State {
        settings::State {
            theme: self.cfg.theme.clone(),
            params: self.cfg.params.get(&self.cfg.theme).cloned().unwrap_or(serde_json::Value::Null),
            saver: lock::saver_installed(),
            idle: self.cfg.lock_idle_mins,
            sync: LOCK_SYNC.with(|c| c.get()),
            autostart: settings::autostart_enabled(),
            hotkey: HOTKEY_OK.with(|c| c.get()),
        }
    }

    /// Dựng lại cảnh của chủ đề hiện tại (đổi chủ đề / thông số / seed).
    fn rebuild_theme(&mut self) {
        let now = self.now();
        self.drop_shared();
        if let Err(e) = self.build_shared() {
            log!("Lỗi dựng chủ đề: {e}");
            return;
        }
        let ctx = self.ctx.take().unwrap();
        let mut walls = std::mem::take(&mut self.walls);
        for (i, wp) in walls.iter_mut().enumerate() {
            ctx.make_current(wp.gl_dc);
            wp.theme.destroy(&ctx.gl);
            let (w, h) = (wp.phys_w as f32 / wp.dpr, wp.phys_h as f32 / wp.dpr);
            wp.theme = self.make_theme(&ctx.gl, w, h, wp.buf_w, wp.buf_h, wp.dpr.min(0.72), i);
            wp.last_icons = None;
            wp.last_frame = now;
        }
        self.walls = walls;
        self.ctx = Some(ctx);
        PENDING_THEME_SYNC.with(|c| c.set(true));
    }

    fn poll_icons(&mut self) {
        let Some((rects, visible)) = self.reader.read() else { return };
        let now = self.now();
        let ctx = self.ctx.as_ref().unwrap();
        for wp in &mut self.walls {
            let s = wp.screen;
            let mine: Vec<RECT> = rects
                .iter()
                .filter(|r| {
                    let (cx, cy) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
                    cx >= s.left && cx < s.right && cy >= s.top && cy < s.bottom
                })
                .copied()
                .collect();
            let mine = if visible { mine } else { Vec::new() };
            if wp.last_icons.as_ref().is_some_and(|l| rects_eq(l, &mine)) {
                continue;
            }
            let first = wp.last_icons.is_none();
            let k = wp.dpr;
            let boxes: Vec<IconBox> = mine
                .iter()
                .map(|r| IconBox {
                    x: (r.left - s.left) as f32 / k,
                    y: (r.top - s.top) as f32 / k,
                    w: (r.right - r.left) as f32 / k,
                    h: (r.bottom - r.top) as f32 / k,
                })
                .collect();
            ctx.make_current(wp.gl_dc);
            wp.theme.set_icons(&ctx.gl, boxes, first, now);
            log!("Biểu tượng thay đổi (màn hình [{},{}]): {} mục, trường {:.1} ms", s.left, s.top, mine.len(), wp.theme.field_ms());
            wp.last_icons = Some(mine);
            if wp.paused {
                wp.needs_settle = true;
            }
        }
    }

    fn poll_foreground(&mut self) {
        let own: Vec<HWND> = self.walls.iter().map(|w| w.hwnd).collect();
        for wp in &mut self.walls {
            let p = desktop::covered_by_fullscreen(wp.screen, &own);
            if p != wp.paused {
                wp.paused = p;
                log!("Màn hình [{},{}] {}", wp.screen.left, wp.screen.top, if p { "tạm dừng (bị che)" } else { "tiếp tục phát" });
            }
        }
    }

    fn render(&mut self) {
        let now = self.now();
        let t0 = Instant::now();
        let ctx = self.ctx.as_ref().unwrap();
        let shared = self.shared.as_mut().unwrap();
        let gl = &ctx.gl;
        let mut drew = false;
        for wp in &mut self.walls {
            let settle = wp.paused && wp.needs_settle;
            if wp.paused && !settle {
                wp.last_frame = 0.0;
                continue;
            }
            ctx.make_current(wp.gl_dc);
            // Chế độ gdi: hiện khung đã đọc về ở lượt trước (GPU xong từ lâu, map không phải chờ)
            Self::present_gdi(gl, wp);
            unsafe {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(wp.fbo));
                gl.viewport(0, 0, wp.buf_w, wp.buf_h);
            }
            if settle {
                wp.theme.settle(gl, shared, now);
                wp.needs_settle = false;
            } else {
                let dt = if wp.last_frame > 0.0 { ((now - wp.last_frame) / 1000.0).min(0.1) as f32 } else { 0.03 };
                wp.last_frame = now;
                wp.theme.frame(gl, shared, dt, now);
            }
            if let Some((out_fbo, _, pbo)) = wp.out {
                unsafe {
                    // GPU phóng to (LINEAR, như trình duyệt phóng canvas) rồi đọc về PBO không chặn
                    gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(wp.fbo));
                    gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(out_fbo));
                    gl.blit_framebuffer(0, 0, wp.buf_w, wp.buf_h, 0, 0, wp.phys_w, wp.phys_h, glow::COLOR_BUFFER_BIT, glow::LINEAR);
                    gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(out_fbo));
                    gl.bind_buffer(glow::PIXEL_PACK_BUFFER, Some(pbo));
                    gl.pixel_store_i32(glow::PACK_ALIGNMENT, 4);
                    gl.read_pixels(0, 0, wp.phys_w, wp.phys_h, glow::BGRA, glow::UNSIGNED_BYTE, glow::PixelPackData::BufferOffset(0));
                    gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
                    gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                    gl.flush();
                }
                wp.pbo_ready = true;
            } else {
                unsafe {
                    gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(wp.fbo));
                    gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
                    gl.blit_framebuffer(0, 0, wp.buf_w, wp.buf_h, 0, 0, wp.phys_w, wp.phys_h, glow::COLOR_BUFFER_BIT, glow::LINEAR);
                    SwapBuffers(wp.hdc);
                }
            }
            drew = true;
        }
        if drew {
            self.frames += 1;
            self.frame_cpu_ms += t0.elapsed().as_secs_f64() * 1000.0;
        }
    }

    /// Đọc lại bộ đệm vẽ của từng màn hình từ GPU và lưu BMP (kiểm tra shader không cần chụp màn hình).
    fn snapshot(&self, dir: &str) {
        for i in 0..self.walls.len() {
            let path = format!("{dir}\\{}-screen{i}.bmp", self.cfg.theme);
            match std::fs::write(&path, self.read_bmp(i)) {
                Ok(_) => log!("Đã lưu ảnh kiểm tra {path}"),
                Err(e) => log!("Lưu ảnh kiểm tra lỗi: {e}"),
            }
        }
    }

    /// Đọc khung hình của màn hình `i` từ GPU thành file BMP 32-bit
    /// (độ phân giải thật nếu có FBO phóng to, không thì độ phân giải bộ đệm vẽ).
    fn read_bmp(&self, i: usize) -> Vec<u8> {
        let ctx = self.ctx.as_ref().unwrap();
        let gl = &ctx.gl;
        let wp = &self.walls[i];
        ctx.make_current(wp.gl_dc);
        let (fbo, w, h) = match wp.out {
            Some((f, _, _)) => (f, wp.phys_w as usize, wp.phys_h as usize),
            None => (wp.fbo, wp.buf_w as usize, wp.buf_h as usize),
        };
        let mut px = vec![0u8; w * h * 4];
        unsafe {
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(fbo));
            gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            gl.read_pixels(0, 0, w as i32, h as i32, glow::BGRA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(&mut px));
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        // BMP 32-bit, hàng dưới lên trên (trùng thứ tự của OpenGL); kênh alpha đặt 255
        for p in px.chunks_exact_mut(4) {
            p[3] = 255;
        }
        let mut f = Vec::with_capacity(54 + px.len());
        let size = (54 + px.len()) as u32;
        f.extend_from_slice(b"BM");
        f.extend_from_slice(&size.to_le_bytes());
        f.extend_from_slice(&[0, 0, 0, 0, 54, 0, 0, 0, 40, 0, 0, 0]);
        f.extend_from_slice(&(w as i32).to_le_bytes());
        f.extend_from_slice(&(h as i32).to_le_bytes());
        f.extend_from_slice(&[1, 0, 32, 0, 0, 0, 0, 0]);
        f.extend_from_slice(&(px.len() as u32).to_le_bytes());
        f.extend_from_slice(&[0u8; 16]);
        f.extend_from_slice(&px);
        f
    }

    /// Chụp cảnh màn hình chính rồi đặt làm ảnh màn khóa Windows (chạy nền).
    fn sync_lock_image(&self) {
        if self.walls.is_empty() {
            return;
        }
        let path = lock::data_dir().join("lockscreen.bmp");
        match std::fs::write(&path, self.read_bmp(0)) {
            Ok(_) => lock::sync_lock_image(&path),
            Err(e) => log!("Lưu ảnh màn khóa lỗi: {e}"),
        }
    }

    fn fix_z_order(&self) {
        for wp in &self.walls {
            if let Some(a) = &wp.attach {
                desktop::ensure_z_order(wp.hwnd, a);
            }
        }
    }
}

fn rects_eq(a: &[RECT], b: &[RECT]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.left == y.left && x.top == y.top && x.right == y.right && x.bottom == y.bottom)
}

fn private_mb() -> (f64, f64) {
    unsafe {
        let mut c: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        c.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        GetProcessMemoryInfo(GetCurrentProcess(), &mut c as *mut _ as *mut PROCESS_MEMORY_COUNTERS, c.cb);
        (c.PrivateUsage as f64 / 1048576.0, c.WorkingSetSize as f64 / 1048576.0)
    }
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // `--settings`: mở cửa sổ cài đặt của bản đang chạy (nếu có), không thì khởi động rồi mở
    let force_win32 = args.iter().any(|a| a == "--settings-win32");
    web::DEBUG_PORT.store(args.iter().any(|a| a == "--webview-debug"), std::sync::atomic::Ordering::Relaxed);
    let open_settings = force_win32 || args.iter().any(|a| a == "--settings");
    if open_settings {
        let cls = wide(MSG_CLASS);
        unsafe {
            let h = FindWindowW(cls.as_ptr(), std::ptr::null());
            if !h.is_null() {
                PostMessageW(h, WM_OPEN_SETTINGS, 0, 0);
                return;
            }
        }
    }
    if args.iter().any(|a| a == "--quit") {
        let cls = wide(MSG_CLASS);
        unsafe {
            let h = FindWindowW(cls.as_ptr(), std::ptr::null());
            if !h.is_null() {
                PostMessageW(h, WM_CLOSE, 0, 0);
            }
        }
        return;
    }

    // Giao thức trình bảo vệ màn hình Windows: "/s" chạy, "/p <hwnd>" xem trước, "/c" hoặc không tham số
    // (khi tên file là .scr) = cấu hình. `--saver` để chạy thử chế độ /s từ file .exe.
    let is_scr = std::env::current_exe().ok().and_then(|p| p.extension().map(|e| e.eq_ignore_ascii_case("scr"))).unwrap_or(false);
    let first = args.get(1).map(|a| a.to_ascii_lowercase()).unwrap_or_default();
    let saver = first.starts_with("/s") || first.starts_with("-s") || first == "--saver";
    if is_scr && !saver {
        if first.starts_with("/c") || first.starts_with("-c") || first.is_empty() {
            unsafe {
                MessageBoxW(
                    std::ptr::null_mut(),
                    wide("Bật / tắt trình bảo vệ màn hình Deskland trong menu khay của Deskland.\n\nKhi quay lại, Windows sẽ hiện màn hình đăng nhập thật (khóa bằng mật khẩu / PIN / Windows Hello của Windows).").as_ptr(),
                    wide("Deskland").as_ptr(),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
        }
        return; // "/p": không vẽ khung xem trước nhỏ
    }

    let log_name = if saver { "deskland-saver.log" } else { "deskland-rs.log" };
    let log_dir = if is_scr { Some(lock::data_dir()) } else { std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) };
    let log_path = log_dir.map(|d| d.join(log_name)).unwrap_or_else(|| log_name.into());
    *LOG.lock().unwrap() = File::create(&log_path).ok();
    std::panic::set_hook(Box::new(|info| log!("PANIC: {info}")));

    unsafe {
        let name = wide(if saver { "DesklandRs_Saver" } else { "DesklandRs_Singleton" });
        CreateMutexW(std::ptr::null(), 1, name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        timeBeginPeriod(1);
    }

    let fps: f64 = arg_value(&args, "--fps").and_then(|s| s.parse().ok()).unwrap_or(20.0);
    let interval = 1000.0 / fps.max(1.0);
    let cfg = load_config(arg_value(&args, "--theme"));
    log!(
        "deskland-rs {} khởi động{}: chủ đề {}, {fps} khung/giây",
        env!("CARGO_PKG_VERSION"),
        if saver { " (trình bảo vệ màn hình)" } else { "" },
        cfg.theme
    );
    CURRENT_THEME.with(|c| c.set(theme_cmd(&cfg.theme)));
    let lock_sync = cfg.lock_sync;
    LOCK_SYNC.with(|c| c.set(lock_sync));

    register_class(MSG_CLASS, Some(msg_proc), false);
    register_class(WALL_CLASS, Some(wall_proc), true);
    register_class(SAVER_CLASS, Some(saver_proc), true);
    let msg_hwnd = unsafe {
        let cls = wide(MSG_CLASS);
        CreateWindowExW(0, cls.as_ptr(), cls.as_ptr(), 0, 0, 0, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), GetModuleHandleW(std::ptr::null()), std::ptr::null())
    };

    // Khay hệ thống (không có ở chế độ trình bảo vệ màn hình)
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = msg_hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };
    for (i, c) in "Deskland (Rust native)".encode_utf16().enumerate() {
        nid.szTip[i] = c;
    }
    if !saver {
        unsafe {
            Shell_NotifyIconW(NIM_ADD, &nid);
            // Báo khi khóa / mở khóa phiên để ngừng vẽ lúc không ai thấy desktop
            windows_sys::Win32::System::RemoteDesktop::WTSRegisterSessionNotification(msg_hwnd, 0);
            // Phím tắt riêng (RegisterHotKey, không hook bàn phím; Win+L để nguyên cho Windows)
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
            let ok = RegisterHotKey(msg_hwnd, HOTKEY_SAVER, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, b'L' as u32) != 0;
            HOTKEY_OK.with(|c| c.set(ok));
            log!("Phím tắt Ctrl+Alt+L (khóa với cảnh động): {}", if ok { "đã đăng ký" } else { "đã bị ứng dụng khác dùng" });
        }
        if lock::winl_disabled() {
            // Bản C# cũ để lại DisableLockWorkstation=1: Win+L của Windows đang bị tắt. Chỉ cảnh báo,
            // việc bật lại do người dùng bấm trong menu khay.
            log!("CẢNH BÁO: DisableLockWorkstation=1 - Win+L của Windows đang bị tắt (do bản C# cũ)");
            let mut b = nid;
            b.uFlags = NIF_INFO;
            b.dwInfoFlags = NIIF_WARNING;
            let title = "Win+L của Windows đang bị tắt";
            let text = "Bản Deskland C# cũ đã tắt khóa Windows. Chuột phải icon Deskland > \"Bật lại Win+L của Windows\".";
            for (i, c) in title.encode_utf16().take(63).enumerate() {
                b.szInfoTitle[i] = c;
            }
            for (i, c) in text.encode_utf16().take(255).enumerate() {
                b.szInfo[i] = c;
            }
            unsafe { Shell_NotifyIconW(NIM_MODIFY, &b) };
        }
    }

    let t_start = Instant::now();
    let mut app = App {
        ctx: None,
        shared: None,
        vao: None,
        walls: Vec::new(),
        reader: desktop::IconReader::new(),
        cfg,
        start: Instant::now(),
        frames: 0,
        frame_cpu_ms: 0.0,
        gdi: arg_value(&args, "--present").as_deref() != Some("gl"),
        dummy: None,
        saver,
    };
    log!("Cách hiển thị: {}", if app.gdi { "gdi (FBO -> đọc về -> StretchDIBits)" } else { "gl (SwapBuffers trực tiếp)" });
    if let Err(e) = app.create_walls() {
        log!("Khởi động thất bại: {e}");
        app.destroy_walls();
        unsafe { Shell_NotifyIconW(NIM_DELETE, &nid) };
        return;
    }
    app.poll_icons();
    app.render();
    if open_settings && !saver {
        PENDING_CMD.with(|c| c.set(CMD_SETTINGS));
    }
    log!("Khung hình đầu tiên sau {:.0} ms kể từ khi khởi động", t_start.elapsed().as_secs_f64() * 1000.0);
    if saver {
        unsafe {
            ShowCursor(0);
            if let Some(w) = app.walls.first() {
                SetForegroundWindow(w.hwnd);
            }
        }
        SAVER_START.with(|c| c.set(Some(Instant::now())));
    }
    // Đồng bộ ảnh màn khóa Windows vài giây sau khi cảnh đã ổn định
    let mut lock_sync_at = (lock_sync && !saver).then(|| app.now() + 6_000.0);
    let mut saver_check_at: Option<f64> = None;
    let mut rebuild_at: Option<f64> = None;
    let mut web_changed = false;
    let mut last_icons_json = String::new();
    // Ngừng vẽ khi không ai thấy desktop: phiên khóa / màn Winlogon / trình bảo vệ màn hình đang chạy
    let mut hidden = false;

    let hook = unsafe {
        let progman = FindWindowW(wide("Progman").as_ptr(), std::ptr::null());
        let mut pid = 0u32;
        GetWindowThreadProcessId(progman, &mut pid);
        SetWinEventHook(0x8000, 0x8004, std::ptr::null_mut(), Some(win_event), pid, 0, 0)
    };

    let mut next_frame = app.now() + interval;
    let mut next_icons = app.now() + 400.0;
    let mut next_fg = app.now();
    let mut next_stat = app.now() + 10_000.0;
    let snapshot_dir = arg_value(&args, "--snapshot");
    let mut snapshot_at = snapshot_dir.as_ref().map(|_| app.now() + 4_000.0);
    'outer: loop {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    break 'outer;
                }
                // Phím Tab / Enter / mũi tên trong cửa sổ cài đặt
                if let Some(sh) = settings::hwnd() {
                    if IsDialogMessageW(sh, &msg) != 0 {
                        continue;
                    }
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        let cmd = PENDING_CMD.with(|c| c.replace(0));
        let actions = settings::take_actions();
        let changed = cmd != 0 || !actions.is_empty();
        for a in actions {
            use settings::Action;
            match a {
                Action::Theme(t) => {
                    app.switch_theme(t);
                    CURRENT_THEME.with(|c| c.set(theme_cmd(t)));
                }
                Action::Param(k, v) => {
                    app.ensure_seed();
                    app.theme_params_mut().insert(k.into(), serde_json::json!(v));
                    log!("Thông số {}.{k} = {v}", app.cfg.theme);
                    app.rebuild_theme();
                }
                Action::Reseed => {
                    app.reseed();
                    log!("Đổi cảnh (seed mới)");
                    app.rebuild_theme();
                }
                Action::Reset => {
                    app.ensure_seed();
                    let m = app.theme_params_mut();
                    let seed = m.get("seed").cloned();
                    m.clear();
                    if let Some(sd) = seed {
                        m.insert("seed".into(), sd);
                    }
                    log!("Đặt lại thông số mặc định cho {}", app.cfg.theme);
                    app.rebuild_theme();
                }
                Action::Lock => lock::lock_now(),
                Action::SaverNow => PENDING_CMD.with(|c| c.set(CMD_SAVER_NOW)),
                Action::Saver(on) => {
                    if !on {
                        lock::uninstall_saver();
                    } else if let Err(e) = lock::install_saver(app.cfg.lock_idle_mins) {
                        log!("Đặt trình bảo vệ màn hình lỗi: {e}");
                        unsafe { MessageBoxW(settings::hwnd().unwrap_or(msg_hwnd), wide(&e).as_ptr(), wide("Deskland").as_ptr(), MB_OK | MB_ICONWARNING) };
                    }
                }
                Action::Idle(m) => {
                    app.cfg.lock_idle_mins = m;
                    if lock::saver_installed() {
                        if let Err(e) = lock::install_saver(m) {
                            log!("Đổi thời gian trình bảo vệ màn hình lỗi: {e}");
                        }
                    }
                }
                Action::Sync(on) => {
                    LOCK_SYNC.with(|c| c.set(on));
                    log!("Đồng bộ ảnh màn khóa Windows: {}", if on { "bật" } else { "tắt" });
                    if on {
                        app.sync_lock_image();
                    }
                }
                Action::Autostart(on) => settings::set_autostart(on),
            }
        }
        if cmd == CMD_SETTINGS {
            // Giống bản gốc: trang settings.html trong WebView2. Không được thì dùng cửa sổ Win32 dự phòng.
            let r = if force_win32 { Err("--settings-win32".to_string()) } else { web::find_prototype().ok_or_else(|| "không thấy thư mục prototype (settings.html)".to_string()).and_then(|p| web::open(&p)) };
            if let Err(e) = r {
                log!("Không mở được cài đặt WebView2 ({e}) -> dùng cửa sổ Win32");
                settings::open(&app.settings_state());
            }
        }
        // Thông điệp từ trang cài đặt WebView2
        for raw in web::take_inbox() {
            let Ok(m) = serde_json::from_str::<serde_json::Value>(&raw) else { continue };
            let s_ = |k: &str| m.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            match s_("t").as_str() {
                "hello" => {
                    web::post(&web_init(&app).to_string());
                    last_icons_json.clear();
                }
                "set" => {
                    let (theme, key) = (s_("theme"), s_("key"));
                    let val = m.get("value").cloned().unwrap_or(serde_json::Value::Null);
                    if theme.is_empty() || key.is_empty() {
                        continue;
                    }
                    if !app.cfg.params.is_object() {
                        app.cfg.params = serde_json::json!({});
                    }
                    let root = app.cfg.params.as_object_mut().unwrap();
                    let e = root.entry(theme.clone()).or_insert_with(|| serde_json::json!({}));
                    if !e.is_object() {
                        *e = serde_json::json!({});
                    }
                    e.as_object_mut().unwrap().insert(key, val);
                    if theme == app.cfg.theme {
                        // Trang gửi liên tục khi kéo thanh trượt: gom lại, dựng cảnh sau khi ngừng kéo 250 ms
                        rebuild_at = Some(app.now() + 250.0);
                    }
                    web_changed = true;
                }
                "reset" => {
                    let theme = s_("theme");
                    if let Some(root) = app.cfg.params.as_object_mut() {
                        let seed = root.get(&theme).and_then(|p| p.get("seed")).cloned();
                        root.insert(theme.clone(), seed.map(|sd| serde_json::json!({ "seed": sd })).unwrap_or(serde_json::json!({})));
                    }
                    if theme == app.cfg.theme {
                        rebuild_at = Some(app.now());
                    }
                    log!("Đặt lại thông số mặc định cho {theme}");
                    web_changed = true;
                }
                "theme" => {
                    let id = s_("id");
                    if let Some(tid) = canonical_theme(&id) {
                        app.switch_theme(tid);
                        CURRENT_THEME.with(|c| c.set(theme_cmd(tid)));
                    } else {
                        log!("Chủ đề '{id}' chưa hỗ trợ");
                        web::post(&serde_json::json!({ "t": "themeChanged", "id": app.cfg.theme }).to_string());
                    }
                    web_changed = true;
                }
                "lockNow" => lock::lock_now(),
                "saverNow" => PENDING_CMD.with(|c| c.set(CMD_SAVER_NOW)),
                "lockCfg" => {
                    if let Some(on) = m.get("syncWin").and_then(|v| v.as_bool()) {
                        if on != LOCK_SYNC.with(|c| c.get()) {
                            LOCK_SYNC.with(|c| c.set(on));
                            log!("Đồng bộ ảnh màn khóa Windows: {}", if on { "bật" } else { "tắt" });
                            if on {
                                app.sync_lock_image();
                            }
                        }
                    }
                    if let Some(mins) = m.get("idleMins").and_then(|v| v.as_u64()).map(|v| v as u32) {
                        let installed = lock::saver_installed();
                        if mins == 0 && installed {
                            lock::uninstall_saver();
                        } else if mins > 0 && (!installed || mins != app.cfg.lock_idle_mins) {
                            app.cfg.lock_idle_mins = mins;
                            if let Err(e) = lock::install_saver(mins) {
                                log!("Đặt trình bảo vệ màn hình lỗi: {e}");
                                unsafe { MessageBoxW(msg_hwnd, wide(&e).as_ptr(), wide("Deskland").as_ptr(), MB_OK | MB_ICONWARNING) };
                                web::post(&web_init(&app).to_string());
                            }
                        }
                    }
                    web_changed = true;
                }
                "autostart" => {
                    settings::set_autostart(m.get("v").and_then(|v| v.as_bool()).unwrap_or(false));
                    web::post(&serde_json::json!({ "t": "autostart", "v": settings::autostart_enabled() }).to_string());
                }
                "lang" => {
                    app.cfg.lang = s_("v");
                    let (eff, sys) = lang_eff(&app.cfg.lang);
                    web::post(&serde_json::json!({ "t": "lang", "v": app.cfg.lang, "eff": eff, "sys": sys }).to_string());
                    web_changed = true;
                }
                other => log!("Bỏ qua thông điệp cài đặt '{other}' (bản Rust chưa hỗ trợ)"),
            }
        }
        if rebuild_at.is_some_and(|t| app.now() >= t) {
            rebuild_at = None;
            app.ensure_seed();
            app.rebuild_theme();
        }
        if web_changed {
            web_changed = false;
            save_config(&app.cfg);
            settings::sync(&app.settings_state());
        }
        if let Some(tid) = cmd_theme(cmd) {
            app.switch_theme(tid);
            CURRENT_THEME.with(|c| c.set(cmd));
        } else {
            match cmd {
                CMD_EXIT => break,
                CMD_LOCK => lock::lock_now(),
                CMD_SAVER => {
                    if lock::saver_installed() {
                        lock::uninstall_saver();
                    } else if let Err(e) = lock::install_saver(app.cfg.lock_idle_mins) {
                        log!("Đặt trình bảo vệ màn hình lỗi: {e}");
                        unsafe { MessageBoxW(msg_hwnd, wide(&e).as_ptr(), wide("Deskland").as_ptr(), MB_OK | MB_ICONWARNING) };
                    }
                }
                CMD_SYNC => {
                    let on = !LOCK_SYNC.with(|c| c.get());
                    LOCK_SYNC.with(|c| c.set(on));
                    log!("Đồng bộ ảnh màn khóa Windows: {}", if on { "bật" } else { "tắt" });
                    if on {
                        app.sync_lock_image();
                    }
                }
                CMD_WINL => {
                    lock::restore_winl();
                }
                CMD_SAVER_NOW => {
                    let mut ready = lock::saver_installed() && lock::saver_secure();
                    if !ready {
                        let q = "\"Khóa với cảnh động\" dùng trình bảo vệ màn hình của Windows, có bật \"khi quay lại, hiện màn hình đăng nhập\".\n\nĐặt Deskland làm trình bảo vệ màn hình (có khóa) ngay bây giờ?";
                        let r = unsafe { MessageBoxW(msg_hwnd, wide(q).as_ptr(), wide("Deskland").as_ptr(), MB_YESNO | MB_ICONQUESTION) };
                        if r == IDYES {
                            match lock::install_saver(app.cfg.lock_idle_mins) {
                                Ok(()) => ready = true,
                                Err(e) => {
                                    log!("Đặt trình bảo vệ màn hình lỗi: {e}");
                                    unsafe { MessageBoxW(msg_hwnd, wide(&e).as_ptr(), wide("Deskland").as_ptr(), MB_OK | MB_ICONWARNING) };
                                }
                            }
                        }
                    }
                    if ready {
                        lock::start_secure_saver(msg_hwnd);
                        saver_check_at = Some(app.now() + 2_500.0);
                    }
                }
                _ => {}
            }
        }
        if changed && cmd != CMD_EXIT {
            save_config(&app.cfg);
            settings::sync(&app.settings_state());
            if web::is_open() && cmd != CMD_SETTINGS && cmd != 0 {
                // Đổi từ menu khay / phím tắt: báo trang cài đặt
                web::post(&serde_json::json!({ "t": "themeChanged", "id": app.cfg.theme }).to_string());
            }
        }
        // Nếu Windows không bật được trình bảo vệ màn hình thì vẫn khóa thật như người dùng muốn
        if saver_check_at.is_some_and(|t| app.now() >= t) {
            saver_check_at = None;
            if !lock::saver_running() && !SESSION_LOCKED.with(|c| c.get()) {
                log!("Trình bảo vệ màn hình không chạy sau 2,5 giây -> khóa bằng LockWorkStation()");
                lock::lock_now();
            }
        }
        if matches!(PENDING_THEME_SYNC.with(|c| c.replace(false)), true) && LOCK_SYNC.with(|c| c.get()) && !saver {
            lock_sync_at = Some(app.now() + 3_000.0);
        }
        if lock_sync_at.is_some_and(|t| app.now() >= t) {
            lock_sync_at = None;
            if LOCK_SYNC.with(|c| c.get()) {
                app.sync_lock_image();
            }
        }
        if DISPLAY_CHANGED.with(|c| c.replace(false)) {
            log!("Cấu hình màn hình thay đổi, dựng lại cửa sổ hình nền");
            app.destroy_walls();
            if let Err(e) = app.create_walls() {
                log!("Dựng lại thất bại: {e}");
                break;
            }
        }
        if REPAINT.with(|c| c.replace(false)) {
            app.repaint_all();
        }
        if ZORDER_DIRTY.with(|c| c.replace(false)) {
            app.fix_z_order();
        }
        let now = app.now();
        if now >= next_icons && !hidden {
            app.fix_z_order();
            app.poll_icons();
            if web::is_open() {
                let j = icons_json(&app);
                if j != last_icons_json {
                    web::post(&j);
                    last_icons_json = j;
                }
            }
            next_icons = now + 400.0;
        }
        if now >= next_fg {
            if !saver {
                app.poll_foreground();
                let h = SESSION_LOCKED.with(|c| c.get()) || lock::saver_running() || !lock::input_desktop_is_default();
                if h != hidden {
                    hidden = h;
                    log!("{}", if h { "Desktop không hiển thị (khóa / trình bảo vệ màn hình) -> ngừng vẽ" } else { "Desktop hiển thị lại -> vẽ tiếp" });
                    if !h {
                        REPAINT.with(|c| c.set(true));
                    }
                }
            }
            next_fg = now + 500.0;
        }
        if hidden {
            // Chỉ thức dậy theo nhịp kiểm tra 500 ms, không vẽ, không đọc biểu tượng
            next_frame = next_fg;
            next_icons = next_fg;
        } else if now >= next_frame {
            app.render();
            next_frame += interval;
            if next_frame < now {
                next_frame = now + interval;
            }
        }
        if snapshot_at.is_some_and(|t| now >= t) {
            app.snapshot(snapshot_dir.as_deref().unwrap());
            snapshot_at = None;
        }
        if now >= next_stat {
            let (pm, ws) = private_mb();
            let n = app.frames.max(1) as f64;
            log!("RAM riêng {pm:.1} MB, working set {ws:.1} MB | {:.1} khung/giây, CPU vẽ {:.2} ms/khung", app.frames as f64 / 10.0, app.frame_cpu_ms / n);
            app.frames = 0;
            app.frame_cpu_ms = 0.0;
            next_stat = now + 10_000.0;
        }
        let wait = (next_frame.min(next_icons).min(next_fg) - app.now()).max(0.0) as u32;
        unsafe { MsgWaitForMultipleObjects(0, std::ptr::null(), 0, wait, QS_ALLINPUT) };
    }

    log!("Đang thoát");
    unsafe { UnhookWinEvent(hook) };
    app.destroy_walls();
    app.drop_shared();
    app.ctx = None;
    unsafe {
        if let Some((h, dc)) = app.dummy.take() {
            ReleaseDC(h, dc);
            DestroyWindow(h);
        }
        Shell_NotifyIconW(NIM_DELETE, &nid);
        DestroyWindow(msg_hwnd);
    }
}
