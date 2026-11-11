//! Chủ đề Hoa mận / Plum Blossoms (mocchau.html): đồi cỏ xuân màu nước bao bọc bởi rừng hoa mận trắng tinh khôi,
//! sương sớm bảng lảng, cánh hoa mận trắng bay trong gió và đàn chim rừng.

use crate::engine::*;
use glow::HasContext;
use std::f32::consts::PI;

pub const FRAG: &str = include_str!("../../assets/shaders/mocchau.frag");

pub struct Params {
    pub sigma: f32,
    pub warp: f32,
    pub bloom: f32,
    pub petals: f32,
    pub wind: f32,
    pub mist: f32,
    pub birds: f32,
}

impl Params {
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        Params {
            sigma: g("sigma", 80.0),
            warp: g("warp", 30.0),
            bloom: g("bloom", 0.80),
            petals: g("petals", 0.55),
            wind: g("wind", 1.0),
            mist: g("mist", 0.35),
            birds: g("birds", 3.0),
        }
    }
}

struct PlumBird {
    x: f32,
    y: f32,
    a: f32,
    sp: f32,
    turn: f32,
    ph: f32,
    timer: f32,
    boost: f32,
}

pub struct PlumBlossoms {
    b: Base,
    p: Params,
    rng: Rng,
    birds: Vec<PlumBird>,
    bursts: Vec<[f32; 4]>,
    shake: f32,
    t0: f64,
}

impl PlumBlossoms {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = PlumBlossoms {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            birds: Vec::new(),
            bursts: Vec::new(),
            shake: 0.0,
            t0: now,
        };
        s.commit(gl, true, now);
        s.step_birds(0.0);
        s
    }

    fn commit(&mut self, gl: &glow::Context, from_empty: bool, now: f64) {
        let rects = icon_rects(&self.b.icons);
        self.b.commit(gl, &rects, self.p.sigma, self.p.warp, from_empty, now);
    }

    fn spawn_bird(&mut self) -> PlumBird {
        let a = self.rng.next() * PI * 2.0;
        PlumBird {
            x: 80.0 + self.rng.next() * 100.0f32.max(self.b.w - 160.0),
            y: 80.0 + self.rng.next() * 100.0f32.max(self.b.h - TASKBAR - 160.0),
            a,
            sp: 20.0 + self.rng.next() * 14.0,
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
                b.turn = (r1 - 0.5) * 0.5;
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
            let mult = 1.0 + 0.65 * b.boost;
            b.a += b.turn * dt;
            b.x += b.a.cos() * b.sp * mult * dt;
            b.y += b.a.sin() * b.sp * mult * dt;
            b.ph += dt * (7.2 + 5.0 * b.boost);
        }
    }

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        let mut bird = [0f32; 32];
        for (i, b) in self.birds.iter().enumerate() {
            bird[i * 4..i * 4 + 4].copy_from_slice(&[b.x, b.y, b.a, b.ph]);
        }
        let mut burst = [0f32; 24];
        for (i, b) in self.bursts.iter().take(6).enumerate() {
            burst[i * 4..i * 4 + 4].copy_from_slice(b);
        }
        let p = &mut sh.prog;
        unsafe { gl.use_program(Some(p.prog)) };
        self.b.bind_common(gl, p, sh.noise_tex, sh.seed, t);
        p.f1(gl, "uBloom", self.p.bloom);
        p.f1(gl, "uPetals", self.p.petals);
        p.f1(gl, "uWind", self.p.wind);
        p.f1(gl, "uMist", self.p.mist);
        p.f1(gl, "uShake", self.shake);
        p.f1(gl, "uBirdN", self.birds.len() as f32);
        p.f4v(gl, "uBird", &bird);
        p.f4v(gl, "uBurst", &burst);
        unsafe { gl.draw_arrays(glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for PlumBlossoms {
    fn set_icons(&mut self, gl: &glow::Context, icons: Vec<IconBox>, first: bool, now: f64) {
        self.b.icons = icons;
        self.commit(gl, first, now);
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, now: f64) {
        self.b.update_mix(now);
        self.shake = (self.shake - dt * 0.7).max(0.0);
        self.bursts.retain_mut(|b| {
            b[2] += dt;
            b[2] < 4.5
        });
        self.step_birds(dt);
        self.draw(gl, sh, now);
    }

    fn settle(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        self.b.settle();
        self.draw(gl, sh, now);
    }

    fn handle_click(&mut self, _gl: &glow::Context, x: f32, y: f32, _now: f64) {
        self.shake = 1.0;
        if self.bursts.len() >= 6 {
            self.bursts.remove(0);
        }
        let seed = 0.15 + self.rng.next() * 0.85;
        self.bursts.push([x, y, 0.0, seed]);

        for b in &mut self.birds {
            if (b.x - x).hypot(b.y - y) < 520.0 {
                let target = (y - b.y).atan2(x - b.x) + (self.rng.next() - 0.5) * 0.4;
                let mut d = target - b.a;
                d = d.sin().atan2(d.cos());
                b.a += d * 0.75;
                b.boost = 1.0;
                b.timer = 2.2;
            }
        }
    }

    fn is_interacting(&self, _now: f64) -> bool {
        self.shake > 0.01 || !self.bursts.is_empty()
    }

    fn destroy(&mut self, gl: &glow::Context) {
        self.b.destroy(gl);
    }

    fn field_ms(&self) -> f64 {
        self.b.field_ms
    }
}
