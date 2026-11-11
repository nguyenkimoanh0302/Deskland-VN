//! Chủ đề Rừng mưa nhiệt đới (rainforest.html): tán rừng mưa nhiều tầng nhìn từ trên cao,
//! khoảng trống rêu ẩm quanh biểu tượng, hạt mưa rơi xiên, sương mù, chim nhiệt đới và đom đóm.

use crate::engine::*;
use glow::HasContext;
use std::f32::consts::PI;

pub const FRAG: &str = include_str!("../../assets/shaders/rainforest.frag");

pub struct Params {
    pub sigma: f32,
    pub warp: f32,
    pub rain: f32,
    pub mist: f32,
    pub wind: f32,
    pub birds: f32,
    pub fireflies: f32,
}

impl Params {
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        Params {
            sigma: g("sigma", 80.0),
            warp: g("warp", 32.0),
            rain: g("rain", 0.65),
            mist: g("mist", 0.40),
            wind: g("wind", 1.0),
            birds: g("birds", 3.0),
            fireflies: g("fireflies", 6.0),
        }
    }
}

struct RainforestBird {
    x: f32,
    y: f32,
    a: f32,
    sp: f32,
    turn: f32,
    ph: f32,
    timer: f32,
    boost: f32,
}

struct Firefly {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    ph: f32,
    alpha: f32,
}

pub struct Rainforest {
    b: Base,
    p: Params,
    rng: Rng,
    birds: Vec<RainforestBird>,
    flies: Vec<Firefly>,
    ripples: Vec<[f32; 4]>,
    attract: Option<(f32, f32, f32)>,
    t0: f64,
}

