//! Chủ đề Đảo Maldives (maldives.html / beach-turquoise): đảo cát trắng, rạn san hô,
//! nước xanh ngọc lam, đàn cá, rùa biển, thuyền kayak và hải âu.

use crate::engine::*;
use glow::HasContext;
use std::f32::consts::PI;

pub const FRAG: &str = include_str!("../../assets/shaders/maldives.frag");

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
    pub fish_schools: f32,
    pub turtles: f32,
    pub kayaks: f32,
    pub birds: f32,
}

impl Params {
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        Params {
            sigma: g("sigma", 85.0),
            warp: g("warp", 34.0),
            shallow: g("shallow", 280.0),
            shoal: g("shoal", 120.0),
            speed: g("speed", 21.0),
            runup: g("runup", 115.0),
            wave: g("wave", 1.5),
            spacing: g("spacing", 72.0),
            waves: g("waves", 1.0),
            grain: g("grain", 1.0),
            fish_schools: g("fishSchools", 3.0),
            turtles: g("turtles", 2.0),
            kayaks: g("kayaks", 1.0),
            birds: g("birds", 3.0),
        }
    }
}

struct WaterActor {
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

pub struct Maldives {
    b: Base,
    p: Params,
    rng: Rng,
    kayaks: Vec<WaterActor>,
    fish_schools: Vec<WaterActor>,
    turtles: Vec<WaterActor>,
    birds: Vec<Bird>,
    t0: f64,
}

fn turn_toward(h: f32, target: f32, rate: f32) -> f32 {
    let mut d = target - h;
    d = d.sin().atan2(d.cos());
    h + d.signum() * d.abs().min(rate)
}

impl Maldives {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = Maldives {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            kayaks: Vec::new(),
            fish_schools: Vec::new(),
            turtles: Vec::new(),
            birds: Vec::new(),
            t0: now,
        };
        s.commit(gl, true, now);
        s.sync_birds(false);
        s
    }

    fn commit(&mut self, gl: &glow::Context, from_empty: bool, now: f64) {
        let rects = icon_rects(&self.b.icons);
        self.b.commit(gl, &rects, self.p.sigma, self.p.warp, from_empty, now);
    }

    fn spawn_water_actor(b: &Base, rng: &mut Rng, list: &[WaterActor], min_depth: f32, min_dist: f32, v0: f32, dv: f32) -> Option<WaterActor> {
        let (w, h) = (b.w, b.h);
        for _ in 0..400 {
            let x = 60.0 + rng.next() * (w - 120.0);
            let y = 60.0 + rng.next() * (h - TASKBAR - 120.0);
            if b.cur_s(x, y) > min_depth && list.iter().all(|k| (k.x - x).hypot(k.y - y) > min_dist) {
                return Some(WaterActor {
                    x,
                    y,
                    h: rng.next() * PI * 2.0,
                    v: v0 + rng.next() * dv,
                    ph: rng.next() * 6.0,
                    wander: rng.next() * 100.0,
                });
            }
        }
        None
    }

    fn sync_kayaks(&mut self) {
        let want = self.p.kayaks.round().clamp(0.0, 4.0) as usize;
        self.kayaks.truncate(want);
        while self.kayaks.len() < want {
            match Self::spawn_water_actor(&self.b, &mut self.rng, &self.kayaks, 170.0, 160.0, 7.0, 4.0) {
                Some(k) => self.kayaks.push(k),
                None => break,
            }
        }
    }

    fn sync_fish(&mut self) {
        let want = self.p.fish_schools.round().clamp(0.0, 6.0) as usize;
        self.fish_schools.truncate(want);
        while self.fish_schools.len() < want {
            match Self::spawn_water_actor(&self.b, &mut self.rng, &self.fish_schools, 95.0, 130.0, 10.0, 6.0) {
                Some(f) => self.fish_schools.push(f),
                None => break,
            }
        }
    }

    fn sync_turtles(&mut self) {
        let want = self.p.turtles.round().clamp(0.0, 4.0) as usize;
        self.turtles.truncate(want);
        while self.turtles.len() < want {
            match Self::spawn_water_actor(&self.b, &mut self.rng, &self.turtles, 80.0, 120.0, 4.5, 2.5) {
                Some(t) => self.turtles.push(t),
                None => break,
            }
        }
    }

