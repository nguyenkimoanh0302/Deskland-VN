//! Cửa sổ Cài đặt - dùng control gốc của Windows (không WebView2) để giữ RAM thấp.
//! Cửa sổ chỉ ghi lại thao tác (`Action`); vòng lặp chính áp dụng, lưu cài đặt rồi gọi `sync` để
//! cập nhật lại giao diện theo trạng thái thật.

use crate::desktop::wide;
use std::cell::RefCell;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLASS: &str = "DesklandRsSettings";

pub enum Action {
    Theme(&'static str),
    Param(&'static str, f64),
    Reseed,
    Reset,
    Lock,
    SaverNow,
    Saver(bool),
    Idle(u32),
    Sync(bool),
    Autostart(bool),
    Interaction(bool),
}

/// Trạng thái thật mà giao diện hiển thị.
pub struct State {
    pub theme: String,
    /// Thông số của chủ đề hiện tại (`params.<theme>`)
    pub params: serde_json::Value,
    pub saver: bool,
    pub idle: u32,
    pub sync: bool,
    pub autostart: bool,
    pub hotkey: bool,
    pub interaction: bool,
}

pub struct ParamDef {
    pub key: &'static str,
    label: &'static str,
    min: f64,
    max: f64,
    step: f64,
    pub def: f64,
    unit: &'static str,
    kind: Fmt,
}

#[derive(Clone, Copy)]
enum Fmt {
    Num,
    Pct,
    Mul,
}

const fn p(key: &'static str, label: &'static str, min: f64, max: f64, step: f64, def: f64, unit: &'static str, kind: Fmt) -> ParamDef {
    ParamDef { key, label, min, max, step, def, unit, kind }
}

// Khoảng giá trị giống bảng điều khiển của bản web (prototype/themes.json)
pub static BEACH: [ParamDef; 6] = [
    p("sigma", "Khoảng cách bờ", 40.0, 200.0, 1.0, 85.0, "px", Fmt::Num),
    p("warp", "Độ uốn bờ biển", 0.0, 80.0, 1.0, 34.0, "", Fmt::Num),
    p("shoal", "Bãi nông trong", 0.0, 300.0, 1.0, 110.0, "px", Fmt::Num),
    p("shallow", "Độ rộng vùng nông", 120.0, 700.0, 1.0, 260.0, "px", Fmt::Num),
    p("kayaks", "Thuyền kayak", 0.0, 4.0, 1.0, 2.0, "chiếc", Fmt::Num),
    p("birds", "Hải âu", 0.0, 8.0, 1.0, 3.0, "con", Fmt::Num),
];
pub static MALDIVES: [ParamDef; 8] = [
    p("sigma", "Bán kính bãi cát", 40.0, 200.0, 1.0, 85.0, "px", Fmt::Num),
    p("warp", "Độ uốn bờ cát", 0.0, 80.0, 1.0, 34.0, "", Fmt::Num),
    p("shoal", "Độ rộng bãi cạn", 0.0, 300.0, 1.0, 120.0, "px", Fmt::Num),
    p("shallow", "Chuyển sắc xanh ngọc", 120.0, 700.0, 1.0, 280.0, "px", Fmt::Num),
    p("fishSchools", "Đàn cá", 0.0, 6.0, 1.0, 3.0, "đàn", Fmt::Num),
    p("turtles", "Rùa biển", 0.0, 4.0, 1.0, 2.0, "con", Fmt::Num),
    p("kayaks", "Thuyền kayak", 0.0, 4.0, 1.0, 1.0, "chiếc", Fmt::Num),
    p("birds", "Chim biển", 0.0, 8.0, 1.0, 3.0, "con", Fmt::Num),
];
pub static MEADOW: [ParamDef; 3] = [
    p("sigma", "Độ lớn đám mây", 40.0, 200.0, 1.0, 72.0, "px", Fmt::Num),
    p("cover", "Mây trên đồng cỏ", 0.0, 1.0, 0.01, 0.3, "", Fmt::Pct),
    p("flow", "Tốc độ trôi", 0.0, 3.0, 0.05, 1.0, "", Fmt::Mul),
];
pub static LAKE: [ParamDef; 6] = [
    p("sigma", "Phạm vi bờ cỏ", 40.0, 200.0, 1.0, 82.0, "px", Fmt::Num),
    p("warp", "Độ uốn bờ hồ", 0.0, 80.0, 1.0, 32.0, "", Fmt::Num),
    p("shoal", "Bãi sỏi nước nông", 20.0, 240.0, 1.0, 95.0, "px", Fmt::Num),
    p("depth", "Độ sâu mặt hồ", 100.0, 600.0, 1.0, 240.0, "px", Fmt::Num),
    p("flow", "Gợn sóng mặt hồ", 0.0, 3.0, 0.05, 1.0, "", Fmt::Mul),
    p("clouds", "Mây phản chiếu", 0.0, 8.0, 1.0, 4.0, "đám", Fmt::Num),
];
pub static SKY: [ParamDef; 8] = [
    p("time", "Thời điểm (0-3)", 0.0, 3.0, 1.0, 0.0, "", Fmt::Num),
    p("sigma", "Kích thước đám mây", 40.0, 200.0, 1.0, 74.0, "px", Fmt::Num),
    p("cover", "Mây trôi tự do", 0.0, 1.0, 0.01, 0.35, "", Fmt::Pct),
    p("flow", "Tốc độ gió mây", 0.0, 3.0, 0.05, 1.0, "", Fmt::Mul),
    p("birds", "Đàn chim bay", 0.0, 8.0, 1.0, 3.0, "con", Fmt::Num),
    p("plane", "Máy bay nhỏ (0-2)", 0.0, 2.0, 1.0, 1.0, "", Fmt::Num),
    p("moon", "Mặt trăng (0-1)", 0.0, 1.0, 1.0, 1.0, "", Fmt::Num),
    p("stars", "Mật độ sao đêm", 0.0, 1.0, 0.05, 0.6, "", Fmt::Pct),
];
pub static OCEAN: [ParamDef; 6] = [
    p("sigma", "Kích thước đám mây", 40.0, 200.0, 1.0, 72.0, "px", Fmt::Num),
    p("cover", "Mây trên biển", 0.0, 1.0, 0.01, 0.3, "", Fmt::Pct),
    p("flow", "Tốc độ gió biển", 0.0, 3.0, 0.05, 1.0, "", Fmt::Mul),
    p("boats", "Thuyền buồm", 0.0, 6.0, 1.0, 2.0, "chiếc", Fmt::Num),
    p("yachts", "Du thuyền", 0.0, 4.0, 1.0, 1.0, "chiếc", Fmt::Num),
    p("birds", "Chim biển", 0.0, 8.0, 1.0, 3.0, "con", Fmt::Num),
];
pub static RAINFOREST: [ParamDef; 7] = [
    p("sigma", "Độ rộng trảng rêu", 40.0, 200.0, 1.0, 80.0, "px", Fmt::Num),
    p("warp", "Độ uốn bìa rừng", 0.0, 80.0, 1.0, 32.0, "", Fmt::Num),
    p("rain", "Mưa & Gợn nước", 0.0, 1.0, 0.05, 0.65, "", Fmt::Pct),
    p("mist", "Sương mù rừng mưa", 0.0, 1.0, 0.05, 0.4, "", Fmt::Pct),
    p("wind", "Gió rừng", 0.0, 3.0, 0.05, 1.0, "", Fmt::Mul),
    p("birds", "Chim nhiệt đới", 0.0, 8.0, 1.0, 3.0, "con", Fmt::Num),
    p("fireflies", "Đom đóm rừng", 0.0, 15.0, 1.0, 6.0, "con", Fmt::Num),
];
pub static MOCCHAU: [ParamDef; 7] = [
    p("sigma", "Độ rộng đồi cỏ", 40.0, 200.0, 1.0, 80.0, "px", Fmt::Num),
    p("warp", "Độ uốn sườn đồi", 0.0, 80.0, 1.0, 30.0, "", Fmt::Num),
    p("bloom", "Độ dày hoa mận", 0.2, 1.0, 0.05, 0.8, "", Fmt::Pct),
    p("petals", "Cánh hoa bay", 0.0, 1.0, 0.05, 0.55, "", Fmt::Pct),
    p("wind", "Gió cao nguyên", 0.0, 3.0, 0.05, 1.0, "", Fmt::Mul),
    p("mist", "Sương sớm", 0.0, 1.0, 0.05, 0.35, "", Fmt::Pct),
    p("birds", "Chim rừng", 0.0, 8.0, 1.0, 3.0, "con", Fmt::Num),
];
pub static TET_SAKURA: [ParamDef; 6] = [
    p("sigma", "Độ rộng sân xuân", 40.0, 200.0, 1.0, 80.0, "px", Fmt::Num),
    p("warp", "Độ uốn cành hoa", 0.0, 80.0, 1.0, 30.0, "", Fmt::Num),
    p("petals", "Cánh đào bay", 0.0, 1.0, 0.05, 0.65, "", Fmt::Pct),
    p("wind", "Gió xuân", 0.0, 3.0, 0.05, 1.0, "", Fmt::Mul),
    p("lanterns", "Đèn lồng & Lì xì", 0.0, 8.0, 1.0, 4.0, "cái", Fmt::Num),
    p("swallows", "Chim én mùa xuân", 0.0, 8.0, 1.0, 3.0, "con", Fmt::Num),
];
pub static CAT_HILL: [ParamDef; 4] = [
    p("cats", "Số lượng mèo", 3.0, 15.0, 1.0, 8.0, "bé", Fmt::Num),
    p("butterflies", "Bươm bướm vàng", 0.0, 6.0, 1.0, 2.0, "con", Fmt::Num),
    p("leaves", "Mật độ lá bay", 0.0, 1.0, 0.05, 0.65, "", Fmt::Pct),
    p("wind", "Sức gió trên đồi", 0.0, 3.0, 0.05, 1.0, "", Fmt::Mul),
];
pub static STADIUM: [ParamDef; 5] = [
    p("players", "Cầu thủ mỗi đội", 3.0, 11.0, 1.0, 6.0, "người", Fmt::Num),
    p("speed", "Nhịp độ trận đấu", 0.4, 2.5, 0.05, 1.0, "", Fmt::Mul),
    p("passFreq", "Tần suất chuyền bóng", 0.4, 2.5, 0.05, 1.0, "", Fmt::Mul),
    p("lighting", "Ánh sáng sân (0-2)", 0.0, 2.0, 1.0, 0.0, "", Fmt::Num),
    p("stripes", "Độ đậm sọc cỏ", 0.0, 1.0, 0.05, 0.65, "", Fmt::Pct),
];
pub static COMMUTE: [ParamDef; 7] = [
    p("time", "Giờ trong ngày (0 theo máy, 1 ngày, 2 chiều, 3 đêm)", 0.0, 3.0, 1.0, 0.0, "", Fmt::Num),
    p("busFreq", "Xe buýt qua lại", 0.2, 2.5, 0.05, 1.0, "", Fmt::Mul),
    p("wind", "Gió", 0.0, 2.0, 0.05, 1.0, "", Fmt::Mul),
    p("reeds", "Lau sậy", 0.0, 1.0, 0.05, 0.7, "", Fmt::Pct),
    p("lights", "Đèn phố ban đêm", 0.0, 1.0, 0.05, 1.0, "", Fmt::Pct),
    p("sigma", "Độ rộng đồi cỏ / mây", 40.0, 200.0, 1.0, 70.0, "px", Fmt::Num),
    p("warp", "Độ uốn mép", 0.0, 80.0, 1.0, 30.0, "", Fmt::Num),
];
pub static AQUARIUM: [ParamDef; 7] = [
    p("fish", "Số cá vàng", 1.0, 8.0, 1.0, 4.0, "con", Fmt::Num),
    p("eager", "Cá háu ăn", 0.4, 2.5, 0.05, 1.0, "", Fmt::Mul),
    p("plants", "Rong rêu", 0.0, 1.0, 0.05, 0.6, "", Fmt::Pct),
    p("bubbles", "Bong bóng", 0.0, 1.0, 0.05, 0.5, "", Fmt::Pct),
    p("light", "Tia nắng", 0.0, 1.0, 0.05, 0.6, "", Fmt::Pct),
    p("sigma", "Độ rộng rạn cát", 40.0, 200.0, 1.0, 80.0, "px", Fmt::Num),
    p("warp", "Độ uốn rạn cát", 0.0, 80.0, 1.0, 30.0, "", Fmt::Num),
];
pub static SUMMER_PORCH: [ParamDef; 6] = [
    p("corgis", "Số lượng Corgi", 1.0, 4.0, 1.0, 3.0, "bé", Fmt::Num),
    p("fanSpeed", "Tốc độ quạt bàn", 0.0, 2.0, 0.05, 1.0, "", Fmt::Mul),
    p("timeOfDay", "Thời điểm (0-3)", 0.0, 3.0, 1.0, 0.0, "", Fmt::Num),
    p("sunbeams", "Nắng & đom đóm", 0.0, 1.0, 0.05, 0.7, "", Fmt::Pct),
    p("sigma", "Độ rộng luống vườn", 45.0, 140.0, 1.0, 78.0, "px", Fmt::Num),
    p("warp", "Độ uốn bờ vườn", 0.0, 70.0, 1.0, 28.0, "", Fmt::Num),
];
// Mèo con trên đồi (bản web: prototype/cats.html)
pub static CATS: [ParamDef; 8] = [
    p("mound", "Độ rộng gò cỏ", 10.0, 80.0, 1.0, 28.0, "px", Fmt::Num),
    p("cats", "Số chú mèo", 1.0, 14.0, 1.0, 8.0, "con", Fmt::Num),
    p("speed", "Tốc độ đi", 0.3, 2.0, 0.05, 1.0, "", Fmt::Mul),
    p("naps", "Độ lười", 0.0, 1.0, 0.01, 0.35, "", Fmt::Pct),
    p("sun", "Nắng ấm", 0.0, 1.0, 0.01, 0.6, "", Fmt::Pct),
    p("flowers", "Hoa dại", 0.0, 1.0, 0.01, 0.6, "", Fmt::Pct),
    p("bushes", "Cây & lùm cây", 0.0, 10.0, 1.0, 4.0, "cây", Fmt::Num),
    p("wind", "Sức gió", 0.0, 2.0, 0.05, 1.0, "", Fmt::Mul),
];
const IDLE_CHOICES: [u32; 8] = [1, 2, 3, 5, 10, 15, 30, 60];

fn defs(theme: &str) -> &'static [ParamDef] {
    match theme {
        "meadow" => &MEADOW,
        "beach-turquoise" => &MALDIVES,
        "lake" => &LAKE,
        "sky" => &SKY,
        "ocean" => &OCEAN,
        "rainforest" => &RAINFOREST,
        "mocchau" => &MOCCHAU,
        "tet-sakura" => &TET_SAKURA,
        "cat-hill" => &CAT_HILL,
        "stadium" => &STADIUM,
        "aquarium" => &AQUARIUM,
        "commute" => &COMMUTE,
        "summer-porch" => &SUMMER_PORCH,
        "cats" => &CATS,
        _ => &BEACH,
    }
}

fn has_adaptive(theme: &str) -> bool {
    matches!(theme, "meadow" | "sky" | "ocean" | "tet-sakura")
}

fn fmt(d: &ParamDef, v: f64) -> String {
    match d.kind {
        Fmt::Pct => format!("{}%", (v * 100.0).round()),
        Fmt::Mul => format!("×{v:.2}"),
        Fmt::Num if d.unit.is_empty() => format!("{}", v.round()),
        Fmt::Num => format!("{} {}", v.round(), d.unit),
    }
}

const ID_THEME: usize = 1001;
const ID_RESEED: usize = 1003;
const ID_RESET: usize = 1004;
const ID_ADAPTIVE: usize = 1150;
const ID_LOCK: usize = 1301;
const ID_SAVER_NOW: usize = 1302;
const ID_SAVER: usize = 1303;
const ID_IDLE: usize = 1304;
const ID_SYNC: usize = 1305;
const ID_AUTO: usize = 1306;
const ID_INTERACTION: usize = 1307;
const ID_ROW: usize = 1100;

const TBM_GETPOS_: u32 = WM_USER;
const TBM_SETPOS_: u32 = WM_USER + 5;
const TBM_SETRANGEMIN_: u32 = WM_USER + 7;
const TBM_SETRANGEMAX_: u32 = WM_USER + 8;
const TBM_SETPAGESIZE_: u32 = WM_USER + 21;
const TB_ENDTRACK_: u32 = 8;
const TBS_NOTICKS_: u32 = 0x10;

struct Row {
    def: &'static ParamDef,
    label: HWND,
    bar: HWND,
    value: HWND,
}

struct Ui {
    hwnd: HWND,
    font: HFONT,
    bold: HFONT,
    s: f32,
    theme: String,
    rows: Vec<Row>,
    adaptive: HWND,
    theme_combo: HWND,
    saver_now: HWND,
    saver: HWND,
    idle: HWND,
    sync: HWND,
    auto: HWND,
    interaction: HWND,
    note: HWND,
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
    static ACTIONS: RefCell<Vec<Action>> = const { RefCell::new(Vec::new()) };
}

pub fn take_actions() -> Vec<Action> {
    ACTIONS.with(|a| std::mem::take(&mut *a.borrow_mut()))
}

fn push(a: Action) {
    ACTIONS.with(|v| v.borrow_mut().push(a));
}

/// HWND cửa sổ cài đặt (để vòng lặp chính gọi IsDialogMessage cho phím Tab).
pub fn hwnd() -> Option<HWND> {
    UI.with(|u| u.try_borrow().ok().and_then(|g| g.as_ref().map(|u| u.hwnd)))
}

impl Ui {
    fn ctl(&self, class: &str, text: &str, style: u32, x: i32, y: i32, w: i32, h: i32, id: usize, bold: bool) -> HWND {
        let s = self.s;
        unsafe {
            let c = CreateWindowExW(
                0,
                wide(class).as_ptr(),
                wide(text).as_ptr(),
                WS_CHILD | WS_VISIBLE | style,
                (x as f32 * s) as i32,
                (y as f32 * s) as i32,
                (w as f32 * s) as i32,
                (h as f32 * s) as i32,
                self.hwnd,
                id as HMENU,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            SendMessageW(c, WM_SETFONT, if bold { self.bold } else { self.font } as WPARAM, 1);
            c
        }
    }

    fn build_rows(&mut self, theme: &str) {
        unsafe {
            for r in self.rows.drain(..) {
                DestroyWindow(r.label);
                DestroyWindow(r.bar);
                DestroyWindow(r.value);
            }
            if !self.adaptive.is_null() {
                DestroyWindow(self.adaptive);
                self.adaptive = std::ptr::null_mut();
            }
        }
        self.theme = theme.into();
        let mut y = 124;
        for (i, d) in defs(theme).iter().enumerate() {
            let label = self.ctl("STATIC", d.label, 0u32 /* SS_LEFT */, 20, y + 4, 170, 22, 0, false);
            let bar = self.ctl("msctls_trackbar32", "", WS_TABSTOP | TBS_NOTICKS_, 192, y, 250, 30, ID_ROW + i, false);
            let value = self.ctl("STATIC", "", 2u32 /* SS_RIGHT */, 446, y + 4, 74, 22, 0, false);
            let steps = ((d.max - d.min) / d.step).round() as isize;
            unsafe {
                SendMessageW(bar, TBM_SETRANGEMIN_, 0, 0);
                SendMessageW(bar, TBM_SETRANGEMAX_, 1, steps);
                SendMessageW(bar, TBM_SETPAGESIZE_, 0, (steps / 10).max(1));
            }
            self.rows.push(Row { def: d, label, bar, value });
            y += 36;
        }
        if has_adaptive(theme) {
            self.adaptive = self.ctl("BUTTON", "Ôm quanh biểu tượng (thích ứng)", BS_AUTOCHECKBOX as u32 | WS_TABSTOP, 20, y + 2, 400, 24, ID_ADAPTIVE, false);
        }
    }
}

unsafe fn checked(h: HWND) -> bool {
    SendMessageW(h, BM_GETCHECK, 0, 0) == 1
}

unsafe fn set_check(h: HWND, on: bool) {
    if !h.is_null() {
        SendMessageW(h, BM_SETCHECK, on as usize, 0);
    }
}

fn row_value(d: &ParamDef, pos: isize) -> f64 {
    let v = d.min + pos as f64 * d.step;
    (v * 1000.0).round() / 1000.0
}

/// Mở (hoặc đưa lên trước) cửa sổ cài đặt.
pub fn open(state: &State) {
    if let Some(h) = hwnd() {
        unsafe {
            ShowWindow(h, SW_SHOWNORMAL);
            SetForegroundWindow(h);
        }
        return;
    }
    unsafe {
        let icc = INITCOMMONCONTROLSEX { dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32, dwICC: ICC_BAR_CLASSES | ICC_STANDARD_CLASSES };
        InitCommonControlsEx(&icc);
        let cls = wide(CLASS);
        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(proc_);
        wc.hInstance = GetModuleHandleW(std::ptr::null());
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        wc.hIcon = crate::app_icon(GetSystemMetrics(SM_CXICON), GetSystemMetrics(SM_CYICON));
        wc.hIconSm = crate::app_icon(GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON));
        wc.hbrBackground = (COLOR_WINDOW + 1) as usize as HBRUSH;
        wc.lpszClassName = cls.as_ptr();
        RegisterClassExW(&wc); // lần sau trả lỗi "đã có" - không sao

        let dpi = GetDpiForSystem().max(96);
        let s = dpi as f32 / 96.0;
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        let mut r = RECT { left: 0, top: 0, right: (540.0 * s) as i32, bottom: (736.0 * s) as i32 };
        AdjustWindowRectEx(&mut r, style, 0, 0);
        let (w, h) = (r.right - r.left, r.bottom - r.top);
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let hwnd = CreateWindowExW(
            0,
            cls.as_ptr(),
            wide("Deskland-VN - Cài đặt").as_ptr(),
            style,
            (sw - w) / 2,
            (sh - h) / 2,
            w,
            h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        );
        if hwnd.is_null() {
            crate::log!("Không tạo được cửa sổ cài đặt: lỗi {}", GetLastError());
            return;
        }
        // Phông chữ giao diện của Windows (Segoe UI) theo DPI hệ thống
        let mut ncm: NONCLIENTMETRICSW = std::mem::zeroed();
        ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
        SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, ncm.cbSize, &mut ncm as *mut _ as *mut _, 0);
        let font = CreateFontIndirectW(&ncm.lfMessageFont);
        let mut lb = ncm.lfMessageFont;
        lb.lfWeight = 600;
        lb.lfHeight = (lb.lfHeight as f32 * 1.12) as i32;
        let bold = CreateFontIndirectW(&lb);

