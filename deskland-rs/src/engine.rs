//! Phần dùng chung giữa các chủ đề: nhiễu CPU, trường khoảng cách quanh biểu tượng,
//! texture nhiễu lặp, và các tiện ích OpenGL (biên dịch shader, uniform).
//! Port 1:1 từ prototype/meadow.html + prototype/flat.html.

use glow::HasContext;
use std::collections::HashMap;

pub const STEP: f32 = 5.0;
pub const MORPH_MS: f64 = 1800.0;
pub const TASKBAR: f32 = 48.0;

// ---------------------------------------------------------------- Ngẫu nhiên (thay Math.random)
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }
    /// Số ngẫu nhiên trong [0, 1)
    pub fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32) / (1u32 << 24) as f32
    }
}

// ---------------------------------------------------------------- Nhiễu CPU (giống hệt bản JS)
#[derive(Clone, Copy)]
pub struct Noise {
    pub seed: i32,
}

impl Noise {
    pub fn hash2(&self, x: f64, y: f64, k: i32) -> f64 {
        let xi = x as i64 as i32 as u32;
        let yi = y as i64 as i32 as u32;
        let mut h = xi.wrapping_mul(374_761_393)
            ^ yi.wrapping_mul(668_265_263)
            ^ (self.seed.wrapping_add(k) as u32).wrapping_mul(2_246_822_519);
        h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
        (h ^ (h >> 16)) as f64 / 4_294_967_296.0
    }

    pub fn vnoise(&self, x: f64, y: f64, k: i32) -> f64 {
        let ix = x.floor();
        let iy = y.floor();
        let fx = x - ix;
        let fy = y - iy;
        let ux = fx * fx * (3.0 - 2.0 * fx);
        let uy = fy * fy * (3.0 - 2.0 * fy);
        let a = self.hash2(ix, iy, k);
        let b = self.hash2(ix + 1.0, iy, k);
        let c = self.hash2(ix, iy + 1.0, k);
        let d = self.hash2(ix + 1.0, iy + 1.0, k);
        a + (b - a) * ux + (c - a) * uy + (a - b - c + d) * ux * uy
    }

    pub fn fbm(&self, mut x: f64, mut y: f64, k: i32) -> f64 {
        let mut a = 0.5;
        let mut s = 0.0;
        for i in 0..4 {
            s += a * self.vnoise(x, y, k + i * 7);
            x = x * 2.03 + 17.1;
            y = y * 2.03 + 17.1;
            a *= 0.5;
        }
        s
    }

    fn tile_noise(&self, x: f64, y: f64, px: i64, py: i64, k: i32) -> f64 {
        let ix = x.floor() as i64;
        let iy = y.floor() as i64;
        let fx = x - ix as f64;
        let fy = y - iy as f64;
        let ux = fx * fx * (3.0 - 2.0 * fx);
        let uy = fy * fy * (3.0 - 2.0 * fy);
        let m = |v: i64, p: i64| ((v % p) + p) % p;
        let (x0, x1, y0, y1) = (m(ix, px), m(ix + 1, px), m(iy, py), m(iy + 1, py));
        let a = self.hash2(x0 as f64, y0 as f64, k);
        let b = self.hash2(x1 as f64, y0 as f64, k);
        let c = self.hash2(x0 as f64, y1 as f64, k);
        let d = self.hash2(x1 as f64, y1 as f64, k);
        a + (b - a) * ux + (c - a) * uy + (a - b - c + d) * ux * uy
    }

    fn tile_fbm(&self, u: f64, v: f64, periods: &[i64], k: i32) -> f64 {
        let (mut s, mut w, mut tw) = (0.0, 0.5, 0.0);
        for (i, &p) in periods.iter().enumerate() {
            s += w * self.tile_noise(u * p as f64, v * p as f64, p, p, k + i as i32 * 13);
            tw += w;
            w *= 0.5;
        }
        s / tw
    }

