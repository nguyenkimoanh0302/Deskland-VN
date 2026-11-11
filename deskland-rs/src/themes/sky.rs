//! Chủ đề Bầu trời (sky.html): mây trắng tròn xoe nâng đỡ biểu tượng, hỗ trợ Ban ngày / Hoàng hôn / Ban đêm,
//! trăng lưỡi liềm, bầu trời sao, đàn chim và máy bay kéo vệt khói.

use crate::engine::*;
use glow::HasContext;
use std::collections::HashMap;
use std::f32::consts::PI;

pub const FRAG: &str = include_str!("../../assets/shaders/sky.frag");

const CELL_G: f32 = 48.0;
const CIRC_K: usize = 36;

pub struct Params {
    pub time: i32,
    pub sigma: f32,
    pub warp: f32,
    pub edge: f32,
    pub cover: f32,
    pub flow: f32,
    pub birds: f32,
    pub plane: i32,
    pub moon: f32,
    pub stars: f32,
    pub adaptive: bool,
}

impl Params {
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        let gb = |k: &str, d: bool| {
            v.and_then(|o| o.get(k))
                .and_then(|x| x.as_bool().or_else(|| x.as_f64().map(|n| n != 0.0)))
                .unwrap_or(d)
        };
        Params {
            time: g("time", 0.0).round() as i32,
            sigma: g("sigma", 74.0),
            warp: g("warp", 38.0),
            edge: g("edge", 10.0),
            cover: g("cover", 0.35),
            flow: g("flow", 1.0),
            birds: g("birds", 3.0),
            plane: g("plane", 1.0).round() as i32,
            moon: g("moon", 1.0),
            stars: g("stars", 0.6),
            adaptive: gb("adaptive", true),
        }
    }
}

struct SkyCloud {
    x: f32,
    y: f32,
    circles: Vec<[f32; 4]>,
    v: f32,
    grow: f32,
    hw: f32,
    dying: bool,
    ttl: Option<f32>,
}

struct Bird {
    x: f32,
    y: f32,
    h: f32,
    v: f32,
    mode: f32,
    mode_t: f32,
    ph: f32,
    flap: bool,
    flap_t: f32,
    wander: f32,
}

struct PlaneState {
    active: bool,
    forced: bool,
    x: f32,
    y: f32,
    dir: f32,
    wait: f32,
}

pub struct Sky {
    b: Base,
    p: Params,
    rng: Rng,
    sky: Vec<SkyCloud>,
    birds: Vec<Bird>,
    plane: PlaneState,
    meteors: Vec<[f32; 4]>,
    flow_t: f32,
    mass: Vec<f32>,
    mass_t: f32,
    mass_valid: bool,
    circ_tex: Option<glow::Texture>,
    circ_buf: Vec<f32>,
    cg_w: usize,
    cg_h: usize,
    t0: f64,
}

fn turn_toward(h: f32, target: f32, rate: f32) -> f32 {
    let mut d = target - h;
    d = d.sin().atan2(d.cos());
    h + d.signum() * d.abs().min(rate)
}

