//! Chủ đề Hồ nước (lake.html): bờ cỏ màu nước bao quanh mặt hồ trong vắt, đá cuội nước nông,
//! bóng mây phản chiếu và mini-game câu cá thư giãn khi nhấn chuột vào mặt hồ.

use crate::engine::*;
use glow::HasContext;
use std::f32::consts::PI;

pub const FRAG: &str = include_str!("../../assets/shaders/lake.frag");

const CELL_G: f32 = 48.0;
const CIRC_K: usize = 36;

pub struct Params {
    pub sigma: f32,
    pub warp: f32,
    pub shoal: f32,
    pub depth: f32,
    pub flow: f32,
    pub clouds: f32,
}

impl Params {
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        Params {
            sigma: g("sigma", 82.0),
            warp: g("warp", 32.0),
            shoal: g("shoal", 95.0),
            depth: g("depth", 240.0),
            flow: g("flow", 1.0),
            clouds: g("clouds", 4.0),
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
}

struct LakeFish {
    x: f32,
    y: f32,
    h: f32,
    v: f32,
    ph: f32,
    wander: f32,
    koi: f32, // 0.0 = cá xanh đen tự nhiên, 1.0 = cá Koi cam vàng
}

enum FishingState {
    Idle,
    Casting { from: (f32, f32), to: (f32, f32), age: f32, dur: f32 },
    Waiting { from: (f32, f32), bob: (f32, f32), age: f32, bite_at: f32 },
    Hooked { from: (f32, f32), bob: (f32, f32), age: f32, next_rip: f32, window: f32 },
    Catching { from: (f32, f32), start: (f32, f32), age: f32, dur: f32 },
    ReelingEmpty { from: (f32, f32), start: (f32, f32), age: f32, dur: f32 },
}

pub struct Lake {
    b: Base,
    p: Params,
    rng: Rng,
    sky: Vec<SkyCloud>,
    fishes: Vec<LakeFish>,
    ripples: Vec<[f32; 4]>, // x, y, age, strength
    fishing: FishingState,
    flow_t: f32,
    circ_tex: Option<glow::Texture>,
    circ_buf: Vec<f32>,
    cg_w: usize,
    cg_h: usize,
    t0: f64,
}

fn turn_toward(h: f32, target: f32, rate: f32) -> f32 {
    let d = (target - h).sin().atan2((target - h).cos());
    h + d.signum() * d.abs().min(rate)
}

impl Lake {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = Lake {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            sky: Vec::new(),
            fishes: Vec::new(),
            ripples: Vec::new(),
            fishing: FishingState::Idle,
            flow_t: 0.0,
            circ_tex: None,
            circ_buf: Vec::new(),
            cg_w: 0,
            cg_h: 0,
            t0: now,
        };
        s.commit(gl, true, now);
        s
    }

    fn commit(&mut self, gl: &glow::Context, from_empty: bool, now: f64) {
        let rects = icon_rects(&self.b.icons);
        self.b.commit(gl, &rects, self.p.sigma, self.p.warp, from_empty, now);
    }

    fn push_ripple(&mut self, x: f32, y: f32, strength: f32) {
        if self.ripples.len() >= 6 {
            self.ripples.remove(0);
        }
        self.ripples.push([x, y, 0.0, strength]);
    }

    fn find_rod_anchor(&self, tx: f32, ty: f32) -> (f32, f32) {
        let mut best: Option<(f32, f32, f32)> = None;
        for dir_i in 0..24 {
            let ang = dir_i as f32 * (PI * 2.0 / 24.0);
            let (dx, dy) = (ang.cos(), ang.sin());
            let mut dist = 24.0;
            while dist <= 460.0 {
                let (px, py) = (tx + dx * dist, ty + dy * dist);
                if px >= 12.0 && px <= self.b.w - 12.0 && py >= 12.0 && py <= self.b.h - TASKBAR - 8.0 && self.b.cur_s(px, py) <= -3.0 {
                    if best.is_none_or(|b| dist < b.2) {
                        best = Some((px, py, dist));
                    }
                    break;
                }
                dist += 16.0;
            }
        }
        if let Some((bx, by, _)) = best {
            (bx, by)
        } else {
            let ax = (tx + 38.0).clamp(60.0, self.b.w - 60.0);
            let ay = self.b.h + 8.0;
            (ax, ay)
        }
    }

