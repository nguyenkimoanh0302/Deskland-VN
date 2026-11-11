//! Chủ đề Chuyến xe chiều (prototype/commute.html): phong cảnh nhìn ngang kiểu hoạt hình - trời đổi màu
//! theo giờ trong ngày, phố bên sông, cầu vòm, con đường có xe buýt, lau sậy tiền cảnh, đồi cỏ / mây ôm
//! biểu tượng. Shader dùng chung với bản web: prototype/commute.frag (một nguồn duy nhất).
//!
//! Click: đường -> gọi xe buýt dừng tại đó (đang chạy qua thì bấm còi); trời -> đàn chim (ngày) / sao băng
//! (đêm); cỏ / sông -> gió rạp lau + hạt bồ công anh (ngày) / đom đóm (đêm).

use crate::engine::*;
use std::f32::consts::PI;

pub const FRAG: &str = include_str!("../../../prototype/commute.frag");

pub struct Params {
    pub sigma: f32,
    pub warp: f32,
    pub time: f32,
    pub wind: f32,
    pub reeds: f32,
    pub lights: f32,
    pub bus_freq: f32,
}

impl Params {
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        Params {
            sigma: g("sigma", 70.0),
            warp: g("warp", 30.0),
            time: g("time", 0.0),
            wind: g("wind", 1.0),
            reeds: g("reeds", 0.7),
            lights: g("lights", 1.0),
            bus_freq: g("busFreq", 1.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum BusState {
    Off,
    Drive,
    Stop,
}

struct Bus {
    x: f32,
    dir: f32,
    v: f32,
    state: BusState,
    target: Option<f32>,
    doors: f32,
    honk: f32,
    brake: f32,
    rot: f32,
    alpha: f32,
    next: f32,
    t: f32,
}

struct Part {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    kind: f32,
    life: f32,
    age: f32,
    ph: f32,
    seed: f32,
}

pub struct Commute {
    b: Base,
    p: Params,
    rng: Rng,
    bus: Bus,
    stop_x: f32,
    parts: Vec<Part>,
    star: Option<[f32; 4]>,
    gust: Option<[f32; 4]>,
    amb_t: f32,
    t0: f64,
}

fn ss(a: f32, b: f32, x: f32) -> f32 {
    sstep(a, b, x)
}

/// Giờ trong ngày 0..1 theo đồng hồ máy (giờ địa phương)
fn clock_tod() -> f32 {
    let mut st: windows_sys::Win32::Foundation::SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st) };
    (st.wHour as f32 + st.wMinute as f32 / 60.0 + st.wSecond as f32 / 3600.0) / 24.0
}

fn night_w(t: f32) -> f32 {
    (1.0 - ss(0.21, 0.29, t) + ss(0.79, 0.87, t)).clamp(0.0, 1.0)
}

impl Commute {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = Commute {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            bus: Bus { x: -9999.0, dir: 1.0, v: 0.0, state: BusState::Off, target: None, doors: 0.0, honk: -1.0, brake: 0.0, rot: 0.0, alpha: 0.0, next: 4.0, t: 0.0 },
            stop_x: w * 0.34,
            parts: Vec::new(),
            star: None,
            gust: None,
            amb_t: 6.0,
            t0: now,
        };
        s.commit(gl, true, now);
        s
    }

    fn commit(&mut self, gl: &glow::Context, from_empty: bool, now: f64) {
        let rects = icon_rects(&self.b.icons);
        self.b.commit(gl, &rects, self.p.sigma, self.p.warp, from_empty, now);
    }

    // Bố cục - khớp commute.frag
    fn hu(&self) -> f32 {
        self.b.h - TASKBAR
    }
    fn y_river(&self) -> f32 {
        self.hu() * 0.56
    }
    fn y_road(&self) -> f32 {
        self.hu() * 0.715
    }
    fn y_wheel(&self) -> f32 {
        self.hu() * 0.79
    }

    fn tod(&self) -> f32 {
        match self.p.time.round() as i32 {
            1 => 0.5,
            2 => 0.735,
            3 => 0.95,
            _ => clock_tod(),
        }
    }

    fn spawn_bus(&mut self, dir: f32, target: Option<f32>) {
        let w = self.b.w;
        let b = &mut self.bus;
        b.dir = dir;
        b.x = if dir > 0.0 { -170.0 } else { w + 170.0 };
        b.v = 95.0;
        b.state = BusState::Drive;
        b.target = target;
        b.doors = 0.0;
        b.alpha = 1.0;
        b.t = 0.0;
    }