        let mut ui = Ui {
            hwnd,
            font,
            bold,
            s,
            theme: String::new(),
            rows: Vec::new(),
            adaptive: std::ptr::null_mut(),
            theme_combo: std::ptr::null_mut(),
            interaction: std::ptr::null_mut(),
            saver_now: std::ptr::null_mut(),
            saver: std::ptr::null_mut(),
            idle: std::ptr::null_mut(),
            sync: std::ptr::null_mut(),
            auto: std::ptr::null_mut(),
            note: std::ptr::null_mut(),
        };
        let btn = BS_PUSHBUTTON as u32 | WS_TABSTOP;
        let chk = BS_AUTOCHECKBOX as u32 | WS_TABSTOP;
        ui.ctl("STATIC", "Chủ đề", 0, 20, 16, 300, 24, 0, true);
        ui.theme_combo = ui.ctl("COMBOBOX", "", CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP, 20, 44, 240, 320, ID_THEME, false);
        for &(_, label, _) in crate::THEMES {
            SendMessageW(ui.theme_combo, CB_ADDSTRING, 0, wide(label).as_ptr() as LPARAM);
        }
        ui.ctl("BUTTON", "Đổi cảnh", btn | WS_GROUP, 380, 44, 140, 30, ID_RESEED, false);
        ui.ctl("STATIC", "Thông số chủ đề", 0, 20, 90, 300, 24, 0, true);
        ui.ctl("BUTTON", "Đặt lại mặc định", btn, 380, 86, 140, 30, ID_RESET, false);
        ui.build_rows(&state.theme);

