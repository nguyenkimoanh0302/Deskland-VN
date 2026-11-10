//! Màn hình khóa - CHỈ dùng cơ chế khóa thật của Windows.
//!
//! Nguyên tắc (khác hẳn bản C#):
//! - KHÔNG hook bàn phím, KHÔNG chặn Win+L, KHÔNG ghi `DisableLockWorkstation`.
//! - KHÔNG tự kiểm mật khẩu / PIN, KHÔNG lưu bất kỳ mã nào.
//! - Khóa = `LockWorkStation()`: Windows chuyển sang desktop an toàn (Winlogon), mở khóa bằng
//!   mật khẩu / PIN / Windows Hello của chính Windows.
//! - Cảnh động khi máy rảnh = trình bảo vệ màn hình Windows (`.scr`) với `ScreenSaverIsSecure=1`:
//!   Windows tự khóa phiên khi người dùng quay lại; Deskland chỉ vẽ, không quyết định khóa / mở.
//! - Ảnh màn khóa: chụp cảnh hiện tại, đặt qua API WinRT `LockScreen.SetImageFileAsync`.

use crate::desktop::wide;
use std::os::windows::process::CommandExt;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::System::Shutdown::LockWorkStation;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const POLICY_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Policies\System";
const DESKTOP_KEY: &str = r"Control Panel\Desktop";
const SAVER_POLICY_KEY: &str = r"Software\Policies\Microsoft\Windows\Control Panel\Desktop";

/// Thư mục dữ liệu của bản Rust (ngoài AppData để tránh ảo hoá MSIX khi chạy từ ứng dụng đóng gói).
pub fn data_dir() -> std::path::PathBuf {
    let base = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    let d = std::path::Path::new(&base).join(".deskland-rs");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn lock_now() {
    crate::log!("Khóa máy bằng LockWorkStation() (khóa thật của Windows)");
    if unsafe { LockWorkStation() } == 0 {
        crate::log!("LockWorkStation thất bại: lỗi {}", unsafe { GetLastError() });
    }
}

fn reg_get_dword(root: HKEY, key: &str, name: &str) -> Option<u32> {
    let (k, n) = (wide(key), wide(name));
    let mut v = 0u32;
    let mut sz = 4u32;
    let r = unsafe { RegGetValueW(root, k.as_ptr(), n.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), &mut v as *mut u32 as *mut _, &mut sz) };
    (r == ERROR_SUCCESS).then_some(v)
}

fn reg_get_string(root: HKEY, key: &str, name: &str) -> Option<String> {
    let (k, n) = (wide(key), wide(name));
    let mut buf = vec![0u16; 1024];
    let mut sz = (buf.len() * 2) as u32;
    let r = unsafe { RegGetValueW(root, k.as_ptr(), n.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr() as *mut _, &mut sz) };
    if r != ERROR_SUCCESS {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
    Some(String::from_utf16_lossy(&buf[..len]))
}

fn reg_set_string(root: HKEY, key: &str, name: &str, val: &str) -> bool {
    let (k, n, v) = (wide(key), wide(name), wide(val));
    let r = unsafe { RegSetKeyValueW(root, k.as_ptr(), n.as_ptr(), REG_SZ, v.as_ptr() as *const _, (v.len() * 2) as u32) };
    r == ERROR_SUCCESS
}

/// Bản C# cũ ghi `DisableLockWorkstation=1` để tự xử lý Win+L - việc này TẮT khóa thật của Windows.
pub fn winl_disabled() -> bool {
    reg_get_dword(HKEY_CURRENT_USER, POLICY_KEY, "DisableLockWorkstation").is_some_and(|v| v != 0)
}

/// Xoá giá trị `DisableLockWorkstation` (chỉ khi người dùng bấm trong menu khay).
pub fn restore_winl() -> bool {
    let (k, n) = (wide(POLICY_KEY), wide("DisableLockWorkstation"));
    let r = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr()) };
    let ok = r == ERROR_SUCCESS || r == ERROR_FILE_NOT_FOUND;
    crate::log!("Khôi phục Win+L của Windows: {}", if ok { "xong" } else { "thất bại" });
    ok
}

fn saver_path() -> std::path::PathBuf {
    data_dir().join("Deskland.scr")
}

/// Trình bảo vệ màn hình hiện tại có phải của Deskland Rust không.
pub fn saver_installed() -> bool {
    let cur = reg_get_string(HKEY_CURRENT_USER, DESKTOP_KEY, "SCRNSAVE.EXE").unwrap_or_default();
    !cur.is_empty() && cur.eq_ignore_ascii_case(&saver_path().to_string_lossy())
}

