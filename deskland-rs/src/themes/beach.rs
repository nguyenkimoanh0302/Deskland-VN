//! Chủ đề Bãi biển (flat.html): bờ biển uốn quanh biểu tượng, sóng vỗ, thuyền kayak và hải âu.
//! Chưa port: cây dừa (beach-palms.js).

use crate::engine::*;
use glow::HasContext;
use std::f32::consts::PI;

pub const FRAG: &str = include_str!("../../assets/shaders/beach.frag");

pub struct Params {
    pub sigma: f32,
    pub warp: f32,
    pub shallow: f32,
    pub shoal: f32,
    pub speed: f32,
    pub runup: f32,
    pub wave: f32,
    pub spacing: f32,
    pub waves: f32,
    pub grain: f32,
    pub kayaks: f32,
    pub birds: f32,
}

impl Params {
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        Params {
            sigma: g("sigma", 85.0),
            warp: g("warp", 34.0),
            shallow: g("shallow", 260.0),
            shoal: g("shoal", 110.0),
            speed: 22.0,
            runup: 120.0,
            wave: 1.6,
            spacing: 70.0,
            waves: 1.0,
            grain: 1.0,
            kayaks: g("kayaks", 2.0),
            birds: g("birds", 3.0),
        }
    }
}

struct Kayak {
    x: f32,
    y: f32,
    h: f32,
    v: f32,
    ph: f32,
    wander: f32,
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

pub struct Beach {
    b: Base,
    p: Params,
    rng: Rng,
    kayaks: Vec<Kayak>,
    birds: Vec<Bird>,
    t0: f64,
}

fn turn_toward(h: f32, target: f32, rate: f32) -> f32 {
    let mut d = target - h;
    d = d.sin().atan2(d.cos());
    h + d.signum() * d.abs().min(rate)
}

impl Beach {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = Beach { b: Base::new(gl, w, h, buf_w, buf_h, scale, seed), p, rng: Rng::new(rng_seed), kayaks: Vec::new(), birds: Vec::new(), t0: now };
        // startHostedMode(): icons = []; commit(true); syncBirds(false)
        s.commit(gl, true, now);
        s.sync_birds(false);
        s
    }

    fn commit(&mut self, gl: &glow::Context, from_empty: bool, now: f64) {
        let rects = icon_rects(&self.b.icons);
        self.b.commit(gl, &rects, self.p.sigma, self.p.warp, from_empty, now);
    }

    fn spawn_kayak(&mut self) -> Option<Kayak> {
        let (w, h) = (self.b.w, self.b.h);
        for _ in 0..400 {
            let x = 60.0 + self.rng.next() * (w - 120.0);
            let y = 60.0 + self.rng.next() * (h - TASKBAR - 120.0);
            if self.b.cur_s(x, y) > 170.0 && self.kayaks.iter().all(|k| (k.x - x).hypot(k.y - y) > 160.0) {
                return Some(Kayak {
                    x,
                    y,
                    h: self.rng.next() * PI * 2.0,
                    v: 7.0 + self.rng.next() * 4.0,
                    ph: self.rng.next() * 6.0,
                    wander: self.rng.next() * 100.0,
                });
            }
        }
        None
    }

    fn sync_kayaks(&mut self) {
        let want = self.p.kayaks.round().clamp(0.0, 4.0) as usize;
        self.kayaks.truncate(want);
        while self.kayaks.len() < want {
            match self.spawn_kayak() {
                Some(k) => self.kayaks.push(k),
                None => break,
            }
        }
    }