        let y = 124 + 8 * 36 + 32; // dưới hàng thông số dài nhất (8 hàng + adaptive)
        ui.ctl("STATIC", "Màn hình khóa - chỉ dùng khóa thật của Windows", 0, 20, y, 500, 24, 0, true);
        ui.ctl("BUTTON", "Khóa máy ngay", btn, 20, y + 32, 160, 30, ID_LOCK, false);
        ui.saver_now = ui.ctl("BUTTON", "Khóa với cảnh động", btn, 190, y + 32, 240, 30, ID_SAVER_NOW, false);
        ui.saver = ui.ctl("BUTTON", "Cảnh động khi máy rảnh, khóa khi quay lại, sau", chk, 20, y + 74, 360, 26, ID_SAVER, false);
        ui.idle = ui.ctl("COMBOBOX", "", CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP, 384, y + 72, 110, 300, ID_IDLE, false);
        for m in IDLE_CHOICES {
            SendMessageW(ui.idle, CB_ADDSTRING, 0, wide(&format!("{m} phút")).as_ptr() as LPARAM);
        }
        ui.sync = ui.ctl("BUTTON", "Dùng cảnh làm ảnh màn khóa Windows", chk, 20, y + 106, 400, 26, ID_SYNC, false);
        ui.note = ui.ctl(
            "STATIC",
            "Win+L và nút Khóa của Windows vẫn ra màn khóa ảnh tĩnh: Windows không cho ứng dụng vẽ lên màn khóa thật. Cảnh động chạy trước khi khóa, có thao tác thì Windows hiện màn đăng nhập.",
            0,
            20,
            y + 138,
            500,
            54,
            0,
            false,
        );
        let y2 = y + 202;
        ui.ctl("STATIC", "Chung", 0, 20, y2, 300, 24, 0, true);
        ui.auto = ui.ctl("BUTTON", "Khởi động cùng Windows", chk, 20, y2 + 30, 400, 26, ID_AUTO, false);
        ui.interaction = ui.ctl("BUTTON", "Tương tác chuột trên màn hình nền", chk, 20, y2 + 60, 400, 26, ID_INTERACTION, false);
        UI.with(|u| *u.borrow_mut() = Some(ui));
        sync(state);
        ShowWindow(hwnd, SW_SHOWNORMAL);
        SetForegroundWindow(hwnd);
        crate::log!("Mở cửa sổ cài đặt");
    }
}