    fn step_bus(&mut self, dt: f32) {
        let b = &mut self.bus;
        b.honk = if b.honk >= 0.0 && b.honk + dt <= 0.8 { b.honk + dt } else { -1.0 };
        if b.state == BusState::Off {
            b.next -= dt;
            if b.next <= 0.0 {
                let dir = if self.rng.next() < 0.75 { 1.0 } else { -1.0 };
                let tgt = if self.rng.next() < 0.7 { Some(self.stop_x) } else { None };
                self.spawn_bus(dir, tgt);
            }
            return;
        }
        let mut want = 95.0f32;
        let mut decel = false;
        if b.state == BusState::Drive {
            if let Some(tx) = b.target {
                let d = (tx - b.x) * b.dir;
                if d < -40.0 {
                    b.target = None; // đã qua trạm: bỏ dừng
                } else if d < 150.0 {
                    want = (d * 0.75).max(0.0);
                    decel = true;
                    if d < 1.5 {
                        b.state = BusState::Stop;
                        b.t = 3.6;
                        b.v = 0.0;
                    }
                }
            }
        }
        if b.state == BusState::Stop {
            b.t -= dt;
            b.doors = if b.t > 1.0 { b.doors + dt * 2.0 } else { b.doors - dt * 2.0 }.clamp(0.0, 1.0);
            if b.t <= 0.0 {
                b.state = BusState::Drive;
                b.target = None;
                b.honk = 0.0;
            }
            want = 0.0;
        }
        b.v += (want - b.v) * (dt * if decel { 3.0 } else { 0.9 }).min(1.0);
        let br = if decel || b.state == BusState::Stop { 1.0 } else { 0.0 };
        b.brake += (br - b.brake) * (dt * 6.0).min(1.0);
        b.x += b.v * b.dir * dt;
        b.rot -= b.v * dt / 13.0;
        if b.state == BusState::Drive && b.target.is_none() && (b.x < -200.0 || b.x > self.b.w + 200.0) {
            b.state = BusState::Off;
            b.alpha = 0.0;
            b.next = (22.0 + self.rng.next() * 26.0) / self.p.bus_freq.max(0.2);
        }
    }

    fn add_part(&mut self, x: f32, y: f32, kind: f32, vx: f32, vy: f32, life: f32) {
        if self.parts.len() >= 24 {
            self.parts.remove(0);
        }
        let ph = self.rng.next() * 6.28;
        let seed = self.rng.next() * 100.0;
        self.parts.push(Part { x, y, vx, vy, kind, life, age: 0.0, ph, seed });
    }

    fn flock(&mut self, x: f32, y: f32, dir: f32) {
        for i in 0..5 {
            let r1 = self.rng.next();
            let r2 = self.rng.next();
            let fi = i as f32;
            let side = if i % 2 == 1 { 1.0 } else { -1.0 };
            self.add_part(x - dir * fi * 18.0, y + side * fi * 7.0, 0.0, dir * (95.0 + r1 * 15.0), -8.0 + r2 * 6.0, 16.0);
        }
    }

