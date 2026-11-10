//! Chủ đề Hoa anh đào Tết (tet-sakura.html): khoảng sân xuân giấy dó ấm áp bao bọc bởi rừng hoa đào / anh đào nở rộ,
//! đèn lồng đỏ, bao lì xì Tết, cánh đào bay trong gió và đàn chim én báo xuân.

use crate::engine::*;
use glow::HasContext;
use std::f32::consts::PI;

pub const FRAG: &str = include_str!("../../assets/shaders/tet_sakura.frag");

pub struct Params {
    pub adaptive: bool,
    pub sigma: f32,
    pub warp: f32,
    pub petals: f32,
    pub wind: f32,
    pub lanterns: f32,
    pub swallows: f32,
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
            adaptive: gb("adaptive", true),
            sigma: g("sigma", 80.0),
            warp: g("warp", 30.0),
            petals: g("petals", 0.65),
            wind: g("wind", 1.0),
            lanterns: g("lanterns", 4.0),
            swallows: g("swallows", 3.0),
        }
    }
}

struct Lantern {
    x: f32,
    y: f32,
    ph: f32,
    kind: f32,
}

struct Swallow {
    x: f32,
    y: f32,
    a: f32,
    sp: f32,
    turn: f32,
    ph: f32,
    timer: f32,
}

pub struct TetSakura {
    b: Base,
    p: Params,
    rng: Rng,
    lanterns: Vec<Lantern>,
    swallows: Vec<Swallow>,
    t0: f64,
}

impl TetSakura {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = TetSakura {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            lanterns: Vec::new(),
            swallows: Vec::new(),
            t0: now,
        };
        s.commit(gl, true, now);
        s.step_swallows(0.0);
        s
    }

    fn commit(&mut self, gl: &glow::Context, from_empty: bool, now: f64) {
        let rects = if self.p.adaptive { icon_rects(&self.b.icons) } else { Vec::new() };
        self.b.commit(gl, &rects, self.p.sigma, self.p.warp, from_empty, now);
        self.place_lanterns();
    }

    fn place_lanterns(&mut self) {
        self.lanterns.clear();
        if self.b.fb.is_empty() {
            return;
        }
        let want = self.p.lanterns.round().clamp(0.0, 8.0) as usize;
        let (w, h, noise) = (self.b.w, self.b.h, &self.b.noise);
        let has_icons = self.p.adaptive && !self.b.icons.is_empty();
        for n in 0..want {
            let mut best: Option<Lantern> = None;
            let mut best_score = -1e9f32;
            for k in 0..60 {
                let u = noise.hash2((n * 31 + k) as f64, 11.0, 700) as f32;
                let v = noise.hash2((n * 31 + k) as f64, 23.0, 700) as f32;
                let x = 60.0 + u * 100.0f32.max(w - 120.0);
                let y = 60.0 + v * 100.0f32.max(h - TASKBAR - 120.0);
                let s = self.b.cur_s(x, y);
                if has_icons && (s < 14.0 || s > 95.0) {
                    continue;
                }
                if !has_icons && s < 14.0 {
                    continue;
                }
                let min_dist = self.lanterns.iter().fold(1e9f32, |d, o| d.min((o.x - x).hypot(o.y - y)));
                let score = if has_icons {
                    min_dist.min(260.0) - (s - 38.0).abs() * 1.2
                } else {
                    min_dist.min(260.0)
                };
                if min_dist > 75.0 && score > best_score {
                    best_score = score;
                    best = Some(Lantern {
                        x,
                        y,
                        ph: noise.hash2(n as f64, k as f64, 710) as f32 * 6.283,
                        kind: if n % 2 == 0 { 0.0 } else { 1.0 },
                    });
                }
            }
            if let Some(ln) = best {
                self.lanterns.push(ln);
            }
        }
    }

    fn spawn_swallow(&mut self) -> Swallow {
        let a = self.rng.next() * PI * 2.0;
        Swallow {
            x: 80.0 + self.rng.next() * 100.0f32.max(self.b.w - 160.0),
            y: 80.0 + self.rng.next() * 100.0f32.max(self.b.h - TASKBAR - 160.0),
            a,
            sp: 28.0 + self.rng.next() * 16.0,
            turn: (self.rng.next() - 0.5) * 0.3,
            ph: self.rng.next() * 6.28,
            timer: 2.0 + self.rng.next() * 3.5,
        }
    }

    fn step_swallows(&mut self, dt: f32) {
        let want = self.p.swallows.round().clamp(0.0, 8.0) as usize;
        self.swallows.truncate(want);
        while self.swallows.len() < want {
            let sw = self.spawn_swallow();
            self.swallows.push(sw);
        }
        let (w, h2, m) = (self.b.w, self.b.h - TASKBAR, 110.0);
        for i in 0..self.swallows.len() {
            let r1 = self.rng.next();
            let r2 = self.rng.next();
            let b = &mut self.swallows[i];
            b.timer -= dt;
            if b.timer <= 0.0 {
                b.turn = (r1 - 0.5) * 0.6;
                b.timer = 2.0 + r2 * 3.5;
            }
            if b.x < m {
                b.turn += if b.a.cos() < 0.0 { 0.9 } else { 0.35 } * dt;
            }
            if b.x > w - m {
                b.turn -= if b.a.cos() > 0.0 { 0.9 } else { 0.35 } * dt;
            }
            if b.y < m {
                b.turn += if b.a.sin() < 0.0 { 0.9 } else { 0.35 } * dt;
            }
            if b.y > h2 - m {
                b.turn -= if b.a.sin() > 0.0 { 0.9 } else { 0.35 } * dt;
            }
            b.a += b.turn * dt;
            b.x += b.a.cos() * b.sp * dt;
            b.y += b.a.sin() * b.sp * dt;
            b.ph += dt * 8.2;
        }
    }

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        let mut lantern = [0f32; 32];
        for (i, ln) in self.lanterns.iter().enumerate() {
            lantern[i * 4..i * 4 + 4].copy_from_slice(&[ln.x, ln.y, ln.ph, ln.kind]);
        }
        let mut swallow = [0f32; 32];
        for (i, b) in self.swallows.iter().enumerate() {
            swallow[i * 4..i * 4 + 4].copy_from_slice(&[b.x, b.y, b.a, b.ph]);
        }
        let p = &mut sh.prog;
        unsafe { gl.use_program(Some(p.prog)) };
        self.b.bind_common(gl, p, sh.noise_tex, sh.seed, t);
        p.f1(gl, "uPetals", self.p.petals);
        p.f1(gl, "uWind", self.p.wind);
        p.f1(gl, "uLanternN", self.lanterns.len() as f32);
        p.f4v(gl, "uLantern", &lantern);
        p.f1(gl, "uSwallowN", self.swallows.len() as f32);
        p.f4v(gl, "uSwallow", &swallow);
        unsafe { gl.draw_arrays(glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for TetSakura {
    fn set_icons(&mut self, gl: &glow::Context, icons: Vec<IconBox>, first: bool, now: f64) {
        self.b.icons = icons;
        self.commit(gl, first, now);
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, now: f64) {
        self.b.update_mix(now);
        self.step_swallows(dt);
        self.draw(gl, sh, now);
    }

    fn settle(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        self.b.settle();
        self.draw(gl, sh, now);
    }

    fn destroy(&mut self, gl: &glow::Context) {
        self.b.destroy(gl);
    }

    fn field_ms(&self) -> f64 {
        self.b.field_ms
    }
}