    fn spawn_fish(&mut self, idx: usize) -> Option<LakeFish> {
        let (w, h) = (self.b.w, self.b.h);
        for _ in 0..250 {
            let x = 60.0 + self.rng.next() * (w - 120.0);
            let y = 60.0 + self.rng.next() * (h - TASKBAR - 120.0);
            if self.b.cur_s(x, y) > 55.0 {
                return Some(LakeFish {
                    x,
                    y,
                    h: self.rng.next() * PI * 2.0,
                    v: 11.0 + self.rng.next() * 6.0,
                    ph: self.rng.next() * 6.28,
                    wander: self.rng.next() * 80.0,
                    koi: if idx == 0 { 1.0 } else if self.rng.next() < 0.45 { 1.0 } else { 0.0 },
                });
            }
        }
        None
    }

    fn sync_fishes(&mut self) {
        while self.fishes.len() < 4 {
            let idx = self.fishes.len();
            match self.spawn_fish(idx) {
                Some(f) => self.fishes.push(f),
                None => break,
            }
        }
    }

    fn step_fishes_and_fishing(&mut self, dt: f32) {
        // Cập nhật các vòng gợn sóng
        let mut r_i = self.ripples.len();
        while r_i > 0 {
            r_i -= 1;
            self.ripples[r_i][2] += dt;
            if self.ripples[r_i][2] > 3.2 {
                self.ripples.remove(r_i);
            }
        }

        // Cập nhật máy trạng thái câu cá
        let mut spawn_rip: Option<(f32, f32, f32)> = None;
        let next_state = match &mut self.fishing {
            FishingState::Idle => None,
            FishingState::Casting { from, to, age, dur } => {
                *age += dt;
                if *age >= *dur {
                    let (fx, fy) = *from;
                    let (tx, ty) = *to;
                    spawn_rip = Some((tx, ty, 1.0));
                    let bite_at = 2.8 + self.rng.next() * 2.8;
                    Some(FishingState::Waiting { from: (fx, fy), bob: (tx, ty), age: 0.0, bite_at })
                } else {
                    None
                }
            }
            FishingState::Waiting { from, bob, age, bite_at } => {
                *age += dt;
                if *age >= *bite_at {
                    let (fx, fy) = *from;
                    let (bx, by) = *bob;
                    spawn_rip = Some((bx, by, 1.15));
                    Some(FishingState::Hooked { from: (fx, fy), bob: (bx, by), age: 0.0, next_rip: 0.38, window: 3.2 })
                } else {
                    None
                }
            }
            FishingState::Hooked { from, bob, age, next_rip, window } => {
                *age += dt;
                *next_rip -= dt;
                if *next_rip <= 0.0 {
                    *next_rip = 0.42;
                    spawn_rip = Some((bob.0, bob.1, 0.95));
                }
                if *age >= *window {
                    let (fx, fy) = *from;
                    let (bx, by) = *bob;
                    Some(FishingState::ReelingEmpty { from: (fx, fy), start: (bx, by), age: 0.0, dur: 0.55 })
                } else {
                    None
                }
            }
            FishingState::Catching { age, dur, .. } => {
                *age += dt;
                if *age >= *dur { Some(FishingState::Idle) } else { None }
            }
            FishingState::ReelingEmpty { age, dur, .. } => {
                *age += dt;
                if *age >= *dur { Some(FishingState::Idle) } else { None }
            }
        };
        if let Some(ns) = next_state {
            self.fishing = ns;
        }
        if let Some((rx, ry, rs)) = spawn_rip {
            self.push_ripple(rx, ry, rs);
        }

        // Điểm thu hút cá khi đang thả phao
        let lure_target = match &self.fishing {
            FishingState::Waiting { bob, .. } | FishingState::Hooked { bob, .. } => Some(*bob),
            _ => None,
        };

        let (w, h) = (self.b.w, self.b.h);
        for i in 0..self.fishes.len() {
            let (fx, fy, fh) = (self.fishes[i].x, self.fishes[i].y, self.fishes[i].h);
            let mut speed_mul = 1.0f32;
            let ax = fx + fh.cos() * 48.0;
            let ay = fy + fh.sin() * 48.0;
            let margin = ax < 36.0 || ay < 36.0 || ax > w - 36.0 || ay > h - TASKBAR - 36.0;
            let mut new_h = fh;
            if self.b.cur_s(ax, ay) < 34.0 || margin {
                let mut gx = self.b.cur_s(fx + 10.0, fy) - self.b.cur_s(fx - 10.0, fy);
                let mut gy = self.b.cur_s(fx, fy + 10.0) - self.b.cur_s(fx, fy - 10.0);
                if margin {
                    gx += (w * 0.5 - fx) * 0.025;
                    gy += ((h - TASKBAR) * 0.5 - fy) * 0.025;
                }
                new_h = turn_toward(fh, gy.atan2(gx), 1.8 * dt);
            } else if let Some((bx, by)) = lure_target {
                let dist = (bx - fx).hypot(by - fy);
                if i == 0 || dist < 240.0 {
                    let desired = if dist > 18.0 {
                        (by - fy).atan2(bx - fx)
                    } else {
                        (by - fy).atan2(bx - fx) + 1.2
                    };
                    new_h = turn_toward(fh, desired, 2.2 * dt);
                    speed_mul = if dist > 28.0 { 1.55 } else { 0.75 };
                } else {
                    self.fishes[i].wander += dt * 0.4;
                    let wv = self.b.noise.vnoise(self.fishes[i].wander as f64, i as f64 * 9.1, 0) as f32;
                    new_h += (wv - 0.5) * 0.5 * dt;
                }
            } else {
                self.fishes[i].wander += dt * 0.4;
                let wv = self.b.noise.vnoise(self.fishes[i].wander as f64, i as f64 * 9.1, 0) as f64 as f32;
                new_h += (wv - 0.5) * 0.5 * dt;
            }
            let f = &mut self.fishes[i];
            f.h = new_h;
            f.ph = (f.ph + dt * 5.2 * speed_mul) % (PI * 2.0);
            let v = f.v * speed_mul;
            f.x += f.h.cos() * v * dt;
            f.y += f.h.sin() * v * dt;
            if self.b.cur_s(f.x, f.y) < 12.0 {
                if let Some(nf) = self.spawn_fish(i) {
                    self.fishes[i] = nf;
                }
            }
        }
    }