    /// Texture nhiễu lặp 256x256 RGBA8
    pub fn make_noise_image(&self) -> (i32, Vec<u8>) {
        const S: usize = 256;
        let mut d = vec![0u8; S * S * 4];
        let chans: [&[i64]; 4] = [&[8, 16, 32, 64], &[8, 16, 32, 64], &[8, 16, 32], &[16, 32, 64, 128]];
        for y in 0..S {
            for x in 0..S {
                let (u, v) = (x as f64 / S as f64, y as f64 / S as f64);
                let i = (y * S + x) * 4;
                for c in 0..4 {
                    let n = self.tile_fbm(u, v, chans[c], 100 + c as i32 * 50);
                    d[i + c] = (((n - 0.5) * 1.9 + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
                }
            }
        }
        (S as i32, d)
    }
}

pub fn sstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ---------------------------------------------------------------- Biểu tượng & trường khoảng cách
/// Khung biểu tượng theo toạ độ CSS (đã chia DPI) như bản JS nhận được.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct IconBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy)]
pub struct IconRect {
    pub cx: f32,
    pub cy: f32,
    pub hw: f32,
    pub hh: f32,
}

pub fn icon_rects(icons: &[IconBox]) -> Vec<IconRect> {
    icons
        .iter()
        .map(|ic| IconRect { cx: ic.x + ic.w / 2.0, cy: ic.y + ic.h / 2.0, hw: ic.w / 2.0 - 4.0, hh: ic.h / 2.0 - 2.0 })
        .collect()
}

pub struct Field {
    pub gw: usize,
    pub gh: usize,
    wx: Vec<f32>,
    wy: Vec<f32>,
}

impl Field {
    pub fn new(w: f32, h: f32, noise: &Noise) -> Self {
        let gw = (w / STEP).ceil() as usize + 2;
        let gh = (h / STEP).ceil() as usize + 2;
        let n = gw * gh;
        let (mut wx, mut wy) = (vec![0f32; n], vec![0f32; n]);
        for gy in 0..gh {
            for gx in 0..gw {
                let i = gy * gw + gx;
                let (x, y) = (gx as f64 * STEP as f64, gy as f64 * STEP as f64);
                wx[i] = (noise.fbm(x * 0.0028, y * 0.0028, 1) - 0.47) as f32;
                wy[i] = (noise.fbm(x * 0.0028, y * 0.0028, 2) - 0.47) as f32;
            }
        }
        Field { gw, gh, wx, wy }
    }

    /// s = khoảng cách có dấu (<0 là bên trong vùng biểu tượng)
    pub fn compute(&self, rects: &[IconRect], sigma: f32, warp: f32) -> Vec<f32> {
        let (gw, gh) = (self.gw, self.gh);
        let n = gw * gh;
        let mut f1 = vec![0f32; n];
        let is1 = 1.0 / (sigma * sigma);
        let wa = warp * 2.2;
        let wmax = wa * 0.5;
        for r in rects {
            let rr = 5.5 * sigma + r.hw.max(r.hh) * 0.5 + wmax;
            let gx0 = ((r.cx - rr) / STEP).floor().max(0.0) as usize;
            let gx1 = ((r.cx + rr) / STEP).ceil().min(gw as f32 - 1.0);
            let gy0 = ((r.cy - rr) / STEP).floor().max(0.0) as usize;
            let gy1 = ((r.cy + rr) / STEP).ceil().min(gh as f32 - 1.0);
            if gx1 < 0.0 || gy1 < 0.0 {
                continue;
            }
            let (gx1, gy1) = (gx1 as usize, gy1 as usize);
            let (chw, chh) = (r.hw * 0.5, r.hh * 0.5);
            for gy in gy0..=gy1 {
                for gx in gx0..=gx1 {
                    let i = gy * gw + gx;
                    let qx = gx as f32 * STEP + self.wx[i] * wa;
                    let qy = gy as f32 * STEP + self.wy[i] * wa;
                    let dx = ((qx - r.cx).abs() - chw).max(0.0);
                    let dy = ((qy - r.cy).abs() - chh).max(0.0);
                    f1[i] += (-(dx * dx + dy * dy) * is1).exp();
                }
            }
        }
        let c0 = (-(0.35f32).ln()).sqrt();
        let fmin = (-30f32).exp();
        f1.iter()
            .map(|&f| {
                let g = f.max(fmin).ln();
                sigma * ((-g).max(0.0).sqrt() - c0) - sigma * 0.6 * g.max(0.0)
            })
            .collect()
    }

    pub fn sample(&self, a: &[f32], x: f32, y: f32) -> f32 {
        let (gw, gh) = (self.gw as f32, self.gh as f32);
        let fx = (x / STEP).clamp(0.0, gw - 1.001);
        let fy = (y / STEP).clamp(0.0, gh - 1.001);
        let (ix, iy) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - ix, fy - iy);
        let i = iy as usize * self.gw + ix as usize;
        let g = self.gw;
        (a[i] * (1.0 - tx) + a[i + 1] * tx) * (1.0 - ty) + (a[i + g] * (1.0 - tx) + a[i + g + 1] * tx) * ty
    }
}

// ---------------------------------------------------------------- Trạng thái chung của một cảnh (1 màn hình)
pub struct Base {
    pub w: f32,
    pub h: f32,
    pub scale: f32,
    pub buf_w: i32,
    pub buf_h: i32,
    pub noise: Noise,
    pub field: Field,
    pub ftex: [glow::Texture; 2],
    pub from_i: usize,
    pub to_i: usize,
    morph_t0: f64,
    morph_dur: f64,
    pub fa: Vec<f32>,
    pub fb: Vec<f32>,
    pub cur_mix: f32,
    pub icons: Vec<IconBox>,
    pub field_ms: f64,
}

impl Base {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32) -> Self {
        let noise = Noise { seed };
        let field = Field::new(w, h, &noise);
        let ftex = [alloc_field_tex(gl, &field), alloc_field_tex(gl, &field)];
        Base {
            w,
            h,
            scale,
            buf_w,
            buf_h,
            noise,
            field,
            ftex,
            from_i: 0,
            to_i: 0,
            morph_t0: -1e9,
            morph_dur: MORPH_MS,
            fa: Vec::new(),
            fb: Vec::new(),
            cur_mix: 1.0,
            icons: Vec::new(),
            field_ms: 0.0,
        }
    }

