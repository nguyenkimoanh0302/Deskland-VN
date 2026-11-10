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
}

pub struct PlumBlossoms {
    b: Base,
    p: Params,
    rng: Rng,
    birds: Vec<PlumBird>,
    t0: f64,
}

impl PlumBlossoms {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = PlumBlossoms {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            birds: Vec::new(),
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
            b.a += b.turn * dt;
            b.x += b.a.cos() * b.sp * dt;
            b.y += b.a.sin() * b.sp * dt;
            b.ph += dt * 7.2;
        }
    }

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        let mut bird = [0f32; 32];
        for (i, b) in self.birds.iter().enumerate() {
            bird[i * 4..i * 4 + 4].copy_from_slice(&[b.x, b.y, b.a, b.ph]);
        }
        let p = &mut sh.prog;
        unsafe { gl.use_program(Some(p.prog)) };
        self.b.bind_common(gl, p, sh.noise_tex, sh.seed, t);
        p.f1(gl, "uBloom", self.p.bloom);
        p.f1(gl, "uPetals", self.p.petals);
        p.f1(gl, "uWind", self.p.wind);
        p.f1(gl, "uMist", self.p.mist);
        p.f1(gl, "uBirdN", self.birds.len() as f32);
        p.f4v(gl, "uBird", &bird);
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
        self.step_birds(dt);
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