/// Cập nhật giao diện theo trạng thái thật (gọi sau mỗi lần vòng lặp chính áp dụng thay đổi).
pub fn sync(state: &State) {
    UI.with(|u| {
        let mut b = u.borrow_mut();
        let Some(ui) = b.as_mut() else { return };
        if ui.theme != state.theme {
            ui.build_rows(&state.theme);
        }
        unsafe {
            let tidx = crate::THEMES.iter().position(|&(tid, _, _)| tid == state.theme).unwrap_or(1);
            SendMessageW(ui.theme_combo, CB_SETCURSEL, tidx, 0);
            for r in &ui.rows {
                let v = state.params.get(r.def.key).and_then(|x| x.as_f64()).unwrap_or(r.def.def).clamp(r.def.min, r.def.max);
                let pos = ((v - r.def.min) / r.def.step).round() as isize;
                SendMessageW(r.bar, TBM_SETPOS_, 1, pos);
                SetWindowTextW(r.value, wide(&fmt(r.def, v)).as_ptr());
            }
            set_check(ui.adaptive, state.params.get("adaptive").and_then(|x| x.as_f64()).unwrap_or(1.0) != 0.0);
            set_check(ui.saver, state.saver);
            let idx = IDLE_CHOICES.iter().enumerate().min_by_key(|(_, &m)| (m as i64 - state.idle as i64).abs()).map(|(i, _)| i).unwrap_or(3);
            SendMessageW(ui.idle, CB_SETCURSEL, idx, 0);
            set_check(ui.sync, state.sync);
            set_check(ui.interaction, state.interaction);
            set_check(ui.auto, state.autostart);
            let t = if state.hotkey { "Khóa với cảnh động (Ctrl+Alt+L)" } else { "Khóa với cảnh động" };
            SetWindowTextW(ui.saver_now, wide(t).as_ptr());
        }
    });
}