impl Sky {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = Sky {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            sky: Vec::new(),
            birds: Vec::new(),
            plane: PlaneState { active: false, forced: false, x: 0.0, y: 120.0, dir: 1.0, wait: 3.0 },
            meteors: Vec::new(),
            flow_t: 0.0,
            mass: Vec::new(),
            mass_t: -1e9,
            mass_valid: false,
            circ_tex: None,
            circ_buf: Vec::new(),
            cg_w: 0,
            cg_h: 0,
            t0: now,
        };
        s.commit(gl, true, now);
        s.sync_birds(false);
        s
    }

    fn commit(&mut self, gl: &glow::Context, from_empty: bool, now: f64) {
        let rects = if self.p.adaptive { icon_rects(&self.b.icons) } else { Vec::new() };
        self.b.commit(gl, &rects, self.p.sigma, self.p.warp, from_empty, now);
    }

    fn effective_phase(&self) -> f32 {
        match self.p.time {
            1 => 1.0,
            2 => 2.0,
            3 => 3.0,
            _ => {
                #[repr(C)]
                struct SystemTime {
                    year: u16,
                    month: u16,
                    day_of_week: u16,
                    day: u16,
                    hour: u16,
                    minute: u16,
                    second: u16,
                    milliseconds: u16,
                }
                extern "system" {
                    fn GetLocalTime(lp_system_time: *mut SystemTime);
                }
                let mut st = SystemTime { year: 0, month: 0, day_of_week: 0, day: 0, hour: 12, minute: 0, second: 0, milliseconds: 0 };
                unsafe { GetLocalTime(&mut st) };
                let hr = st.hour as f32 + st.minute as f32 / 60.0;
                if (7.0..17.5).contains(&hr) {
                    1.0
                } else if (17.5..19.0).contains(&hr) || (6.0..7.0).contains(&hr) {
                    2.0
                } else {
                    3.0
                }
            }
        }
    }

    fn make_sky_cloud(&mut self, x: f32) -> SkyCloud {
        let r_big = 95.0 * 1.2;
        let n = 3 + (self.rng.next() * 3.0).floor() as usize;
        let w = r_big * (1.0 + n as f32 * 0.5);
        let mut body: Vec<[f32; 3]> = Vec::new();
        for i in 0..n {
            let t = if n == 1 { 0.0 } else { i as f32 / (n - 1) as f32 - 0.5 };
            let r = r_big * (0.6 - t.abs() * 0.45 + self.rng.next() * 0.12);
            body.push([t * w, -r * 0.55, r]);
        }
        for i in 0..n - 1 {
            let t = if n == 2 { 0.0 } else { i as f32 / (n - 2) as f32 - 0.5 };
            let r = r_big * (0.55 + self.rng.next() * 0.25);
            let bx = t * w * 0.6 + (self.rng.next() - 0.5) * r_big * 0.4;
            let by = -r_big * (0.5 + self.rng.next() * 0.35);
            body.push([bx, by, r]);
        }
        if self.rng.next() < 0.6 {
            let bx = (self.rng.next() - 0.5) * w * 0.3;
            let r = r_big * (0.4 + self.rng.next() * 0.15);
            body.push([bx, -r_big * 1.05, r]);
        }
        let ci_of = |px: f32, py: f32| body.iter().fold(1e9f32, |d, c| d.min((px - c[0]).hypot(py - c[1]) - c[2]));
        let (mut x0, mut x1, mut y0, mut y1) = (1e9f32, -1e9f32, 1e9f32, -1e9f32);
        for c in &body {
            x0 = x0.min(c[0] - c[2]);
            x1 = x1.max(c[0] + c[2]);
            y0 = y0.min(c[1] - c[2]);
            y1 = y1.max(c[1] + c[2]);
        }
        let mut circles: Vec<[f32; 4]> = Vec::new();
        let s = 0.62 / 1.2;
        let rm = r_big * s;
        let gb = 90.0;
        let rng = &mut self.rng;
        let layer = |rng: &mut Rng, circles: &mut Vec<[f32; 4]>, g: f32, size: [f32; 2], band: f32, band_fn: &dyn Fn(f32) -> f32, attach: bool| {
            let gy0 = ((y0 - g) / g).floor() as i32;
            let gy1 = ((y1 + g) / g).ceil() as i32;
            let gx0 = ((x0 - g) / g).floor() as i32;
            let gx1 = ((x1 + g) / g).ceil() as i32;
            for gy in gy0..=gy1 {
                for gx in gx0..=gx1 {
                    let px = (gx as f32 + 0.1 + rng.next() * 0.8) * g;
                    let py = (gy as f32 + 0.1 + rng.next() * 0.8) * g;
                    let ci = ci_of(px, py);
                    if ci > 0.0 || ci < band {
                        continue;
                    }
                    let mut r = g * (size[0] + (size[1] - size[0]) * rng.next()) * band_fn(ci);
                    if attach {
                        let mut att = 0f32;
                        for c in circles.iter() {
                            att = att.max(sstep(0.2 * r, 0.6 * r, c[2] + r - (c[0] - px).hypot(c[1] - py)));
                        }
                        r *= att;
                    }
                    r *= sstep(5.0, 11.0, r);
                    if r > 0.5 {
                        circles.push([px, py, r, rng.next()]);
                    }
                }
            }
        };
        layer(rng, &mut circles, gb, [0.55, 0.85], -1e9, &|ci| sstep(-0.2 * gb, -0.65 * gb, ci), false);
        layer(rng, &mut circles, rm, [0.6, 0.95], -1e9, &|ci| sstep(-0.1 * rm, -0.7 * rm, ci), false);
        for c in &body {
            if c[2] > 0.75 * rm {
                let ph = rng.next();
                circles.push([c[0], c[1], c[2] - 0.75 * rm, ph]);
            }
        }
        layer(rng, &mut circles, 34.0 * 0.85, [0.5, 0.9], -1.0 * rm, &|ci| sstep(-0.05 * rm, -0.35 * rm, ci) * (1.0 - sstep(-0.7 * rm, -1.0 * rm, ci)), true);
        layer(rng, &mut circles, 24.0 * 0.85, [0.45, 0.8], -0.6 * rm, &|ci| sstep(0.0, -0.2 * rm, ci) * (1.0 - sstep(-0.4 * rm, -0.6 * rm, ci)), true);
        let y = 80.0 + self.rng.next() * (self.b.h - TASKBAR - 120.0);
        let v = 0.7 + self.rng.next() * 0.6;
        SkyCloud { x, y, circles, v, grow: 0.0, hw: (-x0).max(x1) + 20.0, dying: false, ttl: None }
    }

    fn make_puff_cloud(&mut self, x: f32, y: f32) -> SkyCloud {
        let mut c = self.make_sky_cloud(x);
        c.y = y + 12.0;
        for cir in &mut c.circles {
            cir[0] *= 0.56;
            cir[1] *= 0.56;
            cir[2] *= 0.60;
        }
        c.hw *= 0.58;
        c.ttl = Some(7.0);
        c
    }

    fn step_sky_clouds(&mut self, dt: f32) {
        self.flow_t += dt * self.p.flow;
        let want = (self.p.cover * 10.0).round() as usize;
        let alive: Vec<usize> = (0..self.sky.len()).filter(|&i| !self.sky[i].dying && self.sky[i].ttl.is_none()).collect();
        for &i in alive.iter().skip(want) {
            self.sky[i].dying = true;
        }
        for _ in alive.len()..want {
            let mut best: Option<SkyCloud> = None;
            let mut best_d = -1f32;
            for _ in 0..20 {
                let x = self.rng.next() * self.b.w;
                let c = self.make_sky_cloud(x);
                let dmin = self.sky.iter().fold(1e9f32, |d, o| d.min(((o.x - c.x) / 1.6).hypot(o.y - c.y)));
                if dmin > best_d {
                    best_d = dmin;
                    best = Some(c);
                }
            }
            if let Some(c) = best {
                self.sky.push(c);
            }
        }
        let w = self.b.w;
        let flow = self.p.flow;
        let mut i = self.sky.len();
        while i > 0 {
            i -= 1;
            if let Some(ttl) = &mut self.sky[i].ttl {
                *ttl -= dt;
                if *ttl <= 0.0 {
                    self.sky[i].dying = true;
                }
            }
            let rate = if self.sky[i].ttl.is_some() && !self.sky[i].dying { 2.2 } else { 0.5 };
            self.sky[i].x += flow.max(0.25) * 12.0 * self.sky[i].v * dt;
            self.sky[i].grow = if self.sky[i].dying { self.sky[i].grow - dt * 0.65 } else { (self.sky[i].grow + dt * rate).min(1.0) };
            if self.sky[i].grow <= 0.0 {
                self.sky.remove(i);
                continue;
            }
            if self.sky[i].x - self.sky[i].hw > w + 50.0 {
                if self.sky[i].ttl.is_some() {
                    self.sky.remove(i);
                } else {
                    let hw = self.sky[i].hw;
                    let mut n = self.make_sky_cloud(-hw - 50.0);
                    n.grow = 1.0;
                    self.sky[i] = n;
                }
            }
        }
    }

    fn build_mass_circles(&self) -> Vec<f32> {
        let mut list: Vec<f32> = Vec::new();
        let (w, h, edge, noise) = (self.b.w, self.b.h, self.p.edge, &self.b.noise);
        let s = 0.62f32;
        let rm = 95.0 * s;
        let mut mgrid: HashMap<i64, Vec<usize>> = HashMap::new();
        let layer = |list: &mut Vec<f32>, mgrid: &mut HashMap<i64, Vec<usize>>, g: f32, size: [f32; 2], k: i32, band: f32, band_fn: &dyn Fn(f32) -> f32, attach: bool, store: bool| {
            let gy1 = ((h + g) / g).ceil() as i32;
            let gx1 = ((w + g) / g).ceil() as i32;
            for gy in -1..=gy1 {
                for gx in -1..=gx1 {
                    let px = (gx as f32 + 0.1 + noise.hash2(gx as f64, gy as f64, k) as f32 * 0.8) * g;
                    let py = (gy as f32 + 0.1 + noise.hash2(gx as f64, gy as f64, k + 3) as f32 * 0.8) * g;
                    let ci = self.b.cur_s(px, py) - edge;
                    if ci > 0.0 || ci < band {
                        continue;
                    }
                    let mut r = g * (size[0] + (size[1] - size[0]) * noise.hash2(gx as f64, gy as f64, k + 7) as f32) * band_fn(ci);
                    if attach {
                        let mut att = 0f32;
                        let cx = (px / rm).floor() as i64;
                        let cy = (py / rm).floor() as i64;
                        for dy in -1..=1 {
                            for dx in -1..=1 {
                                if let Some(cell) = mgrid.get(&((cy + dy) * 10000 + cx + dx)) {
                                    for &i in cell {
                                        let d = (list[i] - px).hypot(list[i + 1] - py);
                                        att = att.max(sstep(0.2 * r, 0.6 * r, list[i + 2] + r - d));
                                    }
                                }
                            }
                        }
                        r *= att;
                    }
                    r *= sstep(5.0, 11.0, r);
                    if r > 0.5 {
                        if store {
                            let key = (py / rm).floor() as i64 * 10000 + (px / rm).floor() as i64;
                            mgrid.entry(key).or_default().push(list.len());
                        }
                        let ph = noise.hash2(gx as f64, gy as f64, k + 13) as f32;
                        list.extend_from_slice(&[px, py, r, ph]);
                    }
                }
            }
        };
        layer(&mut list, &mut mgrid, 90.0, [0.55, 0.85], 101, -1e9, &|ci| sstep(-0.2 * 90.0, -0.65 * 90.0, ci), false, true);
        layer(&mut list, &mut mgrid, rm, [0.6, 0.95], 203, -1e9, &|ci| sstep(-0.1 * rm, -0.7 * rm, ci), false, true);
        let g = 32.0f32;
        let gy1 = (h / g).ceil() as i32;
        let gx1 = (w / g).ceil() as i32;
        for gy in 0..=gy1 {
            for gx in 0..=gx1 {
                let px = (gx as f32 + 0.5) * g;
                let py = (gy as f32 + 0.5) * g;
                let ci = self.b.cur_s(px, py) - edge;
                if -ci > 0.75 * rm {
                    let ph = noise.hash2(gx as f64, gy as f64, 505) as f32;
                    list.extend_from_slice(&[px, py, -ci - 0.75 * rm, ph]);
                }
            }
        }
        layer(&mut list, &mut mgrid, 34.0 * 0.85, [0.5, 0.9], 307, -1.0 * rm, &|ci| sstep(-0.05 * rm, -0.35 * rm, ci) * (1.0 - sstep(-0.7 * rm, -1.0 * rm, ci)), true, true);
        layer(&mut list, &mut mgrid, 24.0 * 0.85, [0.45, 0.8], 409, -0.6 * rm, &|ci| sstep(0.0, -0.2 * rm, ci) * (1.0 - sstep(-0.4 * rm, -0.6 * rm, ci)), true, false);
        list
    }

    fn upload_circles(&mut self, gl: &glow::Context, t: f32) {
        unsafe { gl.active_texture(glow::TEXTURE3) };
        let gw = (self.b.w / CELL_G).ceil() as usize + 7;
        let gh = (self.b.h / CELL_G).ceil() as usize + 7;
        if self.circ_tex.is_none() || gw != self.cg_w || gh != self.cg_h {
            self.cg_w = gw;
            self.cg_h = gh;
            unsafe {
                if let Some(t) = self.circ_tex.take() {
                    gl.delete_texture(t);
                }
                let tex = gl.create_texture().expect("circle texture");
                gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA32F as i32, (gw * CIRC_K) as i32, gh as i32, 0, glow::RGBA, glow::FLOAT, None);
                set_tex_params(gl, glow::NEAREST, glow::CLAMP_TO_EDGE);
                self.circ_tex = Some(tex);
                self.circ_buf = vec![0f32; gw * CIRC_K * gh * 4];
            }
        }
        let morphing = self.b.cur_mix < 1.0 || !self.mass_valid;
        if morphing || t - self.mass_t > 0.5 {
            self.mass = self.build_mass_circles();
            self.mass_valid = true;
            self.mass_t = t;
        }
        let ft = self.flow_t;
        let wob_x = |x: f32, y: f32, r: f32, ph: f32| (4.5 + 0.12 * r) * (0.85 * (ft * 0.24 + x * 0.011 + y * 0.004).sin() + 0.3 * (ft * 0.3 + ph * 6.283).sin());
        let wob_y = |x: f32, y: f32, r: f32, ph: f32| (4.5 + 0.12 * r) * (0.65 * (ft * 0.2 + y * 0.013 - x * 0.006 + 1.7).sin() + 0.25 * (ft * 0.28 + ph * 6.283).cos());
        let mut list: Vec<f32> = Vec::with_capacity(self.mass.len() + 256);
        for c in self.mass.chunks_exact(4) {
            let (x, y, r, ph) = (c[0], c[1], c[2], c[3]);
            list.extend_from_slice(&[x + wob_x(x, y, r, ph), y + wob_y(x, y, r, ph), r, ph]);
        }
        for c in &self.sky {
            let g = c.grow * c.grow * (3.0 - 2.0 * c.grow);
            for (i, &[dx, dy, r, ph]) in c.circles.iter().enumerate() {
                let rb = r * g * (1.0 + 0.04 * (ft * (0.09 + 0.035 * ph) + ph * 6.283 + i as f32).sin());
                if rb > 0.5 {
                    list.extend_from_slice(&[c.x + dx + wob_x(dx, dy, r, ph), c.y + dy + wob_y(dx, dy, r, ph), rb, ph]);
                }
            }
        }
        self.circ_buf.fill(0.0);
        let mut cnt = vec![0u8; gw * gh];
        let mut order: Vec<usize> = (0..list.len() / 4).map(|i| i * 4).collect();
        order.sort_by(|&a, &b| list[b + 2].partial_cmp(&list[a + 2]).unwrap_or(std::cmp::Ordering::Equal));
        for i in order {
            let (x, y, r) = (list[i], list[i + 1], list[i + 2] + 6.0);
            let fx0 = ((x - r) / CELL_G).floor() as i64 + 4;
            let fx1 = ((x + r) / CELL_G).floor() as i64 + 4;
            let fy0 = ((y - r) / CELL_G).floor() as i64 + 4;
            let fy1 = ((y + r) / CELL_G).floor() as i64 + 4;
            let x0 = fx0.max(0);
            let x1 = fx1.min(gw as i64 - 1);
            let y0 = fy0.max(0);
            let y1 = fy1.min(gh as i64 - 1);
            for gy in y0..=y1 {
                for gx in x0..=x1 {
                    let ci = gy as usize * gw + gx as usize;
                    let nx = ((gx - 4) as f32 * CELL_G).max(x.min((gx - 3) as f32 * CELL_G));
                    let ny = ((gy - 4) as f32 * CELL_G).max(y.min((gy - 3) as f32 * CELL_G));
                    if (nx - x) * (nx - x) + (ny - y) * (ny - y) > r * r {
                        continue;
                    }
                    if cnt[ci] as usize >= CIRC_K {
                        continue;
                    }
                    let o = ((gy as usize * gw + gx as usize) * CIRC_K + cnt[ci] as usize) * 4;
                    cnt[ci] += 1;
                    self.circ_buf[o..o + 4].copy_from_slice(&[x, y, list[i + 2], list[i + 3]]);
                }
            }
        }
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, self.circ_tex);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                (gw * CIRC_K) as i32,
                gh as i32,
                glow::RGBA,
                glow::FLOAT,
                glow::PixelUnpackData::Slice(f32_bytes(&self.circ_buf)),
            );
        }
    }

    fn spawn_bird(&mut self, from_edge: bool) -> Bird {
        let (w, h2) = (self.b.w, self.b.h - TASKBAR);
        let mut x = self.rng.next() * w;
        let mut y = self.rng.next() * h2;
        if from_edge {
            match (self.rng.next() * 4.0).floor() as i32 {
                0 => x = -40.0,
                1 => x = w + 40.0,
                2 => y = -40.0,
                _ => y = h2 + 40.0,
            }
        }
        let h = (h2 / 2.0 - y).atan2(w / 2.0 - x) + (self.rng.next() - 0.5) * 1.2;
        Bird {
            x,
            y,
            h,
            v: 36.0 + self.rng.next() * 16.0,
            mode: 0.0,
            mode_t: 2.0 + self.rng.next() * 4.0,
            ph: 0.0,
            flap: false,
            flap_t: self.rng.next() * 3.0,
            wander: self.rng.next() * 100.0,
        }
    }

    fn sync_birds(&mut self, from_edge: bool) {
        let want = self.p.birds.round().clamp(0.0, 8.0) as usize;
        self.birds.truncate(want);
        while self.birds.len() < want {
            let b = self.spawn_bird(from_edge);
            self.birds.push(b);
        }
    }

    fn step_birds(&mut self, dt: f32) {
        let (w, h2, m) = (self.b.w, self.b.h - TASKBAR, 90.0);
        let noise = self.b.noise;
        for i in 0..self.birds.len() {
            let r1 = self.rng.next();
            let r2 = self.rng.next();
            let r3 = self.rng.next();
            let r4 = self.rng.next();
            let b = &mut self.birds[i];
            b.mode_t -= dt;
            if b.mode_t <= 0.0 {
                b.mode = if r1 < 0.4 { if r2 < 0.5 { -1.0 } else { 1.0 } } else { 0.0 };
                b.mode_t = if b.mode != 0.0 { 4.0 + r3 * 6.0 } else { 3.0 + r3 * 5.0 };
            }
            b.wander += dt * 0.4;
            if b.x < -m || b.x > w + m || b.y < -m || b.y > h2 + m {
                b.h = turn_toward(b.h, (h2 / 2.0 - b.y).atan2(w / 2.0 - b.x), 1.3 * dt);
            } else {
                let wv = noise.vnoise(b.wander as f64, i as f64 * 7.3, 0) as f32;
                b.h += (b.mode * 0.55 + (wv - 0.5) * 0.8) * dt;
            }
            b.flap_t -= dt;
            if b.flap_t <= 0.0 {
                b.flap = !b.flap;
                b.flap_t = if b.flap { 0.8 + r4 * 1.2 } else { 2.5 + r4 * 4.0 };
            }
            if b.flap || b.ph.cos() < 0.97 {
                b.ph = (b.ph + dt * 9.0) % (PI * 2.0);
            }
            let v = b.v * if b.flap { 1.15 } else { 1.0 };
            b.x += b.h.cos() * v * dt;
            b.y += b.h.sin() * v * dt;
        }
    }

    fn step_plane(&mut self, dt: f32) {
        if self.p.plane == 0 && !self.plane.forced {
            self.plane.active = false;
            return;
        }
        if !self.plane.active {
            self.plane.wait -= dt * if self.p.plane == 2 { 2.5 } else { 1.0 };
            if self.plane.wait <= 0.0 {
                self.plane.active = true;
                self.plane.forced = false;
                self.plane.dir = if self.rng.next() < 0.5 { 1.0 } else { -1.0 };
                self.plane.x = if self.plane.dir > 0.0 { -40.0 } else { self.b.w + 40.0 };
                self.plane.y = 50.0 + self.rng.next() * 80.0f32.max(self.b.h * 0.25);
            }
        } else {
            let speed = if self.plane.forced { 74.0 } else { 52.0 };
            self.plane.x += self.plane.dir * speed * dt;
            if (self.plane.dir > 0.0 && self.plane.x > self.b.w + 280.0) || (self.plane.dir < 0.0 && self.plane.x < -280.0) {
                self.plane.active = false;
                self.plane.forced = false;
                self.plane.wait = if self.p.plane == 2 {
                    4.0 + self.rng.next() * 6.0
                } else {
                    14.0 + self.rng.next() * 16.0
                };
            }
        }
    }

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        self.upload_circles(gl, t);
        let mut bird = [0f32; 32];
        for (i, b) in self.birds.iter().enumerate() {
            bird[i * 4..i * 4 + 4].copy_from_slice(&[b.x, b.y, b.h, 0.68 + 0.32 * b.ph.cos()]);
        }
        let mut meteor = [0f32; 16];
        for (i, m) in self.meteors.iter().take(4).enumerate() {
            meteor[i * 4..i * 4 + 4].copy_from_slice(m);
        }
        let phase = self.effective_phase();
        let plane = [self.plane.x, self.plane.y, self.plane.dir, if self.plane.active { 220.0 } else { 0.0 }];
        let p = &mut sh.prog;
        unsafe { gl.use_program(Some(p.prog)) };
        self.b.bind_common(gl, p, sh.noise_tex, sh.seed, t);
        unsafe {
            gl.active_texture(glow::TEXTURE3);
            gl.bind_texture(glow::TEXTURE_2D, self.circ_tex);
        }
        p.i1(gl, "uCirc", 3);
        p.f2(gl, "uCGrid", self.cg_w as f32, self.cg_h as f32);
        p.f1(gl, "uCellG", CELL_G);
        p.f1(gl, "uEdge", self.p.edge);
        p.f1(gl, "uPhase", phase);
        p.f1(gl, "uMoon", self.p.moon);
        p.f1(gl, "uStars", self.p.stars);
        p.f4v(gl, "uPlane", &plane);
        p.f1(gl, "uBirdN", self.birds.len() as f32);
        p.f4v(gl, "uBird", &bird);
        p.f4v(gl, "uMeteor", &meteor);
        unsafe { gl.draw_arrays(glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for Sky {
    fn set_icons(&mut self, gl: &glow::Context, icons: Vec<IconBox>, first: bool, now: f64) {
        self.b.icons = icons;
        self.commit(gl, first, now);
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, now: f64) {
        self.b.update_mix(now);
        self.meteors.retain_mut(|m| {
            m[2] += dt;
            m[2] < 1.4
        });
        self.step_sky_clouds(dt);
        self.step_birds(dt);
        self.step_plane(dt);
        self.draw(gl, sh, now);
    }

    fn settle(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        self.b.settle();
        self.mass_valid = false;
        self.draw(gl, sh, now);
    }

    fn handle_click(&mut self, _gl: &glow::Context, x: f32, y: f32, _now: f64) {
        let phase = self.effective_phase();
        if phase > 2.5 {
            if self.meteors.len() >= 4 {
                self.meteors.remove(0);
            }
            let ang = if x > self.b.w * 0.55 {
                PI - (0.46 + self.rng.next() * 0.28)
            } else {
                0.46 + self.rng.next() * 0.28
            };
            let sx = x - ang.cos() * 42.0;
            let sy = (y - ang.sin() * 42.0).max(16.0);
            self.meteors.push([sx, sy, 0.0, ang]);

            if self.b.cur_s(x, y) < 28.0 {
                let active_puffs: Vec<usize> = (0..self.sky.len())
                    .filter(|&i| self.sky[i].ttl.is_some() && !self.sky[i].dying)
                    .collect();
                if active_puffs.len() >= 3 {
                    self.sky[active_puffs[0]].dying = true;
                }
                let c = self.make_puff_cloud(x, y);
                self.sky.push(c);
            }
        } else {
            let active_puffs: Vec<usize> = (0..self.sky.len())
                .filter(|&i| self.sky[i].ttl.is_some() && !self.sky[i].dying)
                .collect();
            if active_puffs.len() >= 4 {
                self.sky[active_puffs[0]].dying = true;
            }
            let c = self.make_puff_cloud(x, y);
            self.sky.push(c);

            for b in &mut self.birds {
                if (b.x - x).hypot(b.y - y) < 520.0 {
                    b.h = turn_toward(b.h, (y - b.y).atan2(x - b.x), 1.05);
                    b.flap = true;
                    b.flap_t = 1.6;
                }
            }

            if !self.plane.active {
                self.plane.active = true;
                self.plane.forced = true;
                self.plane.dir = if x < self.b.w * 0.5 { 1.0 } else { -1.0 };
                self.plane.x = if self.plane.dir > 0.0 { -32.0 } else { self.b.w + 32.0 };
                self.plane.y = y.clamp(36.0, (self.b.h - TASKBAR - 60.0).max(60.0));
            }
        }
    }

    fn is_interacting(&self, _now: f64) -> bool {
        !self.meteors.is_empty() || self.plane.forced || self.sky.iter().any(|c| c.ttl.is_some())
    }

    fn destroy(&mut self, gl: &glow::Context) {
        self.b.destroy(gl);
        if let Some(t) = self.circ_tex.take() {
            unsafe { gl.delete_texture(t) };
        }
    }

    fn field_ms(&self) -> f64 {
        self.b.field_ms
    }
}