    fn step_water_list(b: &Base, rng: &mut Rng, list: &mut Vec<WaterActor>, dt: f32, min_depth: f32, ph_rate: f32, sp_depth: f32, sp_dist: f32, v0: f32, dv: f32) {
        let (w, h) = (b.w, b.h);
        let mut i = list.len();
        while i > 0 {
            i -= 1;
            let k = &mut list[i];
            let ax = k.x + k.h.cos() * 65.0;
            let ay = k.y + k.h.sin() * 65.0;
            let margin = ax < 45.0 || ay < 45.0 || ax > w - 45.0 || ay > h - TASKBAR - 45.0;
            if b.cur_s(ax, ay) < min_depth || margin {
                let mut gx = b.cur_s(k.x + 10.0, k.y) - b.cur_s(k.x - 10.0, k.y);
                let mut gy = b.cur_s(k.x, k.y + 10.0) - b.cur_s(k.x, k.y - 10.0);
                if margin {
                    gx += (w / 2.0 - k.x) * 0.02;
                    gy += ((h - TASKBAR) / 2.0 - k.y) * 0.02;
                }
                let mut d = gy.atan2(gx) - k.h;
                d = d.sin().atan2(d.cos());
                k.h += d.signum() * d.abs().min(0.8 * dt);
            } else {
                k.wander += dt * 0.35;
                let wv = b.noise.vnoise(k.wander as f64, (i * 10) as f64, 0) as f32;
                k.h += (wv - 0.5) * 0.35 * dt;
            }
            k.ph += dt * ph_rate;
            let v = k.v * (0.8 + 0.2 * (k.ph * 0.5).sin().abs());
            k.x += k.h.cos() * v * dt;
            k.y += k.h.sin() * v * dt;
            let (kx, ky) = (k.x, k.y);
            if b.cur_s(kx, ky) < 30.0 {
                match Self::spawn_water_actor(b, rng, list, sp_depth, sp_dist, v0, dv) {
                    Some(n) => list[i] = n,
                    None => {
                        list.remove(i);
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
        let mut fish = [0f32; 24];
        for (i, f) in self.fish_schools.iter().enumerate() {
            fish[i * 4..i * 4 + 4].copy_from_slice(&[f.x, f.y, f.h, f.ph]);
        }
        let mut turt = [0f32; 16];
        for (i, tr) in self.turtles.iter().enumerate() {
            turt[i * 4..i * 4 + 4].copy_from_slice(&[tr.x, tr.y, tr.h, tr.ph]);
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
        p.f1(gl, "uFishN", self.fish_schools.len() as f32);
        p.f4v(gl, "uFish", &fish);
        p.f1(gl, "uTurtN", self.turtles.len() as f32);
        p.f4v(gl, "uTurt", &turt);
        p.f4v(gl, "uRip", &rip);
        p.f1(gl, "uBirdN", self.birds.len() as f32);
        p.f4v(gl, "uBird", &bird);
        unsafe { gl.draw_arrays(glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for Maldives {
    fn set_icons(&mut self, gl: &glow::Context, icons: Vec<IconBox>, first: bool, now: f64) {
        self.b.icons = icons;
        self.commit(gl, first, now);
        if first {
            self.b.cur_mix = 1.0;
            self.kayaks.clear();
            self.fish_schools.clear();
            self.turtles.clear();
            self.sync_kayaks();
            self.sync_fish();
            self.sync_turtles();
            self.b.cur_mix = 0.0;
        }
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, now: f64) {
        self.b.update_mix(now);
        Self::step_water_list(&self.b, &mut self.rng, &mut self.kayaks, dt, 150.0, 2.4, 170.0, 160.0, 7.0, 4.0);
        Self::step_water_list(&self.b, &mut self.rng, &mut self.fish_schools, dt, 90.0, 4.2, 95.0, 130.0, 10.0, 6.0);
        Self::step_water_list(&self.b, &mut self.rng, &mut self.turtles, dt, 75.0, 1.8, 80.0, 120.0, 4.5, 2.5);
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
