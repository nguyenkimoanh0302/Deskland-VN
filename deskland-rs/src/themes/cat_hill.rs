//! Chủ đề Đồi mèo nắng ấm (cat-hill.html): đàn mèo đủ màu sắc nằm ngủ trên đệm hồng, phơi bụng,
//! liếm tay và đi dạo trên sườn đồi xanh mướt ngập nắng, bướm vàng bay lượn và lá xanh bay trong gió.

use crate::engine::*;
use glow::HasContext;
use std::f32::consts::PI;

pub const FRAG: &str = include_str!("../../assets/shaders/cat_hill.frag");

pub struct Params {
    pub cats: f32,
    pub speed: f32,
    pub wind: f32,
    pub leaves: f32,
    pub butterflies: f32,
}

impl Params {
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        Params {
            cats: g("cats", 8.0),
            speed: g("speed", 1.0),
            wind: g("wind", 1.0),
            leaves: g("leaves", 0.55),
            butterflies: g("butterflies", 3.0),
        }
    }
}

#[derive(Clone)]
struct Cat {
    mode: u8, // 0=curled, 1=belly, 2=groom, 3=walk
    fur_idx: u8,
    x: f32,
    y: f32,
    tx: f32,
    ty: f32,
    dir: f32,
    ph: f32,
    wait: f32,
    sc: f32,
}

struct Butterfly {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    ph: f32,
    timer: f32,
}

struct Leaf {
    x: f32,
    y: f32,
    vy: f32,
    vx: f32,
    rot: f32,
    vrot: f32,
    state: u8, // 0=air, 1=ground
    rest: f32,
    fade: f32,
    ground_y: f32,
}

pub struct CatHill {
    b: Base,
    p: Params,
    rng: Rng,
    cats: Vec<Cat>,
    butterflies: Vec<Butterfly>,
    leaves: Vec<Leaf>,
    t0: f64,
}

