//! Ngữ cảnh OpenGL 3.3 core qua WGL. MỘT ngữ cảnh dùng chung cho mọi cửa sổ hình nền
//! (mỗi màn hình chỉ cần wglMakeCurrent(hdc) rồi SwapBuffers), nên shader và texture nhiễu chỉ nạp 1 lần.

use std::ffi::{c_void, CString};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::Graphics::OpenGL::*;
use windows_sys::Win32::System::LibraryLoader::*;

pub struct GlContext {
    pub hglrc: HGLRC,
    pub gl: glow::Context,
}

pub fn set_pixel_format(hdc: HDC) -> bool {
    unsafe {
        let mut pfd: PIXELFORMATDESCRIPTOR = std::mem::zeroed();
        pfd.nSize = std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16;
        pfd.nVersion = 1;
        pfd.dwFlags = PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER | PFD_SUPPORT_COMPOSITION;
        pfd.iPixelType = PFD_TYPE_RGBA;
        pfd.cColorBits = 32;
        pfd.cAlphaBits = 8;
        pfd.iLayerType = PFD_MAIN_PLANE as u8;
        let pf = ChoosePixelFormat(hdc, &pfd);
        pf != 0 && SetPixelFormat(hdc, pf, &pfd) != 0
    }
}

type CreateCtxAttribs = unsafe extern "system" fn(HDC, HGLRC, *const i32) -> HGLRC;
type SwapInterval = unsafe extern "system" fn(i32) -> i32;

unsafe fn wgl_proc(name: &str) -> Option<unsafe extern "system" fn() -> isize> {
    let c = CString::new(name).unwrap();
    wglGetProcAddress(c.as_ptr() as *const u8)
}

impl GlContext {
    /// `hdc` phải thuộc một cửa sổ đã SetPixelFormat.
    pub fn new(hdc: HDC) -> Result<GlContext, String> {
        unsafe {
            let legacy = wglCreateContext(hdc);
            if legacy.is_null() {
                return Err(format!("wglCreateContext lỗi {}", GetLastError()));
            }
            wglMakeCurrent(hdc, legacy);
            let mut hglrc = legacy;
            if let Some(f) = wgl_proc("wglCreateContextAttribsARB") {
                let create: CreateCtxAttribs = std::mem::transmute(f);
                // WGL_CONTEXT_MAJOR/MINOR_VERSION_ARB = 3.3, WGL_CONTEXT_PROFILE_MASK_ARB = CORE
                let attribs = [0x2091, 3, 0x2092, 3, 0x9126, 0x1, 0];
                let core = create(hdc, std::ptr::null_mut(), attribs.as_ptr());
                if !core.is_null() {
                    wglMakeCurrent(hdc, core);
                    wglDeleteContext(legacy);
                    hglrc = core;
                } else {
                    crate::log!("Không tạo được ngữ cảnh 3.3 core, dùng ngữ cảnh mặc định");
                }
            }
            // Không chờ VSync trong SwapBuffers: tự giới hạn khung hình bằng bộ hẹn giờ,
            // tránh việc 2 màn hình chờ VSync nối tiếp nhau.
            if let Some(f) = wgl_proc("wglSwapIntervalEXT") {
                let si: SwapInterval = std::mem::transmute(f);
                si(0);
            }
            let opengl32 = LoadLibraryA(b"opengl32.dll\0".as_ptr());
            let gl = glow::Context::from_loader_function(|name| {
                let c = CString::new(name).unwrap();
                let p = wglGetProcAddress(c.as_ptr() as *const u8).map(|f| f as usize).unwrap_or(0);
                if p == 0 || p == 1 || p == 2 || p == 3 || p == usize::MAX {
                    GetProcAddress(opengl32, c.as_ptr() as *const u8).map(|f| f as usize).unwrap_or(0) as *const c_void
                } else {
                    p as *const c_void
                }
            });
            Ok(GlContext { hglrc, gl })
        }
    }

    pub fn make_current(&self, hdc: HDC) -> bool {
        unsafe { wglMakeCurrent(hdc, self.hglrc) != 0 }
    }
}

impl Drop for GlContext {
    fn drop(&mut self) {
        unsafe {
            wglMakeCurrent(std::ptr::null_mut(), std::ptr::null_mut());
            wglDeleteContext(self.hglrc);
        }
    }
}