/// Đặt Deskland làm trình bảo vệ màn hình, BẮT BUỘC kèm "khi tiếp tục, hiện màn hình đăng nhập".
pub fn install_saver(idle_mins: u32) -> Result<(), String> {
    if let Some(p) = reg_get_string(HKEY_CURRENT_USER, SAVER_POLICY_KEY, "SCRNSAVE.EXE") {
        return Err(format!("Chính sách nhóm đang ép trình bảo vệ màn hình '{p}', không đổi được"));
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dst = saver_path();
    std::fs::copy(&exe, &dst).map_err(|e| format!("Chép {} lỗi: {e}", dst.display()))?;
    let secs = (idle_mins.max(1) * 60).to_string();
    let ok = reg_set_string(HKEY_CURRENT_USER, DESKTOP_KEY, "SCRNSAVE.EXE", &dst.to_string_lossy())
        && reg_set_string(HKEY_CURRENT_USER, DESKTOP_KEY, "ScreenSaveActive", "1")
        && reg_set_string(HKEY_CURRENT_USER, DESKTOP_KEY, "ScreenSaveTimeOut", &secs)
        && reg_set_string(HKEY_CURRENT_USER, DESKTOP_KEY, "ScreenSaverIsSecure", "1");
    if !ok {
        return Err("Ghi cài đặt trình bảo vệ màn hình thất bại".into());
    }
    unsafe {
        SystemParametersInfoW(SPI_SETSCREENSAVETIMEOUT, idle_mins.max(1) * 60, std::ptr::null_mut(), SPIF_SENDCHANGE);
        SystemParametersInfoW(SPI_SETSCREENSAVEACTIVE, 1, std::ptr::null_mut(), SPIF_SENDCHANGE);
    }
    crate::log!("Đã đặt trình bảo vệ màn hình {} sau {idle_mins} phút, có khóa khi quay lại", dst.display());
    Ok(())
}

/// Trình bảo vệ màn hình hiện tại có bật "khi tiếp tục, hiện màn hình đăng nhập" không.
pub fn saver_secure() -> bool {
    reg_get_string(HKEY_CURRENT_USER, DESKTOP_KEY, "ScreenSaverIsSecure").is_some_and(|v| v.trim() == "1")
}

/// Windows có đang chạy trình bảo vệ màn hình (bất kỳ) không.
pub fn saver_running() -> bool {
    let mut on: BOOL = 0;
    unsafe { SystemParametersInfoW(SPI_GETSCREENSAVERRUNNING, 0, &mut on as *mut BOOL as *mut _, 0) };
    on != 0
}

/// Desktop nhận bàn phím có phải desktop thường ("Default") không. Khi khóa máy, UAC hay
/// Ctrl+Alt+Del, input desktop là "Winlogon" (mở không được hoặc tên khác) -> desktop không ai thấy.
pub fn input_desktop_is_default() -> bool {
    use windows_sys::Win32::System::StationsAndDesktops::*;
    unsafe {
        let d = OpenInputDesktop(0, 0, DESKTOP_READOBJECTS);
        if d.is_null() {
            return false;
        }
        let mut buf = [0u16; 64];
        let mut need = 0u32;
        let ok = GetUserObjectInformationW(d as HANDLE, UOI_NAME, buf.as_mut_ptr() as *mut _, (buf.len() * 2) as u32, &mut need) != 0;
        CloseDesktop(d);
        let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
        !ok || String::from_utf16_lossy(&buf[..len]).eq_ignore_ascii_case("Default")
    }
}

/// "Khóa với cảnh động": nhờ CHÍNH Windows bật trình bảo vệ màn hình đã cài (SC_SCREENSAVE).
/// Vì `ScreenSaverIsSecure=1`, Windows coi đây là trạng thái an toàn và tự hiện màn đăng nhập khi
/// có thao tác. Deskland không tự chạy file .scr (như vậy Windows không biết để khóa).
pub fn start_secure_saver(hwnd: HWND) {
    crate::log!("Nhờ Windows bật trình bảo vệ màn hình (SC_SCREENSAVE)");
    unsafe {
        SetForegroundWindow(hwnd);
        SendMessageW(hwnd, WM_SYSCOMMAND, 0xF140usize /* SC_SCREENSAVE */, 0);
    }
}

/// Gỡ: chỉ bỏ SCRNSAVE.EXE nếu đang trỏ vào Deskland (giữ nguyên ScreenSaverIsSecure).
pub fn uninstall_saver() {
    if saver_installed() {
        reg_set_string(HKEY_CURRENT_USER, DESKTOP_KEY, "SCRNSAVE.EXE", "");
        unsafe { SystemParametersInfoW(SPI_SETSCREENSAVEACTIVE, 0, std::ptr::null_mut(), SPIF_SENDCHANGE) };
        crate::log!("Đã gỡ trình bảo vệ màn hình Deskland");
    }
}

/// Đặt ảnh màn khóa Windows từ ảnh BMP (chạy nền, không chặn vòng vẽ).
/// Chuyển BMP -> JPG rồi gọi WinRT `Windows.System.UserProfile.LockScreen.SetImageFileAsync`.
pub fn sync_lock_image(bmp: &std::path::Path) {
    let jpg = bmp.with_extension("jpg");
    let esc = |p: &std::path::Path| p.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.Drawing; \
         $b=[System.Drawing.Bitmap]::FromFile('{bmp}'); $b.Save('{jpg}',[System.Drawing.Imaging.ImageFormat]::Jpeg); $b.Dispose(); Remove-Item -LiteralPath '{bmp}'; \
         Add-Type -AssemblyName System.Runtime.WindowsRuntime; \
         $g=([System.WindowsRuntimeSystemExtensions].GetMethods()|?{{$_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'}})[0]; \
         $a=([System.WindowsRuntimeSystemExtensions].GetMethods()|?{{$_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction'}})[0]; \
         [Windows.Storage.StorageFile,Windows.Storage,ContentType=WindowsRuntime]|Out-Null; \
         [Windows.System.UserProfile.LockScreen,Windows.System.UserProfile,ContentType=WindowsRuntime]|Out-Null; \
         $f=$g.MakeGenericMethod([Windows.Storage.StorageFile]).Invoke($null,@([Windows.Storage.StorageFile]::GetFileFromPathAsync('{jpg}'))).GetAwaiter().GetResult(); \
         $a.Invoke($null,@([Windows.System.UserProfile.LockScreen]::SetImageFileAsync($f))).GetAwaiter().GetResult()",
        bmp = esc(bmp),
        jpg = esc(&jpg)
    );
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    match std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_) => crate::log!("Đang đặt ảnh màn khóa Windows: {}", jpg.display()),
        Err(e) => crate::log!("Không chạy được PowerShell để đặt ảnh màn khóa: {e}"),
    }
}