unsafe extern "system" fn proc_(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        WM_COMMAND => {
            let (id, code) = (w & 0xFFFF, (w >> 16) & 0xFFFF);
            let ctl = l as HWND;
            match (id, code as u32) {
                (ID_THEME, CBN_SELCHANGE) => {
                    let i = SendMessageW(ctl, CB_GETCURSEL, 0, 0);
                    if let Some(&(tid, _, _)) = crate::THEMES.get(i as usize) {
                        push(Action::Theme(tid));
                    }
                }
                (ID_RESEED, BN_CLICKED) => push(Action::Reseed),
                (ID_RESET, BN_CLICKED) => push(Action::Reset),
                (ID_ADAPTIVE, BN_CLICKED) => push(Action::Param("adaptive", if checked(ctl) { 1.0 } else { 0.0 })),
                (ID_LOCK, BN_CLICKED) => push(Action::Lock),
                (ID_SAVER_NOW, BN_CLICKED) => push(Action::SaverNow),
                (ID_SAVER, BN_CLICKED) => push(Action::Saver(checked(ctl))),
                (ID_SYNC, BN_CLICKED) => push(Action::Sync(checked(ctl))),
                (ID_INTERACTION, BN_CLICKED) => push(Action::Interaction(checked(ctl))),
                (ID_AUTO, BN_CLICKED) => push(Action::Autostart(checked(ctl))),
                (ID_IDLE, CBN_SELCHANGE) => {
                    let i = SendMessageW(ctl, CB_GETCURSEL, 0, 0);
                    if let Some(&m) = IDLE_CHOICES.get(i as usize) {
                        push(Action::Idle(m));
                    }
                }
                _ => {}
            }
            0
        }
        WM_HSCROLL => {
            let bar = l as HWND;
            let end = (w & 0xFFFF) as u32 == TB_ENDTRACK_;
            // try_borrow: thông điệp có thể tới lúc `sync` đang giữ UI (SendMessage lồng nhau)
            UI.with(|u| {
                if let Some(ui) = u.try_borrow().ok().as_ref().and_then(|g| g.as_ref()) {
                    if let Some(r) = ui.rows.iter().find(|r| r.bar == bar) {
                        let v = row_value(r.def, SendMessageW(bar, TBM_GETPOS_, 0, 0));
                        SetWindowTextW(r.value, wide(&fmt(r.def, v)).as_ptr());
                        // Áp dụng khi thả chuột / phím: dựng lại cảnh tốn vài chục ms, không làm mỗi bước kéo
                        if end {
                            push(Action::Param(r.def.key, v));
                        }
                    }
                }
            });
            0
        }
        WM_CTLCOLORSTATIC => {
            let dc = w as HDC;
            SetBkColor(dc, GetSysColor(COLOR_WINDOW));
            let is_note = UI.with(|u| u.try_borrow().ok().is_some_and(|g| g.as_ref().is_some_and(|ui| ui.note == l as HWND)));
            if is_note {
                SetTextColor(dc, 0x00666666);
            }
            GetSysColorBrush(COLOR_WINDOW) as LRESULT
        }
        WM_CLOSE => {
            DestroyWindow(h);
            0
        }
        WM_DESTROY => {
            if let Some(ui) = UI.with(|u| u.borrow_mut().take()) {
                DeleteObject(ui.font as HGDIOBJ);
                DeleteObject(ui.bold as HGDIOBJ);
            }
            crate::log!("Đóng cửa sổ cài đặt");
            0
        }
        _ => DefWindowProcW(h, m, w, l),
    }
}

// ---------------------------------------------------------------- Khởi động cùng Windows
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_NAME: &str = "DesklandRs";

pub fn autostart_enabled() -> bool {
    crate::lock::reg_get_string(windows_sys::Win32::System::Registry::HKEY_CURRENT_USER, RUN_KEY, RUN_NAME).is_some()
}

pub fn set_autostart(on: bool) {
    use windows_sys::Win32::System::Registry::HKEY_CURRENT_USER;
    if on {
        let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        crate::lock::reg_set_string(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME, &format!("\"{exe}\""));
    } else {
        crate::lock::reg_delete_value(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME);
    }
    crate::log!("Khởi động cùng Windows: {}", if on { "bật" } else { "tắt" });
}
