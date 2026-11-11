//! Chủ đề Bể cá vàng (prototype/aquarium.html): nhìn ngang qua thành kính, rạn cát ôm biểu tượng,
//! rong + bong bóng, cá vàng bơi tự do và trò câu cá bằng click chuột.
//! Shader dùng chung với bản web: prototype/aquarium.frag (một nguồn duy nhất).
//!
//! Câu cá: click vào nước -> thả câu; cá tới rỉa mồi; phao giật mạnh + dấu "!" -> click ngay để kéo cá
//! lên (cá văng khỏi mặt nước). Click sớm / trễ thì cá chạy mất. Click cát -> tung bụi cát.

use crate::engine::*;
use std::f32::consts::PI;

pub const FRAG: &str = include_str!("../../../prototype/aquarium.frag");

const SURF: f32 = 30.0;

pub struct Params {
    pub sigma: f32,
    pub warp: f32,
    pub fish: f32,
    pub plants: f32,
    pub bubbles: f32,
    pub light: f32,
    pub eager: f32,
}

impl Params {
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        Params {
            sigma: g("sigma", 80.0),
            warp: g("warp", 30.0),
            fish: g("fish", 4.0),
            plants: g("plants", 0.6),
            bubbles: g("bubbles", 0.5),
            light: g("light", 0.6),
            eager: g("eager", 1.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Swim,
    Approach,
    Nibble,
    Bite,
    Hooked,
    Flee,
    Caught,
    Gone,
}

struct Fish {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    head: f32,
    ang: f32,
    ph: f32,
    size: f32,
    spots: f32,
    bend: f32,
    alpha: f32,
    mode: Mode,
    t: f32,
    wh: f32,
    id: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum LineState {
    Cast,
    Wait,
    Reel,
    Up,
}

struct Line {
    bx: f32,
    hx: f32,
    hy: f32,
    ty: f32,
    state: LineState,
    bait: bool,
    dip: f32,
    bite: f32,
    alpha: f32,
    idle: f32,
    t: f32,
}

pub struct Aquarium {
    b: Base,
    p: Params,
    rng: Rng,
    fish: Vec<Fish>,
    plants: Vec<[f32; 4]>,
    line: Option<Line>,
    fx: Vec<[f32; 4]>,
    caught: u32,
    tt: f32,
    t0: f64,
}

fn turn_toward(h: f32, target: f32, rate: f32) -> f32 {
    let mut d = target - h;
    d = d.sin().atan2(d.cos());
    h + d.signum() * d.abs().min(rate)
}

impl Aquarium {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = Aquarium {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            fish: Vec::new(),
            plants: Vec::new(),
            line: None,
            fx: Vec::new(),
            caught: 0,
            tt: 0.0,
            t0: now,
        };
        s.commit(gl, true, now);
        s.sync_fish();
        for f in &mut s.fish {
            f.alpha = 0.0;
        }
        s
    }

    fn commit(&mut self, gl: &glow::Context, from_empty: bool, now: f64) {
        let rects = icon_rects(&self.b.icons);
        self.b.commit(gl, &rects, self.p.sigma, self.p.warp, from_empty, now);
        self.build_plants();
    }

    fn seed_u(&self) -> f32 {
        self.b.noise.seed.rem_euclid(997) as f32 * 0.37
    }

    /// Mép cát đáy bể - y hệt sandTop() trong shader
    fn sand_top(&self, x: f32) -> f32 {
        let su = self.seed_u();
        self.b.h - TASKBAR - (46.0 + 18.0 * (x * 0.0042 + su).sin() + 10.0 * (x * 0.0113 + su * 1.7 + 1.3).sin())
    }

    fn tgt_s(&self, x: f32, y: f32) -> f32 {
        if self.b.fb.is_empty() {
            return 999.0;
        }
        self.b.field.sample(&self.b.fb, x, y)
    }

    fn is_water(&self, x: f32, y: f32) -> bool {
        y > SURF + 12.0 && y < self.sand_top(x) - 8.0 && self.b.cur_s(x, y) > 8.0
    }

    fn hash(&self, x: f32, y: f32, k: i32) -> f32 {
        self.b.noise.hash2(x as f64, y as f64, k) as f32
    }

    /// Rong trên mép trên của rạn cát (có nước phía trên) và dọc đáy bể; cố định theo seed
    fn build_plants(&mut self) {
        let dens = self.p.plants;
        let (w, h) = (self.b.w, self.b.h);
        let mut out: Vec<[f32; 4]> = Vec::new();
        let mut k = 0.0f32;
        let far = |out: &Vec<[f32; 4]>, x: f32, y: f32, d: f32| out.iter().all(|p| (p[0] - x).hypot(p[1] - y) > d);
        if dens > 0.0 && !self.b.fb.is_empty() {
            let mut y = SURF + 40.0;
            'rows: while y < h - TASKBAR - 60.0 {
                let mut x = 10.0;
                while x < w - 10.0 {
                    if out.len() >= 18 {
                        break 'rows;
                    }
                    let s = self.tgt_s(x, y);
                    if s.abs() <= 3.0 && self.tgt_s(x, y - 26.0) >= 16.0 && self.tgt_s(x, y + 10.0) <= -4.0 && self.hash(x, y, 41) <= dens * 0.55 && far(&out, x, y, 46.0) {
                        out.push([x, y + 3.0, 30.0 + 34.0 * self.hash(x, y, 42), k * 1.37 + self.hash(x, y, 43) * 50.0]);
                        k += 1.0;
                    }
                    x += 10.0;
                }
                y += 10.0;
            }
        }
        let mut x = 24.0f32;
        while x < w - 10.0 && dens > 0.0 && out.len() < 32 {
            if self.hash(x, 7.0, 44) <= dens * 0.9 {
                let xx = x + (self.hash(x, 8.0, 45) - 0.5) * 20.0;
                if self.tgt_s(xx, self.sand_top(xx) - 30.0) >= 10.0 {
                    out.push([xx, self.sand_top(xx) + 4.0, 60.0 + 80.0 * self.hash(x, 9.0, 46), k * 1.37 + self.hash(x, 9.0, 47) * 50.0]);
                    k += 1.0;
                }
            }
            x += 44.0;
        }
        out.truncate(32);
        self.plants = out;
    }

    fn spawn_fish(&mut self, from_edge: bool) -> Option<Fish> {
        let (w, h) = (self.b.w, self.b.h);
        for _ in 0..300 {
            let left = self.rng.next() < 0.5;
            let x = if from_edge {
                if left {
                    -40.0
                } else {
                    w + 40.0
                }
            } else {
                60.0 + self.rng.next() * (w - 120.0)
            };
            let y = SURF + 70.0 + self.rng.next() * (h - TASKBAR - SURF - 220.0).max(10.0);
            if !from_edge && !self.is_water(x, y) {
                continue;
            }
            if from_edge && y > self.sand_top(x.clamp(0.0, w)) - 70.0 {
                continue;
            }
            let head = if from_edge {
                (if left { 0.0 } else { PI }) + (self.rng.next() - 0.5) * 0.4
            } else if self.rng.next() < 0.5 {
                0.0
            } else {
                PI
            };
            return Some(Fish {
                x,
                y,
                vx: head.cos() * 20.0,
                vy: 0.0,
                head,
                ang: head,
                ph: self.rng.next() * 6.0,
                size: 1.35 + self.rng.next() * 0.5,
                spots: self.rng.next(),
                bend: 0.6,
                alpha: if from_edge { 1.0 } else { 0.0 },
                mode: Mode::Swim,
                t: 0.0,
                wh: self.rng.next() * 100.0,
                id: self.rng.next() * 50.0,
            });
        }
        None
    }

    fn sync_fish(&mut self) {
        let want = self.p.fish.round().clamp(1.0, 8.0) as usize;
        self.fish.truncate(want);
        while self.fish.len() < want {
            match self.spawn_fish(false) {
                Some(f) => self.fish.push(f),
                None => break,
            }
        }
    }

    /// Vector đẩy cá khỏi rạn cát, đáy, mặt nước, mép màn hình và cá khác (nhìn trước 60px)
    fn repel(&self, i: usize) -> (f32, f32) {
        let f = &self.fish[i];
        let look = 60.0 * f.size;
        let (ax, ay) = (f.x + f.head.cos() * look, f.y + f.head.sin() * look);
        let (mut rx, mut ry) = (0.0f32, 0.0f32);
        let s = self.b.cur_s(ax, ay);
        if s < 50.0 {
            let gx = self.b.cur_s(ax + 8.0, ay) - self.b.cur_s(ax - 8.0, ay);
            let gy = self.b.cur_s(ax, ay + 8.0) - self.b.cur_s(ax, ay - 8.0);
            let g = gx.hypot(gy).max(1e-3);
            rx += gx / g * (50.0 - s) / 25.0;
            ry += gy / g * (50.0 - s) / 25.0;
        }
        let bot = self.sand_top(ax.clamp(0.0, self.b.w)) - 46.0 - ay;
        if bot < 0.0 {
            ry -= (-bot / 25.0).min(2.0);
        }
        if ay < SURF + 50.0 {
            ry += ((SURF + 50.0 - ay) / 25.0).min(2.0);
        }
        if ax < 40.0 {
            rx += (40.0 - ax) / 30.0;
        }
        if ax > self.b.w - 40.0 {
            rx -= (ax - self.b.w + 40.0) / 30.0;
        }
        for (j, o) in self.fish.iter().enumerate() {
            if j == i || o.mode == Mode::Gone {
                continue;
            }
            let (dx, dy) = (f.x - o.x, f.y - o.y);
            let d = dx.hypot(dy);
            let m = 70.0 * f.size;
            if d < m && d > 0.01 {
                rx += dx / d * (m - d) / m * 1.5;
                ry += dy / d * (m - d) / m * 1.5;
            }
        }
        (rx, ry)
    }

    fn line_fish(&self) -> Option<usize> {
        self.fish.iter().position(|f| matches!(f.mode, Mode::Approach | Mode::Nibble | Mode::Bite | Mode::Hooked))
    }

    fn add_fx(&mut self, x: f32, y: f32, kind: f32) {
        if self.fx.len() >= 8 {
            self.fx.remove(0);
        }
        self.fx.push([x, y, 0.0, kind]);
    }

    fn flee(&mut self, i: usize) {
        let r = self.rng.next();
        let hx = self.line.as_ref().map(|l| l.hx);
        let f = &mut self.fish[i];
        f.mode = Mode::Flee;
        f.t = 1.1;
        if let Some(hx) = hx {
            f.head = if f.x < hx { PI + (r - 0.5) * 0.6 } else { (r - 0.5) * 0.6 };
        }
        let (x, y) = (f.x, f.y);
        self.add_fx(x, y, 3.0);
    }

    fn step_fish(&mut self, i: usize, dt: f32) {
        let tt = self.tt;
        if self.fish[i].mode == Mode::Caught {
            let f = &mut self.fish[i];
            f.t += dt;
            f.vy += 520.0 * dt;
            f.x += f.vx * dt;
            f.y += f.vy * dt;
            f.ang += dt * 7.0 * if f.vx < 0.0 { -1.0 } else { 1.0 };
            f.ph += dt * 20.0;
            f.alpha = (1.0 - (f.t - 0.45).max(0.0) / 0.35).max(0.0);
            if f.t > 0.8 {
                f.mode = Mode::Gone;
                f.t = 2.2;
                f.alpha = 0.0;
            }
            return;
        }
        if self.fish[i].mode == Mode::Gone {
            self.fish[i].alpha = 0.0;
            self.fish[i].t -= dt;
            if self.fish[i].t <= 0.0 {
                if let Some(n) = self.spawn_fish(true) {
                    self.fish[i] = n;
                }
            }
            return;
        }
        self.fish[i].alpha = (self.fish[i].alpha + dt * 1.2).min(1.0);
        let (mut dvx, mut dvy, mut k) = (0.0f32, 0.0f32, 2.0f32);
        let line = self.line.as_ref().map(|l| (l.hx, l.hy));
        let mode = self.fish[i].mode;
        match (mode, line) {
            (Mode::Swim, _) => {
                let noise = self.b.noise;
                {
                    let f = &mut self.fish[i];
                    f.wh += dt * 0.35;
                    f.head += (noise.vnoise(f.wh as f64, (i as f32 * 9.1 + f.id) as f64, 3) as f32 - 0.5) * 1.6 * dt;
                }
                let (rx, ry) = self.repel(i);
                let f = &mut self.fish[i];
                if rx != 0.0 || ry != 0.0 {
                    f.head = turn_toward(f.head, (ry + f.head.sin() * 0.3).atan2(rx + f.head.cos() * 0.3), 1.8 * dt);
                }
                if f.head.sin().abs() > 0.5 {
                    f.head = turn_toward(f.head, if f.head.cos() >= 0.0 { 0.0 } else { PI }, 0.6 * dt);
                }
                let sp = 26.0 * f.size * (0.45 + 0.9 * noise.vnoise((f.wh * 0.6) as f64, (i as f32 * 3.3 + f.id) as f64, 5) as f32);
                dvx = f.head.cos() * sp;
                dvy = f.head.sin() * sp;
            }
            (Mode::Approach | Mode::Nibble | Mode::Bite, Some((hx, hy))) => {
                let eager = self.p.eager.max(0.3);
                let r = self.rng.next();
                let f = &mut self.fish[i];
                let side = if f.x < hx { -1.0 } else { 1.0 };
                let mut tx = hx + side * 14.0 * f.size;
                let ty = hy + if mode == Mode::Bite { 7.0 } else { 1.0 };
                if mode == Mode::Nibble {
                    tx += side * (2.0 + 3.0 * (tt * 6.0 + f.id).sin().abs());
                }
                let (dx, dy) = (tx - f.x, ty - f.y);
                let d = dx.hypot(dy);
                let sp = (if mode == Mode::Approach { 48.0 * f.size } else { 60.0 }).min(d * 3.0);
                if d > 0.01 {
                    dvx = dx / d * sp;
                    dvy = dy / d * sp;
                }
                k = 4.0;
                f.head = if side < 0.0 { 0.0 } else { PI };
                if mode == Mode::Approach && d < 5.0 {
                    f.mode = Mode::Nibble;
                    f.t = (1.2 + r * 2.2) / eager;
                } else if mode == Mode::Nibble {
                    f.t -= dt;
                    if f.t <= 0.0 {
                        f.mode = Mode::Bite;
                        f.t = 1.35;
                    }
                } else if mode == Mode::Bite {
                    f.t -= dt;
                    if f.t <= 0.0 {
                        if let Some(l) = self.line.as_mut() {
                            l.bait = false;
                        }
                        self.flee(i);
                    }
                }
            }
            (Mode::Hooked, Some((hx, hy))) => {
                let f = &mut self.fish[i];
                let wig = (tt * 16.0).sin() * 0.55;
                f.ang = -PI / 2.0 + wig;
                f.head = f.ang;
                f.x = hx + (tt * 16.0).sin() * 3.0;
                f.y = hy + 13.0 * f.size;
                f.ph += dt * 18.0;
                return;
            }
            (Mode::Flee, _) => {
                let f = &mut self.fish[i];
                f.t -= dt;
                let sp = 140.0 * f.size;
                dvx = f.head.cos() * sp;
                dvy = f.head.sin() * sp * 0.4;
                k = 5.0;
                if f.t <= 0.0 {
                    f.mode = Mode::Swim;
                }
            }
            _ => self.fish[i].mode = Mode::Swim,
        }
        let (w, h) = (self.b.w, self.b.h);
        let f = &mut self.fish[i];
        let a = (dt * k).min(1.0);
        f.vx += (dvx - f.vx) * a;
        f.vy += (dvy - f.vy) * a;
        f.x = (f.x + f.vx * dt).clamp(-80.0, w + 80.0);
        f.y = (f.y + f.vy * dt).clamp(SURF + 16.0, h - TASKBAR - 20.0);
        let spd = f.vx.hypot(f.vy);
        if spd > 4.0 {
            f.ang = turn_toward(f.ang, (f.vy * 0.6).atan2(f.vx), 4.0 * dt);
        }
        f.ph += dt * (4.0 + spd * 0.12);
        f.bend = 0.5 + (spd / 50.0).min(1.2);
    }

    fn step_line(&mut self, dt: f32) {
        let Some(mut l) = self.line.take() else { return };
        l.alpha = (l.alpha + dt * 4.0).min(1.0);
        let lf = self.line_fish();
        let lf_mode = lf.map(|i| self.fish[i].mode);
        let mut dip_t = 0.0;
        let mut keep = true;
        match l.state {
            LineState::Cast => {
                l.hy = (l.hy + 280.0 * dt).min(l.ty);
                if l.hy >= l.ty {
                    l.state = LineState::Wait;
                    l.t = (0.8 + self.rng.next() * 1.6) / self.p.eager.max(0.3);
                }
            }
            LineState::Wait => {
                l.idle += dt;
                if lf.is_none() && l.bait {
                    l.t -= dt;
                    if l.t <= 0.0 {
                        let best = self
                            .fish
                            .iter()
                            .enumerate()
                            .filter(|(_, f)| f.mode == Mode::Swim && f.alpha > 0.9)
                            .min_by(|a, b| (a.1.x - l.hx).hypot(a.1.y - l.hy).total_cmp(&(b.1.x - l.hx).hypot(b.1.y - l.hy)))
                            .map(|(i, _)| i);
                        match best {
                            Some(i) => self.fish[i].mode = Mode::Approach,
                            None => l.t = 1.0,
                        }
                    }
                }
                if lf_mode == Some(Mode::Nibble) {
                    dip_t = 1.5 + 2.5 * (self.tt * 7.0).sin().abs();
                }
                if lf_mode == Some(Mode::Bite) {
                    dip_t = 11.0;
                }
                if (!l.bait && lf.is_none() && l.idle > 3.0) || l.idle > 45.0 {
                    l.state = LineState::Up;
                }
            }
            LineState::Reel => {
                dip_t = 4.0;
                l.hy -= 210.0 * dt;
                if l.hy <= SURF + 6.0 {
                    if let Some(i) = lf {
                        let r = self.rng.next();
                        let f = &mut self.fish[i];
                        f.mode = Mode::Caught;
                        f.t = 0.0;
                        f.vx = (r - 0.5) * 120.0;
                        f.vy = -330.0;
                    }
                    self.caught += 1;
                    crate::log!("Bể cá: câu được con cá thứ {}", self.caught);
                    self.add_fx(l.bx, SURF, 1.0);
                    keep = false;
                }
            }
            LineState::Up => {
                l.hy -= 320.0 * dt;
                l.alpha = l.alpha.min((l.hy - SURF) / 30.0 + 0.2).max(0.0);
                if l.hy <= SURF {
                    keep = false;
                }
            }
        }
        if keep {
            l.dip += (dip_t - l.dip) * (dt * 10.0).min(1.0);
            let bt = if lf_mode == Some(Mode::Bite) { 1.0 } else { 0.0 };
            l.bite += (bt - l.bite) * (dt * 12.0).min(1.0);
            self.line = Some(l);
        }
    }

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        let mut fa = [0f32; 32];
        let mut fb = [0f32; 32];
        for (i, f) in self.fish.iter().take(8).enumerate() {
            fa[i * 4..i * 4 + 4].copy_from_slice(&[f.x, f.y, f.ang, f.ph]);
            fb[i * 4..i * 4 + 4].copy_from_slice(&[f.size, f.spots, f.bend, f.alpha]);
        }
        let mut pa = [0f32; 128];
        for (i, pl) in self.plants.iter().take(32).enumerate() {
            pa[i * 4..i * 4 + 4].copy_from_slice(pl);
        }
        let mut fxa = [-1f32; 32];
        for (i, f) in self.fx.iter().take(8).enumerate() {
            fxa[i * 4..i * 4 + 4].copy_from_slice(f);
        }
        let p = &mut sh.prog;
        unsafe { glow::HasContext::use_program(gl, Some(p.prog)) };
        self.b.bind_common(gl, p, sh.noise_tex, sh.seed, t);
        p.f1(gl, "uLight", self.p.light);
        p.f1(gl, "uBubbles", self.p.bubbles);
        p.f1(gl, "uSurf", SURF);
        p.f1(gl, "uFishN", self.fish.len().min(8) as f32);
        p.f4v(gl, "uFish", &fa);
        p.f4v(gl, "uFishB", &fb);
        p.f1(gl, "uPlantN", self.plants.len().min(32) as f32);
        p.f4v(gl, "uPlant", &pa);
        let (l1, l2) = match &self.line {
            Some(l) => (
                [l.bx, l.hx, l.hy, l.alpha],
                [l.dip, if l.bait { 1.0 } else { 0.0 }, l.bite, if l.state == LineState::Reel { 1.0 } else { 0.0 }],
            ),
            None => ([0.0; 4], [0.0; 4]),
        };
        p.f4v(gl, "uLine", &l1);
        p.f4v(gl, "uLineB", &l2);
        p.f4v(gl, "uFx", &fxa);
        unsafe { glow::HasContext::draw_arrays(gl, glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for Aquarium {
    fn set_icons(&mut self, gl: &glow::Context, icons: Vec<IconBox>, first: bool, now: f64) {
        self.b.icons = icons;
        self.commit(gl, first, now);
        // Cá đang nằm trong vùng rạn cát mới thì thả lại chỗ khác
        for i in 0..self.fish.len() {
            let (x, y, m) = (self.fish[i].x, self.fish[i].y, self.fish[i].mode);
            if m == Mode::Swim && self.tgt_s(x, y) < 8.0 {
                if let Some(mut n) = self.spawn_fish(false) {
                    n.alpha = 0.0;
                    self.fish[i] = n;
                }
            }
        }
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, now: f64) {
        self.b.update_mix(now);
        let dt = dt.min(0.1);
        self.tt += dt;
        if self.fish.len() != self.p.fish.round().clamp(1.0, 8.0) as usize {
            self.sync_fish();
        }
        for i in 0..self.fish.len() {
            self.step_fish(i, dt);
        }
        self.step_line(dt);
        self.fx.retain_mut(|f| {
            f[2] += dt;
            f[2] < 2.0
        });
        self.draw(gl, sh, now);
    }

    fn settle(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        self.b.settle();
        self.draw(gl, sh, now);
    }

    fn handle_click(&mut self, _gl: &glow::Context, x: f32, y: f32, _now: f64) {
        if self.line.as_ref().is_some_and(|l| l.state == LineState::Reel) {
            return;
        }
        let lf = self.line_fish();
        if let (Some(i), Some(l)) = (lf, self.line.as_mut()) {
            if self.fish[i].mode == Mode::Bite {
                // Trúng nhịp: dính câu!
                self.fish[i].mode = Mode::Hooked;
                l.state = LineState::Reel;
                l.bite = 0.0;
                let bx = l.bx;
                self.add_fx(bx, SURF, 0.0);
                return;
            }
        }
        if let Some(i) = lf {
            self.flee(i); // bấm sớm: cá giật mình bỏ chạy
        }
        if !self.is_water(x, y) {
            if let Some(l) = self.line.as_mut() {
                l.state = LineState::Up;
            } else if self.b.cur_s(x, y) < 0.0 || y >= self.sand_top(x) - 8.0 {
                self.add_fx(x, y, 2.0);
            }
            for i in 0..self.fish.len() {
                let (fx, fy, m) = (self.fish[i].x, self.fish[i].y, self.fish[i].mode);
                if m == Mode::Swim && (fx - x).hypot(fy - y) < 160.0 {
                    self.flee(i);
                    self.fish[i].head = (fy - y).atan2(fx - x); // bơi ra xa chỗ bấm
                }
            }
            return;
        }
        crate::log!("Bể cá: thả câu tại ({x:.0}, {y:.0})");
        self.line = Some(Line {
            bx: x,
            hx: x,
            hy: SURF,
            ty: y,
            state: LineState::Cast,
            bait: true,
            dip: 0.0,
            bite: 0.0,
            alpha: 0.0,
            idle: 0.0,
            t: 0.0,
        });
        self.add_fx(x, SURF, 0.0);
    }

    fn is_interacting(&self, _now: f64) -> bool {
        self.line.is_some() || !self.fx.is_empty() || self.fish.iter().any(|f| matches!(f.mode, Mode::Caught | Mode::Flee))
    }

    fn destroy(&mut self, gl: &glow::Context) {
        self.b.destroy(gl);
    }

    fn field_ms(&self) -> f64 {
        self.b.field_ms
    }
}