    fn make_sky_cloud(&mut self, x: f32) -> SkyCloud {
        let r_big = 95.0 * 1.15;
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
        let s = 0.62 / 1.15;
        let rm = r_big * s;
        let gb = 86.0;
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
        SkyCloud { x, y, circles, v, grow: 0.0, hw: (-x0).max(x1) + 20.0, dying: false }
    }

    fn step_sky_clouds(&mut self, dt: f32) {
        self.flow_t += dt * self.p.flow;
        let want = self.p.clouds.round().clamp(0.0, 8.0) as usize;
        let alive: Vec<usize> = (0..self.sky.len()).filter(|&i| !self.sky[i].dying).collect();
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
            self.sky[i].x += flow * 10.0 * self.sky[i].v * dt;
            self.sky[i].grow = if self.sky[i].dying { self.sky[i].grow - dt * 0.5 } else { (self.sky[i].grow + dt * 0.5).min(1.0) };
            if self.sky[i].grow <= 0.0 {
                self.sky.remove(i);
                continue;
            }
            if self.sky[i].x - self.sky[i].hw > w + 50.0 {
                let hw = self.sky[i].hw;
                let mut n = self.make_sky_cloud(-hw - 50.0);
                n.grow = 1.0;
                self.sky[i] = n;
            }
        }
    }

    fn upload_circles(&mut self, gl: &glow::Context) {
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
        let ft = self.flow_t;
        let wob_x = |x: f32, y: f32, r: f32, ph: f32| (4.5 + 0.12 * r) * (0.85 * (ft * 0.24 + x * 0.011 + y * 0.004).sin() + 0.3 * (ft * 0.3 + ph * 6.283).sin());
        let wob_y = |x: f32, y: f32, r: f32, ph: f32| (4.5 + 0.12 * r) * (0.65 * (ft * 0.2 + y * 0.013 - x * 0.006 + 1.7).sin() + 0.25 * (ft * 0.28 + ph * 6.283).cos());
        let mut list: Vec<f32> = Vec::with_capacity(256);
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

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        self.upload_circles(gl);

        let mut rip = [0f32; 24];
        for (i, r) in self.ripples.iter().take(6).enumerate() {
            rip[i * 4..i * 4 + 4].copy_from_slice(r);
        }
        let mut fish = [0f32; 16];
        for (i, f) in self.fishes.iter().take(4).enumerate() {
            fish[i * 4..i * 4 + 4].copy_from_slice(&[f.x, f.y, f.h, f.ph + f.koi * 100.0]);
        }
        // uRod[0] = (from_x, from_y, cur_x, cur_y), uRod[1] = (mode, progress, start_x, start_y)
        // mode: 0 = ẩn, 1 = đang quăng, 2 = đang chờ, 3 = cá cắn câu!, 4 = giật cá Koi nhảy lên!, 5 = thu cần không cá
        let rod = match &self.fishing {
            FishingState::Idle => [0f32; 8],
            FishingState::Casting { from, to, age, dur } => {
                let k = (*age / *dur).clamp(0.0, 1.0);
                let ease = 1.0 - (1.0 - k) * (1.0 - k);
                let cx = from.0 + (to.0 - from.0) * ease;
                let cy = from.1 + (to.1 - from.1) * ease - (k * PI).sin() * 42.0;
                [from.0, from.1, cx, cy, 1.0, k, to.0, to.1]
            }
            FishingState::Waiting { from, bob, age, .. } => {
                [from.0, from.1, bob.0, bob.1, 2.0, *age, bob.0, bob.1]
            }
            FishingState::Hooked { from, bob, age, .. } => {
                [from.0, from.1, bob.0, bob.1, 3.0, *age, bob.0, bob.1]
            }
            FishingState::Catching { from, start, age, dur } => {
                let k = (*age / *dur).clamp(0.0, 1.0);
                let ease = k * k * (3.0 - 2.0 * k);
                let cx = start.0 + (from.0 - start.0) * ease;
                let cy = start.1 + (from.1 - start.1) * ease - (k * PI).sin() * 54.0;
                [from.0, from.1, cx, cy, 4.0, k, start.0, start.1]
            }
            FishingState::ReelingEmpty { from, start, age, dur } => {
                let k = (*age / *dur).clamp(0.0, 1.0);
                let cx = start.0 + (from.0 - start.0) * k;
                let cy = start.1 + (from.1 - start.1) * k - (k * PI).sin() * 18.0;
                [from.0, from.1, cx, cy, 5.0, k, start.0, start.1]
            }
        };

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
        p.f1(gl, "uShoal", self.p.shoal);
        p.f1(gl, "uDepth", self.p.depth);
        p.f1(gl, "uFlow", self.p.flow);
        p.f4v(gl, "uRip", &rip);
        p.f1(gl, "uFishN", self.fishes.len() as f32);
        p.f4v(gl, "uFish", &fish);
        p.f4v(gl, "uRod", &rod);
        unsafe { gl.draw_arrays(glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for Lake {
    fn set_icons(&mut self, gl: &glow::Context, icons: Vec<IconBox>, first: bool, now: f64) {
        self.b.icons = icons;
        self.commit(gl, first, now);
        if first {
            self.b.cur_mix = 1.0;
            self.fishes.clear();
            self.sync_fishes();
            self.b.cur_mix = 0.0;
        }
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, now: f64) {
        self.b.update_mix(now);
        if self.fishes.is_empty() {
            self.sync_fishes();
        }
        self.step_sky_clouds(dt);
        self.step_fishes_and_fishing(dt);
        self.draw(gl, sh, now);
    }

    fn settle(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        self.b.settle();
        self.draw(gl, sh, now);
    }

    fn handle_click(&mut self, _gl: &glow::Context, x: f32, y: f32, _now: f64) {
        // Nếu cá đang cắn câu: click bất kỳ đâu trên vùng trống sẽ giật cần bắt cá ngay!
        if let FishingState::Hooked { from, bob, .. } = self.fishing {
            self.push_ripple(bob.0, bob.1, 1.45);
            self.fishing = FishingState::Catching { from, start: bob, age: 0.0, dur: 1.25 };
            return;
        }
        // Chống nháy đúp vô tình ngay lúc vừa mới quăng dây
        if let FishingState::Casting { age, .. } = self.fishing {
            if age < 0.22 {
                return;
            }
        }
        let s = self.b.cur_s(x, y);
        if s > 10.0 {
            // Đang chờ cá mà bấm vào sát phao -> thu cần nghỉ câu
            if let FishingState::Waiting { from, bob, .. } = self.fishing {
                if (bob.0 - x).hypot(bob.1 - y) < 42.0 {
                    self.push_ripple(bob.0, bob.1, 0.7);
                    self.fishing = FishingState::ReelingEmpty { from, start: bob, age: 0.0, dur: 0.5 };
                    return;
                }
            }
            let from = self.find_rod_anchor(x, y);
            self.fishing = FishingState::Casting { from, to: (x, y), age: 0.0, dur: 0.36 };
        } else {
            // Nhấn trên bờ cỏ: búng viên sỏi nhỏ nhảy ra mép nước gần nhất
            let gx = self.b.cur_s(x + 12.0, y) - self.b.cur_s(x - 12.0, y);
            let gy = self.b.cur_s(x, y + 12.0) - self.b.cur_s(x, y - 12.0);
            let len = gx.hypot(gy).max(0.001);
            let (dx, dy) = (gx / len, gy / len);
            for step in 1..=20 {
                let px = x + dx * (step as f32 * 14.0);
                let py = y + dy * (step as f32 * 14.0);
                if self.b.cur_s(px, py) > 18.0 {
                    self.push_ripple(px, py, 0.9);
                    break;
                }
            }
        }
    }

    fn is_interacting(&self, _now: f64) -> bool {
        !self.ripples.is_empty() || !matches!(self.fishing, FishingState::Idle)
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