    fn compute(&mut self, rects: &[IconRect], sigma: f32, warp: f32) -> Vec<f32> {
        let t0 = std::time::Instant::now();
        let s = self.field.compute(rects, sigma, warp);
        self.field_ms = t0.elapsed().as_secs_f64() * 1000.0;
        s
    }

    /// Trường mới -> nội suy từ trường cũ sang (giống commit() trong JS)
    pub fn commit(&mut self, gl: &glow::Context, rects: &[IconRect], sigma: f32, warp: f32, from_empty: bool, now: f64) {
        let nf = self.compute(rects, sigma, warp);
        let target = 1 - self.to_i;
        if from_empty {
            self.fa = self.field.compute(&[], sigma, warp);
            upload_field(gl, self.ftex[self.to_i], &self.field, &self.fa);
            self.fb = nf;
        } else {
            self.fa = std::mem::replace(&mut self.fb, nf);
        }
        upload_field(gl, self.ftex[target], &self.field, &self.fb);
        self.from_i = self.to_i;
        self.to_i = target;
        self.morph_t0 = now;
        self.morph_dur = if from_empty { 2600.0 } else { MORPH_MS };
    }

    pub fn update_mix(&mut self, now: f64) {
        let e = ((now - self.morph_t0) / self.morph_dur).min(1.0) as f32;
        self.cur_mix = if e < 0.5 { 2.0 * e * e } else { 1.0 - (-2.0 * e + 2.0).powi(2) / 2.0 };
    }

    pub fn settle(&mut self) {
        self.cur_mix = 1.0;
        self.morph_t0 = -1e9;
    }

    pub fn cur_s(&self, x: f32, y: f32) -> f32 {
        if self.fb.is_empty() {
            return 999.0;
        }
        self.field.sample(&self.fa, x, y) * (1.0 - self.cur_mix) + self.field.sample(&self.fb, x, y) * self.cur_mix
    }

    /// Các uniform chung của mọi shader chủ đề
    pub fn bind_common(&self, gl: &glow::Context, p: &mut Prog, noise_tex: glow::Texture, seed: i32, t: f32) {
        unsafe {
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(noise_tex));
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.ftex[self.from_i]));
            gl.active_texture(glow::TEXTURE2);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.ftex[self.to_i]));
        }
        p.i1(gl, "uNoise", 0);
        p.i1(gl, "uFA", 1);
        p.i1(gl, "uFB", 2);
        p.f2(gl, "uRes", self.buf_w as f32 / self.scale, self.buf_h as f32 / self.scale);
        p.f2(gl, "uGrid", self.field.gw as f32, self.field.gh as f32);
        p.f1(gl, "uScale", self.scale);
        p.f1(gl, "uSeed", (seed.rem_euclid(997)) as f32 * 0.37);
        p.f1(gl, "uStep", STEP);
        p.f1(gl, "uMix", self.cur_mix);
        p.f1(gl, "uTime", t);
        p.f1(gl, "uDebug", 0.0);
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_texture(self.ftex[0]);
            gl.delete_texture(self.ftex[1]);
        }
    }
}

fn alloc_field_tex(gl: &glow::Context, f: &Field) -> glow::Texture {
    unsafe {
        let t = gl.create_texture().expect("field texture");
        gl.bind_texture(glow::TEXTURE_2D, Some(t));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::R16F as i32,
            f.gw as i32,
            f.gh as i32,
            0,
            glow::RED,
            glow::FLOAT,
            None,
        );
        set_tex_params(gl, glow::LINEAR, glow::CLAMP_TO_EDGE);
        t
    }
}

fn upload_field(gl: &glow::Context, tex: glow::Texture, f: &Field, s: &[f32]) {
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            0,
            0,
            f.gw as i32,
            f.gh as i32,
            glow::RED,
            glow::FLOAT,
            glow::PixelUnpackData::Slice(f32_bytes(s)),
        );
    }
}

pub fn set_tex_params(gl: &glow::Context, filter: u32, wrap: u32) {
    unsafe {
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap as i32);
    }
}