impl CatHill {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = CatHill {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            cats: Vec::new(),
            butterflies: Vec::new(),
            leaves: Vec::new(),
            t0: now,
        };
        s.b.commit(gl, &[], 80.0, 30.0, true, now);
        s.sync_entities();
        s
    }

    fn hill_y(&self, nx: f32) -> f32 {
        self.b.h * (0.295 - 0.024 * (nx * PI * 1.15 - 0.25).sin() + 0.010 * (nx * PI * 2.4 + 0.6).cos())
    }

    fn random_hill_pos(&mut self) -> (f32, f32) {
        let nx = 0.10 + self.rng.next() * 0.78;
        let min_y = self.hill_y(nx) + 42.0;
        let max_y = self.b.h - TASKBAR - 48.0;
        let y = min_y + self.rng.next() * 80.0f32.max(max_y - min_y);
        (nx * self.b.w, y)
    }

    fn make_cat(&mut self, idx: usize) -> Cat {
        let (w, h) = (self.b.w, self.b.h);
        let fur_idx = ((idx + (self.b.noise.seed as usize)) % 8) as u8;
        if idx == 0 {
            return Cat {
                mode: 0,
                fur_idx: 0,
                x: 0.26 * w,
                y: 0.562 * h,
                tx: 0.26 * w,
                ty: 0.562 * h,
                dir: 1.0,
                ph: self.rng.next() * 6.28,
                wait: 0.0,
                sc: 0.96,
            };
        }
        if idx == 1 {
            return Cat {
                mode: 1,
                fur_idx: 5,
                x: 0.54 * w,
                y: 0.63 * h,
                tx: 0.54 * w,
                ty: 0.63 * h,
                dir: 1.0,
                ph: self.rng.next() * 6.28,
                wait: 0.0,
                sc: 0.96,
            };
        }
        if idx == 2 {
            return Cat {
                mode: 2,
                fur_idx: 2,
                x: 0.37 * w,
                y: 0.68 * h,
                tx: 0.37 * w,
                ty: 0.68 * h,
                dir: 1.0,
                ph: self.rng.next() * 6.28,
                wait: 0.0,
                sc: 0.98,
            };
        }
        if idx == 3 {
            return Cat {
                mode: 0,
                fur_idx: 3,
                x: 0.13 * w,
                y: 0.44 * h,
                tx: 0.13 * w,
                ty: 0.44 * h,
                dir: -1.0,
                ph: self.rng.next() * 6.28,
                wait: 0.0,
                sc: 0.86,
            };
        }
        let (x, y) = self.random_hill_pos();
        let (tx, ty) = self.random_hill_pos();
        let depth_scale = 0.80 + ((y - h * 0.3) / (h * 0.7)).clamp(0.0, 1.0) * 0.26;
        Cat {
            mode: 3,
            fur_idx,
            x,
            y,
            tx,
            ty,
            dir: if self.rng.next() < 0.5 { 1.0 } else { -1.0 },
            ph: self.rng.next() * 6.28,
            wait: if self.rng.next() < 0.35 { 1.5 + self.rng.next() * 3.0 } else { 0.0 },
            sc: depth_scale,
        }
    }

    fn spawn_leaf(&mut self, initial: bool) -> Leaf {
        let (w, h) = (self.b.w, self.b.h);
        let from_tree = self.rng.next() < 0.72;
        let x = if initial {
            self.rng.next() * w
        } else if from_tree {
            (0.86 + self.rng.next() * 0.14) * w
        } else {
            w + 15.0
        };
        let y = if initial {
            (0.12 + self.rng.next() * 0.72) * h
        } else if from_tree {
            (0.10 + self.rng.next() * 0.36) * h
        } else {
            (0.15 + self.rng.next() * 0.45) * h
        };
        let ground_y = (0.36 + self.rng.next() * 0.54) * h;
        let is_ground = initial && y >= ground_y && self.rng.next() < 0.45;
        Leaf {
            x,
            y: if is_ground { ground_y } else { y },
            vy: 14.0 + self.rng.next() * 14.0,
            vx: -(24.0 + self.rng.next() * 22.0),
            rot: self.rng.next() * 6.28,
            vrot: (self.rng.next() - 0.5) * 3.2,
            state: if is_ground { 1 } else { 0 },
            rest: if is_ground { 1.5 + self.rng.next() * 3.5 } else { 0.0 },
            fade: 1.0,
            ground_y,
        }
    }

    fn sync_entities(&mut self) {
        let want_cats = self.p.cats.round().clamp(1.0, 15.0) as usize;
        self.cats.truncate(want_cats);
        while self.cats.len() < want_cats {
            let idx = self.cats.len();
            let c = self.make_cat(idx);
            self.cats.push(c);
        }

        let want_bf = self.p.butterflies.round().clamp(0.0, 6.0) as usize;
        self.butterflies.truncate(want_bf);
        while self.butterflies.len() < want_bf {
            let (x, y) = self.random_hill_pos();
            let b = Butterfly {
                x,
                y: y - 20.0,
                vx: (self.rng.next() - 0.5) * 28.0,
                vy: (self.rng.next() - 0.5) * 14.0,
                ph: self.rng.next() * 6.28,
                timer: 1.5 + self.rng.next() * 2.5,
            };
            self.butterflies.push(b);
        }

        let want_leaves = (self.p.leaves * 28.0).round().clamp(0.0, 28.0) as usize;
        self.leaves.truncate(want_leaves);
        while self.leaves.len() < want_leaves {
            let lf = self.spawn_leaf(true);
            self.leaves.push(lf);
        }
    }

    fn step(&mut self, dt: f32) {
        let (w, h) = (self.b.w, self.b.h);
        let sp_mul = self.p.speed;
        let wind = self.p.wind;

        for i in 0..self.cats.len() {
            if self.cats[i].mode != 3 {
                self.cats[i].ph += dt * (1.2 + 0.8 * sp_mul);
                continue;
            }
            if self.cats[i].wait > 0.0 {
                self.cats[i].wait -= dt;
                self.cats[i].ph += dt * 1.4;
                continue;
            }
            let dx = self.cats[i].tx - self.cats[i].x;
            let dy = self.cats[i].ty - self.cats[i].y;
            let d = dx.hypot(dy);
            if d < 10.0 {
                let (ntx, nty) = self.random_hill_pos();
                let r1 = self.rng.next();
                let r2 = self.rng.next();
                self.cats[i].tx = ntx;
                self.cats[i].ty = nty;
                if r1 < 0.45 {
                    self.cats[i].wait = 2.0 + r2 * 3.5;
                }
            } else {
                let sp = 22.0 * sp_mul.max(0.2);
                let c = &mut self.cats[i];
                c.x += (dx / d) * sp * dt;
                c.y += (dy / d) * sp * 0.72 * dt;
                if dx.abs() > 1.5 {
                    c.dir = if dx > 0.0 { 1.0 } else { -1.0 };
                }
                c.ph += dt * 6.5 * sp_mul.max(0.3);
                c.sc = 0.80 + ((c.y - h * 0.3) / (h * 0.7)).clamp(0.0, 1.0) * 0.26;
            }
        }

        for i in 0..self.butterflies.len() {
            let r1 = self.rng.next();
            let r2 = self.rng.next();
            let r3 = self.rng.next();
            let min_y = self.hill_y(self.butterflies[i].x / w) - 10.0;
            let b = &mut self.butterflies[i];
            b.timer -= dt;
            if b.timer <= 0.0 {
                b.vx = (r1 - 0.5) * 34.0;
                b.vy = (r2 - 0.5) * 18.0;
                b.timer = 1.2 + r3 * 2.5;
            }
            b.x += b.vx * dt;
            b.y += b.vy * dt + (b.ph * 0.7).sin() * 8.0 * dt;
            b.ph += dt * 13.0;
            if b.x < 40.0 {
                b.vx = b.vx.abs();
            }
            if b.x > w - 40.0 {
                b.vx = -b.vx.abs();
            }
            if b.y < min_y {
                b.vy = b.vy.abs();
            }
            if b.y > h - TASKBAR - 40.0 {
                b.vy = -b.vy.abs();
            }
        }

        for i in 0..self.leaves.len() {
            if self.leaves[i].state == 0 {
                let r_rest = self.rng.next();
                let lf = &mut self.leaves[i];
                lf.x += lf.vx * wind.max(0.25) * dt + (lf.y * 0.03).sin() * 10.0 * dt;
                lf.y += lf.vy * dt;
                lf.rot += lf.vrot * dt;
                if lf.y >= lf.ground_y {
                    lf.y = lf.ground_y;
                    lf.state = 1;
                    lf.rest = 2.2 + r_rest * 3.5;
                    lf.fade = 1.0;
                }
                if lf.x < -30.0 || lf.y > h + 20.0 {
                    self.leaves[i] = self.spawn_leaf(false);
                }
            } else {
                let r_blow = self.rng.next();
                let r_vy = self.rng.next();
                let lf = &mut self.leaves[i];
                lf.rest -= dt;
                if wind > 1.3 && r_blow < 0.25 * dt {
                    lf.state = 0;
                    lf.ground_y = (lf.y + 30.0 + r_vy * 60.0).min(h - TASKBAR - 25.0);
                    lf.vy = 10.0 + r_vy * 10.0;
                } else if lf.rest <= 0.0 {
                    lf.fade -= dt * 0.7;
                    if lf.fade <= 0.0 {
                        self.leaves[i] = self.spawn_leaf(false);
                    }
                }
            }
        }
    }

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        let mut order: Vec<usize> = (0..self.cats.len()).collect();
        order.sort_by(|&a, &b| self.cats[a].y.partial_cmp(&self.cats[b].y).unwrap_or(std::cmp::Ordering::Equal));
        let mut cat_pos = [0f32; 60];
        let mut cat_meta = [0f32; 60];
        for (slot, &idx) in order.iter().enumerate() {
            let c = &self.cats[idx];
            cat_pos[slot * 4..slot * 4 + 4].copy_from_slice(&[c.x, c.y, c.sc * c.dir, c.ph]);
            cat_meta[slot * 4..slot * 4 + 4].copy_from_slice(&[c.mode as f32, c.fur_idx as f32, 0.0, 0.0]);
        }
        let mut bfly = [0f32; 24];
        for (i, bf) in self.butterflies.iter().enumerate() {
            bfly[i * 4..i * 4 + 4].copy_from_slice(&[bf.x, bf.y, if bf.vx >= 0.0 { 1.0 } else { -1.0 }, bf.ph]);
        }
        let mut leaf = [0f32; 112];
        for (i, lf) in self.leaves.iter().enumerate() {
            leaf[i * 4..i * 4 + 4].copy_from_slice(&[lf.x, lf.y, lf.rot, lf.fade]);
        }

        let p = &mut sh.prog;
        unsafe { gl.use_program(Some(p.prog)) };
        self.b.bind_common(gl, p, sh.noise_tex, sh.seed, t);
        p.f1(gl, "uWind", self.p.wind);
        p.f1(gl, "uCatN", self.cats.len() as f32);
        p.f4v(gl, "uCatPos", &cat_pos);
        p.f4v(gl, "uCatMeta", &cat_meta);
        p.f1(gl, "uBflyN", self.butterflies.len() as f32);
        p.f4v(gl, "uBfly", &bfly);
        p.f1(gl, "uLeafN", self.leaves.len() as f32);
        p.f4v(gl, "uLeaf", &leaf);
        unsafe { gl.draw_arrays(glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for CatHill {
    fn set_icons(&mut self, _gl: &glow::Context, icons: Vec<IconBox>, _first: bool, _now: f64) {
        self.b.icons = icons;
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, now: f64) {
        self.step(dt);
        self.draw(gl, sh, now);
    }

    fn settle(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        self.draw(gl, sh, now);
    }

    fn destroy(&mut self, gl: &glow::Context) {
        self.b.destroy(gl);
    }

    fn field_ms(&self) -> f64 {
        0.0
    }
}