impl Rainforest {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = Rainforest {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            birds: Vec::new(),
            flies: Vec::new(),
            ripples: Vec::new(),
            attract: None,
            t0: now,
        };
        s.commit(gl, true, now);
        s.step_birds(0.0);
        s.step_flies(0.0);
        s
    }

    fn commit(&mut self, gl: &glow::Context, from_empty: bool, now: f64) {
        let rects = icon_rects(&self.b.icons);
        self.b.commit(gl, &rects, self.p.sigma, self.p.warp, from_empty, now);
    }

    fn spawn_bird(&mut self) -> RainforestBird {
        let a = self.rng.next() * PI * 2.0;
        RainforestBird {
            x: 80.0 + self.rng.next() * 100.0f32.max(self.b.w - 160.0),
            y: 80.0 + self.rng.next() * 100.0f32.max(self.b.h - TASKBAR - 160.0),
            a,
            sp: 22.0 + self.rng.next() * 14.0,
            turn: (self.rng.next() - 0.5) * 0.25,
            ph: self.rng.next() * 6.28,
            timer: 2.0 + self.rng.next() * 4.0,
            boost: 0.0,
        }
    }

    fn step_birds(&mut self, dt: f32) {
        let want = self.p.birds.round().clamp(0.0, 8.0) as usize;
        self.birds.truncate(want);
        while self.birds.len() < want {
            let b = self.spawn_bird();
            self.birds.push(b);
        }
        let (w, h2, m) = (self.b.w, self.b.h - TASKBAR, 110.0);
        for i in 0..self.birds.len() {
            let r1 = self.rng.next();
            let r2 = self.rng.next();
            let b = &mut self.birds[i];
            b.boost = (b.boost - dt * 0.55).max(0.0);
            b.timer -= dt;
            if b.timer <= 0.0 {
                b.turn = (r1 - 0.5) * 0.55;
                b.timer = 2.5 + r2 * 4.0;
            }
            if b.x < m {
                b.turn += if b.a.cos() < 0.0 { 0.8 } else { 0.3 } * dt;
            }
            if b.x > w - m {
                b.turn -= if b.a.cos() > 0.0 { 0.8 } else { 0.3 } * dt;
            }
            if b.y < m {
                b.turn += if b.a.sin() < 0.0 { 0.8 } else { 0.3 } * dt;
            }
            if b.y > h2 - m {
                b.turn -= if b.a.sin() > 0.0 { 0.8 } else { 0.3 } * dt;
            }
            let mult = 1.0 + 0.7 * b.boost;
            b.a += b.turn * dt;
            b.x += b.a.cos() * b.sp * mult * dt;
            b.y += b.a.sin() * b.sp * mult * dt;
            b.ph += dt * (7.5 + 5.5 * b.boost);
        }
    }

    fn spawn_fly(&mut self) -> Firefly {
        Firefly {
            x: 40.0 + self.rng.next() * 100.0f32.max(self.b.w - 80.0),
            y: 40.0 + self.rng.next() * 100.0f32.max(self.b.h - TASKBAR - 80.0),
            vx: (self.rng.next() - 0.5) * 14.0,
            vy: (self.rng.next() - 0.5) * 14.0,
            ph: self.rng.next() * 6.28,
            alpha: 0.7 + self.rng.next() * 0.3,
        }
    }

    fn step_flies(&mut self, dt: f32) {
        let base_want = self.p.fireflies.round().clamp(0.0, 15.0) as usize;
        let want = if self.attract.is_some() { base_want.max(12) } else { base_want };
        self.flies.truncate(want);
        while self.flies.len() < want {
            let mut f = self.spawn_fly();
            if let Some((tx, ty, _)) = self.attract {
                let ang = self.rng.next() * PI * 2.0;
                let dist = 90.0 + self.rng.next() * 160.0;
                f.x = (tx + ang.cos() * dist).clamp(24.0, self.b.w - 24.0);
                f.y = (ty + ang.sin() * dist).clamp(24.0, (self.b.h - TASKBAR - 24.0).max(40.0));
            }
            self.flies.push(f);
        }
        let (w, h2) = (self.b.w, self.b.h - TASKBAR);
        let attract = self.attract;
        for i in 0..self.flies.len() {
            let rx = self.rng.next();
            let ry = self.rng.next();
            let f = &mut self.flies[i];
            if let Some((tx, ty, _)) = attract {
                let dx = tx - f.x;
                let dy = ty - f.y;
                let dist = dx.hypot(dy).max(1.0);
                let pull = if dist > 36.0 { 115.0 } else { -28.0 };
                let swirl = if i % 2 == 0 { 1.0 } else { -1.0 };
                f.vx += ((dx / dist) * pull + (-dy / dist) * 42.0 * swirl + (rx - 0.5) * 26.0) * dt;
                f.vy += ((dy / dist) * pull + (dx / dist) * 42.0 * swirl + (ry - 0.5) * 26.0) * dt;
                f.vx *= (1.0 - 1.4 * dt).max(0.0);
                f.vy *= (1.0 - 1.4 * dt).max(0.0);
                let sp = f.vx.hypot(f.vy);
                if sp > 88.0 {
                    f.vx = (f.vx / sp) * 88.0;
                    f.vy = (f.vy / sp) * 88.0;
                }
            } else {
                f.vx += (rx - 0.5) * 18.0 * dt;
                f.vy += (ry - 0.5) * 18.0 * dt;
                let sp = f.vx.hypot(f.vy);
                if sp > 18.0 {
                    f.vx = (f.vx / sp) * 18.0;
                    f.vy = (f.vy / sp) * 18.0;
                }
            }
            f.x += f.vx * dt;
            f.y += f.vy * dt;
            if f.x < 20.0 {
                f.vx = f.vx.abs();
            }
            if f.x > w - 20.0 {
                f.vx = -f.vx.abs();
            }
            if f.y < 20.0 {
                f.vy = f.vy.abs();
            }
            if f.y > h2 - 20.0 {
                f.vy = -f.vy.abs();
            }
        }
    }

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        let mut bird = [0f32; 32];
        for (i, b) in self.birds.iter().enumerate() {
            bird[i * 4..i * 4 + 4].copy_from_slice(&[b.x, b.y, b.a, b.ph]);
        }
        let mut fly = [0f32; 60];
        for (i, f) in self.flies.iter().enumerate() {
            let glow_mult = if let Some((tx, ty, ttl)) = self.attract {
                let d = (f.x - tx).hypot(f.y - ty);
                1.0 + 0.85 * (1.0 - (d / 260.0).min(1.0)) * (ttl / 1.2).min(1.0)
            } else {
                1.0
            };
            fly[i * 4..i * 4 + 4].copy_from_slice(&[f.x, f.y, f.ph, f.alpha * glow_mult]);
        }
        let mut rip = [0f32; 24];
        for (i, r) in self.ripples.iter().take(6).enumerate() {
            rip[i * 4..i * 4 + 4].copy_from_slice(r);
        }
        let p = &mut sh.prog;
        unsafe { gl.use_program(Some(p.prog)) };
        self.b.bind_common(gl, p, sh.noise_tex, sh.seed, t);
        p.f1(gl, "uRain", self.p.rain);
        p.f1(gl, "uMist", self.p.mist);
        p.f1(gl, "uWind", self.p.wind);
        p.f1(gl, "uBirdN", self.birds.len() as f32);
        p.f4v(gl, "uBird", &bird);
        p.f1(gl, "uFlyN", self.flies.len() as f32);
        p.f4v(gl, "uFly", &fly);
        p.f4v(gl, "uRip", &rip);
        unsafe { gl.draw_arrays(glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for Rainforest {
    fn set_icons(&mut self, gl: &glow::Context, icons: Vec<IconBox>, first: bool, now: f64) {
        self.b.icons = icons;
        self.commit(gl, first, now);
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, now: f64) {
        self.b.update_mix(now);
        self.ripples.retain_mut(|r| {
            r[2] += dt;
            r[2] < 2.8
        });
        if let Some((_, _, ttl)) = &mut self.attract {
            *ttl -= dt;
            if *ttl <= 0.0 {
                self.attract = None;
            }
        }
        self.step_birds(dt);
        self.step_flies(dt);
        self.draw(gl, sh, now);
    }

    fn settle(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        self.b.settle();
        self.draw(gl, sh, now);
    }

    fn handle_click(&mut self, _gl: &glow::Context, x: f32, y: f32, _now: f64) {
        if self.ripples.len() >= 6 {
            self.ripples.remove(0);
        }
        self.ripples.push([x, y, 0.0, 1.35]);
        self.attract = Some((x, y, 4.5));

        for b in &mut self.birds {
            if (b.x - x).hypot(b.y - y) < 520.0 {
                let target = (y - b.y).atan2(x - b.x) + (self.rng.next() - 0.5) * 0.45;
                let mut d = target - b.a;
                d = d.sin().atan2(d.cos());
                b.a += d * 0.75;
                b.boost = 1.0;
                b.timer = 2.0;
            }
        }
    }

    fn is_interacting(&self, _now: f64) -> bool {
        !self.ripples.is_empty() || self.attract.is_some()
    }

    fn destroy(&mut self, gl: &glow::Context) {
        self.b.destroy(gl);
    }

    fn field_ms(&self) -> f64 {
        self.b.field_ms
    }
}