    fn step_parts(&mut self, dt: f32, night: bool) {
        let noise = self.b.noise;
        let (w, wind) = (self.b.w, self.p.wind);
        for p in &mut self.parts {
            p.age += dt;
            if p.kind < 0.5 {
                p.ph += dt * 11.0;
                p.vy += (p.age * 1.3 + p.seed).sin() * 6.0 * dt;
            } else if p.kind < 1.5 {
                p.vx += (wind * 22.0 - p.vx) * dt * 0.5;
                p.vy += (-14.0 + (p.age * 2.0 + p.seed).sin() * 10.0 - p.vy) * dt;
                p.ph += dt;
            } else {
                p.ph += dt;
                p.vx += (noise.vnoise((p.age * 0.6) as f64, p.seed as f64, 9) as f32 - 0.5) * 60.0 * dt;
                p.vy += (noise.vnoise((p.age * 0.6) as f64, (p.seed + 30.0) as f64, 9) as f32 - 0.5) * 60.0 * dt;
                p.vx *= 0.98;
                p.vy *= 0.98;
            }
            p.x += p.vx * dt;
            p.y += p.vy * dt;
        }
        self.parts.retain(|p| p.age <= p.life && p.x >= -60.0 && p.x <= w + 60.0 && p.y >= -40.0);
        // Cảnh nền tự sinh: đom đóm ban đêm, thỉnh thoảng có đàn chim ban ngày
        self.amb_t -= dt;
        if self.amb_t <= 0.0 {
            let r = self.rng.next();
            self.amb_t = if night { 1.2 } else { 30.0 + r * 30.0 };
            let hu = self.hu();
            if night {
                if self.parts.iter().filter(|p| p.kind > 1.5).count() < 8 {
                    let (x, y) = (self.rng.next() * w, hu * (0.78 + self.rng.next() * 0.18));
                    self.add_part(x, y, 2.0, 0.0, 0.0, 14.0);
                }
            } else {
                let d = if self.rng.next() < 0.5 { 1.0 } else { -1.0 };
                let y = hu * (0.12 + self.rng.next() * 0.25);
                self.flock(if d > 0.0 { -40.0 } else { w + 40.0 }, y, d);
            }
        }
        if let Some(s) = &mut self.star {
            s[2] += dt;
            if s[2] > 1.2 {
                self.star = None;
            }
        }
        if let Some(g) = &mut self.gust {
            g[2] += dt;
            if g[2] > 3.0 {
                self.gust = None;
            }
        }
    }

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        let mut pa = [0f32; 96];
        for (i, q) in self.parts.iter().take(24).enumerate() {
            pa[i * 4..i * 4 + 4].copy_from_slice(&[q.x, q.y, q.ph, q.kind]);
        }
        let tod = self.tod();
        let p = &mut sh.prog;
        unsafe { glow::HasContext::use_program(gl, Some(p.prog)) };
        self.b.bind_common(gl, p, sh.noise_tex, sh.seed, t);
        p.f1(gl, "uTod", tod);
        p.f1(gl, "uWind", self.p.wind);
        p.f1(gl, "uReeds", self.p.reeds);
        p.f1(gl, "uLights", self.p.lights);
        let b = &self.bus;
        p.f4v(gl, "uBus", &[b.x, b.dir, b.rot, b.alpha]);
        p.f4v(gl, "uBusB", &[b.doors, b.honk, b.brake, 0.0]);
        p.f4v(gl, "uStop", &[self.stop_x, 1.0, 0.0, 0.0]);
        p.f4v(gl, "uGust", &self.gust.unwrap_or([0.0, 0.0, -1.0, 0.0]));
        p.f1(gl, "uPartN", self.parts.len().min(24) as f32);
        p.f4v(gl, "uPart", &pa);
        p.f4v(gl, "uStar", &self.star.unwrap_or([0.0, 0.0, -1.0, 0.0]));
        unsafe { glow::HasContext::draw_arrays(gl, glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for Commute {
    fn set_icons(&mut self, gl: &glow::Context, icons: Vec<IconBox>, first: bool, now: f64) {
        self.b.icons = icons;
        self.commit(gl, first, now);
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, now: f64) {
        self.b.update_mix(now);
        let dt = dt.min(0.1);
        self.step_bus(dt);
        let night = night_w(self.tod()) > 0.5;
        self.step_parts(dt, night);
        self.draw(gl, sh, now);
    }

    fn settle(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        self.b.settle();
        self.draw(gl, sh, now);
    }

    fn handle_click(&mut self, _gl: &glow::Context, x: f32, y: f32, _now: f64) {
        let s = self.b.cur_s(x, y);
        let night = night_w(self.tod()) > 0.5;
        let w = self.b.w;
        if s > 10.0 && y > self.y_road() - 40.0 && y < self.y_wheel() + 25.0 {
            // Gọi xe buýt dừng ở chỗ bấm
            self.stop_x = x.clamp(60.0, w - 60.0);
            let sx = self.stop_x;
            let b = &mut self.bus;
            if b.state == BusState::Off {
                self.spawn_bus(if sx > 260.0 { 1.0 } else { -1.0 }, Some(sx));
            } else if b.state == BusState::Drive && (sx - b.x) * b.dir > 70.0 {
                b.target = Some(sx);
            } else {
                b.honk = 0.0;
            }
            crate::log!("Chuyến xe chiều: gọi xe buýt dừng tại x = {sx:.0}");
            return;
        }
        if s > 10.0 && y < self.y_river() - 20.0 {
            if night {
                self.star = Some([x, y, 0.0, if x < w / 2.0 { 0.35 } else { PI - 0.35 }]);
            } else {
                self.flock(x, y, if x < w / 2.0 { 1.0 } else { -1.0 });
            }
            return;
        }
        // Cỏ / đồi / sông: gió rạp lau + hạt bay
        self.gust = Some([x, y, 0.0, 1.0]);
        for _ in 0..8 {
            let (r1, r2, r3, r4) = (self.rng.next(), self.rng.next(), self.rng.next(), self.rng.next());
            if night {
                self.add_part(x + (r1 - 0.5) * 60.0, y + (r2 - 0.5) * 40.0, 2.0, (r3 - 0.5) * 40.0, (r4 - 0.5) * 40.0, 9.0);
            } else {
                self.add_part(x + (r1 - 0.5) * 40.0, y - r2 * 20.0, 1.0, (r3 - 0.3) * 40.0, -20.0 - r4 * 20.0, 8.0);
            }
        }
    }

    fn is_interacting(&self, _now: f64) -> bool {
        self.star.is_some() || self.gust.is_some() || self.bus.state != BusState::Off || self.parts.iter().any(|p| p.kind < 1.5)
    }

    fn destroy(&mut self, gl: &glow::Context) {
        self.b.destroy(gl);
    }

    fn field_ms(&self) -> f64 {
        self.b.field_ms
    }
}