    fn step_kayaks(&mut self, dt: f32) {
        let clearance = 64.0f32;
        let (w, h) = (self.b.w, self.b.h);
        let n = self.kayaks.len();
        let mut plans: Vec<(f32, f32, f32)> = Vec::with_capacity(n);
        for i in 0..n {
            let (kx, ky, kh, kv) = (self.kayaks[i].x, self.kayaks[i].y, self.kayaks[i].h, self.kayaks[i].v);
            let mut heading = kh;
            let ax = kx + kh.cos() * 70.0;
            let ay = ky + kh.sin() * 70.0;
            let margin = ax < 40.0 || ay < 40.0 || ax > w - 40.0 || ay > h - TASKBAR - 40.0;
            if self.b.cur_s(ax, ay) < 150.0 || margin {
                let mut gx = self.b.cur_s(kx + 10.0, ky) - self.b.cur_s(kx - 10.0, ky);
                let mut gy = self.b.cur_s(kx, ky + 10.0) - self.b.cur_s(kx, ky - 10.0);
                if margin {
                    gx += (w / 2.0 - kx) * 0.02;
                    gy += ((h - TASKBAR) / 2.0 - ky) * 0.02;
                }
                let mut d = gy.atan2(gx) - kh;
                d = d.sin().atan2(d.cos());
                heading += d.signum() * d.abs().min(0.7 * dt);
            } else {
                self.kayaks[i].wander += dt * 0.3;
                let wv = self.b.noise.vnoise(self.kayaks[i].wander as f64, (i * 10) as f64, 0) as f32;
                heading += (wv - 0.5) * 0.25 * dt;
            }
            let (fx, fy) = (kh.cos(), kh.sin());
            let (mut steer_x, mut steer_y, mut speed, mut avoiding) = (heading.cos(), heading.sin(), 1.0f32, false);
            for j in 0..n {
                if i == j {
                    continue;
                }
                let o = &self.kayaks[j];
                let (dx, dy) = (o.x - kx, o.y - ky);
                let distance = dx.hypot(dy);
                if distance > 210.0 {
                    continue;
                }
                let rvx = o.h.cos() * o.v - fx * kv;
                let rvy = o.h.sin() * o.v - fy * kv;
                let closing = dx * rvx + dy * rvy;
                let ahead = (-closing / (rvx * rvx + rvy * rvy).max(0.01)).clamp(0.0, 7.0);
                let closest = (dx + rvx * ahead).hypot(dy + rvy * ahead);
                let a = ((110.0 - distance) / 46.0).min(1.0);
                let b = if closing < 0.0 { (1.0 - (closest / 90.0).min(1.0)) * (1.0 - ahead / 8.0) } else { 0.0 };
                let risk = 0f32.max(a).max(b);
                if risk <= 0.0 {
                    continue;
                }
                avoiding = true;
                let away_x = if distance > 0.001 { -dx / distance } else { -fy };
                let away_y = if distance > 0.001 { -dy / distance } else { fx };
                steer_x += (away_x * 2.0 - fy * 1.6) * risk;
                steer_y += (away_y * 2.0 + fx * 1.6) * risk;
                if dx * fx + dy * fy > 0.0 && closing < 0.0 {
                    speed = speed.min(0.25 + 0.75 * ((distance - clearance) / 50.0).clamp(0.0, 1.0));
                }
            }
            if avoiding {
                let t = steer_y.atan2(steer_x) - kh;
                let d = t.sin().atan2(t.cos());
                heading = kh + d.signum() * d.abs().min(dt);
            }
            let k = &mut self.kayaks[i];
            k.ph += dt * 2.4;
            let v = k.v * (0.75 + 0.25 * k.ph.sin().abs()) * speed;
            plans.push((heading, heading.cos() * v * dt, heading.sin() * v * dt));
        }
        for _ in 0..n {
            for i in 0..n {
                for j in i + 1..n {
                    let dx = self.kayaks[j].x - self.kayaks[i].x;
                    let dy = self.kayaks[j].y - self.kayaks[i].y;
                    let (a, b) = (plans[i], plans[j]);
                    let (vx, vy) = (b.1 - a.1, b.2 - a.2);
                    let vv = vx * vx + vy * vy;
                    let dot = dx * vx + dy * vy;
                    if dot >= 0.0 || vv < 1e-12 {
                        continue;
                    }
                    let closest_at = (-dot / vv).clamp(0.0, 1.0);
                    if (dx + vx * closest_at).hypot(dy + vy * closest_at) >= clearance {
                        continue;
                    }
                    let gap = dx * dx + dy * dy - clearance * clearance;
                    let fraction = if gap <= 0.0 { 0.0 } else { (gap / (-dot + (dot * dot - vv * gap).max(0.0).sqrt()) - 0.001).clamp(0.0, 1.0) };
                    plans[i].1 *= fraction;
                    plans[i].2 *= fraction;
                    plans[j].1 *= fraction;
                    plans[j].2 *= fraction;
                }
            }
        }
        let mut i = n;
        while i > 0 {
            i -= 1;
            let (ph, pdx, pdy) = plans[i];
            let k = &mut self.kayaks[i];
            k.h = ph;
            k.x += pdx;
            k.y += pdy;
            let (x, y) = (k.x, k.y);
            if self.b.cur_s(x, y) < 40.0 {
                match self.spawn_kayak() {
                    Some(nk) => self.kayaks[i] = nk,
                    None => {
                        self.kayaks.remove(i);
                    }
                }
            }
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

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        let rip = [0f32; 24];
        let mut kay = [0f32; 16];
        for (i, k) in self.kayaks.iter().enumerate() {
            kay[i * 4..i * 4 + 4].copy_from_slice(&[k.x, k.y, k.h, k.ph]);
        }
        let mut bird = [0f32; 32];
        for (i, b) in self.birds.iter().enumerate() {
            bird[i * 4..i * 4 + 4].copy_from_slice(&[b.x, b.y, b.h, 0.68 + 0.32 * b.ph.cos()]);
        }
        let p = &mut sh.prog;
        unsafe { gl.use_program(Some(p.prog)) };
        self.b.bind_common(gl, p, sh.noise_tex, sh.seed, t);
        let q = &self.p;
        p.f1(gl, "uSpeed", q.speed);
        p.f1(gl, "uRunup", q.runup);
        p.f1(gl, "uShallow", q.shallow);
        p.f1(gl, "uShoal", q.shoal);
        p.f1(gl, "uGrain", q.grain);
        p.f1(gl, "uWave", q.wave);
        p.f1(gl, "uSigma", q.sigma);
        p.f1(gl, "uSpacing", q.spacing);
        p.f1(gl, "uWaves", q.waves);
        p.f1(gl, "uKayN", self.kayaks.len() as f32);
        p.f4v(gl, "uKay", &kay);
        p.f4v(gl, "uRip", &rip);
        p.f1(gl, "uBirdN", self.birds.len() as f32);
        p.f4v(gl, "uBird", &bird);
        unsafe { gl.draw_arrays(glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for Beach {
    fn set_icons(&mut self, gl: &glow::Context, icons: Vec<IconBox>, first: bool, now: f64) {
        self.b.icons = icons;
        self.commit(gl, first, now);
        if first {
            self.b.cur_mix = 1.0;
            self.kayaks.clear();
            self.sync_kayaks();
            self.b.cur_mix = 0.0;
        }
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, now: f64) {
        self.b.update_mix(now);
        self.step_kayaks(dt);
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