pub fn f32_bytes(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

pub fn build_noise_tex(gl: &glow::Context, noise: &Noise) -> glow::Texture {
    let (s, d) = noise.make_noise_image();
    unsafe {
        let t = gl.create_texture().expect("noise texture");
        gl.bind_texture(glow::TEXTURE_2D, Some(t));
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, s, s, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&d));
        gl.generate_mipmap(glow::TEXTURE_2D);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
        t
    }
}

// ---------------------------------------------------------------- Chương trình shader
pub struct Prog {
    pub prog: glow::Program,
    locs: HashMap<&'static str, Option<glow::UniformLocation>>,
}

const VERT: &str = include_str!("../assets/shaders/fullscreen.vert");

/// GLSL ES 3.00 (WebGL2) -> GLSL 3.30 core (OpenGL desktop). Phần thân shader giữ nguyên.
fn to_desktop(src: &str) -> String {
    src.replacen("#version 300 es", "#version 330 core", 1)
}

impl Prog {
    pub fn new(gl: &glow::Context, frag: &str) -> Result<Prog, String> {
        let attempt = |strip_precision: bool| -> Result<glow::Program, String> {
            unsafe {
                let fix = |s: &str| {
                    let s = to_desktop(s);
                    if strip_precision {
                        s.replace("precision highp float;", "").replace("uniform highp ", "uniform ")
                    } else {
                        s
                    }
                };
                let mk = |ty: u32, src: String| -> Result<glow::Shader, String> {
                    let sh = gl.create_shader(ty)?;
                    gl.shader_source(sh, &src);
                    gl.compile_shader(sh);
                    if !gl.get_shader_compile_status(sh) {
                        let log = gl.get_shader_info_log(sh);
                        gl.delete_shader(sh);
                        return Err(log);
                    }
                    Ok(sh)
                };
                let vs = mk(glow::VERTEX_SHADER, fix(VERT))?;
                let fs = mk(glow::FRAGMENT_SHADER, fix(frag))?;
                let p = gl.create_program()?;
                gl.attach_shader(p, vs);
                gl.attach_shader(p, fs);
                gl.link_program(p);
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                if !gl.get_program_link_status(p) {
                    return Err(gl.get_program_info_log(p));
                }
                Ok(p)
            }
        };
        let prog = match attempt(false) {
            Ok(p) => p,
            Err(e) => {
                crate::log!("Biên dịch shader lần 1 lỗi, thử bỏ precision: {e}");
                attempt(true)?
            }
        };
        Ok(Prog { prog, locs: HashMap::new() })
    }

    fn loc(&mut self, gl: &glow::Context, name: &'static str) -> Option<glow::UniformLocation> {
        let prog = self.prog;
        self.locs
            .entry(name)
            .or_insert_with(|| unsafe { gl.get_uniform_location(prog, name) })
            .clone()
    }
    pub fn f1(&mut self, gl: &glow::Context, n: &'static str, v: f32) {
        let l = self.loc(gl, n);
        unsafe { gl.uniform_1_f32(l.as_ref(), v) }
    }
    pub fn f2(&mut self, gl: &glow::Context, n: &'static str, a: f32, b: f32) {
        let l = self.loc(gl, n);
        unsafe { gl.uniform_2_f32(l.as_ref(), a, b) }
    }
    pub fn i1(&mut self, gl: &glow::Context, n: &'static str, v: i32) {
        let l = self.loc(gl, n);
        unsafe { gl.uniform_1_i32(l.as_ref(), v) }
    }
    pub fn f4v(&mut self, gl: &glow::Context, n: &'static str, v: &[f32]) {
        let l = self.loc(gl, n);
        unsafe { gl.uniform_4_f32_slice(l.as_ref(), v) }
    }
}

// ---------------------------------------------------------------- Giao diện chủ đề
pub trait Theme {
    /// Biểu tượng thay đổi (toạ độ CSS của màn hình này). `first` = lần đầu nhận biểu tượng.
    fn set_icons(&mut self, gl: &glow::Context, icons: Vec<IconBox>, first: bool, now: f64);
    /// Cập nhật mô phỏng (dt giây) rồi vẽ vào framebuffer đang bind (kích thước buf_w x buf_h).
    fn frame(&mut self, gl: &glow::Context, shared: &mut Shared, dt: f32, now: f64);
    /// Vẽ lại một khung đứng yên (khi đang tạm dừng nhưng biểu tượng vừa đổi).
    fn settle(&mut self, gl: &glow::Context, shared: &mut Shared, now: f64);
    fn destroy(&mut self, gl: &glow::Context);
    fn field_ms(&self) -> f64;
}

/// Tài nguyên GPU dùng chung cho mọi màn hình (một ngữ cảnh OpenGL duy nhất).
pub struct Shared {
    pub noise_tex: glow::Texture,
    pub seed: i32,
    pub prog: Prog,
}
