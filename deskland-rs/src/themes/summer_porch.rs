//! Chủ đề Hiên nhà ngày hạ (prototype/summer-porch.html): phong cách màu nước / Ghibli với hiên gỗ
//! mái ngói âm dương, đàn Corgi mập ngủ trưa lăn mình trước quạt bàn cổ điển, đĩa dưa hấu, sách mở,
//! chuông gió đồng và luống vườn xanh tự động uốn quanh biểu tượng Desktop.
//! Shader dùng chung một nguồn duy nhất: prototype/summer-porch.frag.

use crate::engine::*;
use std::f32::consts::PI;
use windows_sys::Win32::Foundation::SYSTEMTIME;

extern "system" {
    fn GetLocalTime(lpSystemTime: *mut SYSTEMTIME);
}

pub const FRAG: &str = include_str!("../../../prototype/summer-porch.frag");

pub struct Params {
    pub corgis: f32,
    pub fan_speed: f32,
    pub time_of_day: i32,
    pub sunbeams: f32,
    pub sigma: f32,
    pub warp: f32,
}

impl Params {
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        Params {
            corgis: g("corgis", 3.0).clamp(1.0, 4.0),
            fan_speed: g("fanSpeed", 1.0).clamp(0.0, 2.0),
            time_of_day: g("timeOfDay", 0.0).round() as i32,
            sunbeams: g("sunbeams", 0.7).clamp(0.0, 1.0),
            sigma: g("sigma", 78.0).clamp(45.0, 140.0),
            warp: g("warp", 28.0).clamp(0.0, 70.0),
        }
    }
}

struct Corgi {
    nx: f32,
    ny: f32,
    scale: f32,
    coat: f32,
    pose: u8,          // 0 = sploot (đùi gà), 1 = phơi bụng, 2 = nằm nghiêng
    next_pose: u8,
    roll_t: f32,       // <0 = đang nằm yên, 0..1 = đang lăn đổi tư thế
    nap_wait: f32,     // đếm ngược tự lăn mình lúc ngủ
    breath_ph: f32,
    happy: f32,        // 0..1 khi được click gãi bụng
}

struct Bug {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    ph: f32,
    ttl: Option<f32>,
}

pub struct SummerPorch {
    b: Base,
    p: Params,
    rng: Rng,
    corgis: Vec<Corgi>,
    bugs: Vec<Bug>,
    fx: Vec<[f32; 4]>,
    fan_mult: f32,
    fan_osc_ph: f32,
    fan_blade_ph: f32,
    chime_ang: f32,
    chime_vel: f32,
    melon_bites: u8,
    melon_refill: f32,
    bird_hop: f32,
    t0: f64,
}

fn resolve_day_mode(setting: i32) -> f32 {
    match setting {
        0 => 0.0,
        1 => 1.0,
        2 => 2.0,
        _ => unsafe {
            let mut st: SYSTEMTIME = std::mem::zeroed();
            GetLocalTime(&mut st);
            let mins = st.wHour as u32 * 60 + st.wMinute as u32;
            if (360..990).contains(&mins) {
                0.0
            } else if (990..1140).contains(&mins) {
                1.0
            } else {
                2.0
            }
        },
    }
}

impl SummerPorch {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = SummerPorch {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            corgis: Vec::new(),
            bugs: Vec::new(),
            fx: Vec::new(),
            fan_mult: 1.0,
            fan_osc_ph: 0.4,
            fan_blade_ph: 0.0,
            chime_ang: 0.0,
            chime_vel: 0.0,
            melon_bites: 0,
            melon_refill: 0.0,
            bird_hop: 0.0,
            t0: now,
        };
        s.b.commit(gl, &[], s.p.sigma, s.p.warp, true, now);
        s.init_corgis();
        s.sync_bugs();
        s
    }

    fn ref_scale(&self) -> f32 {
        (self.b.w / 1536.0).min(self.b.h / 864.0)
    }

    fn init_corgis(&mut self) {
        let slots: [(f32, f32, f32, f32, u8); 4] = [
            (0.415, 0.508, 1.00, 0.0, 0), // Corgi trái: nằm sấp đùi gà (sploot)
            (0.580, 0.556, 1.06, 0.0, 1), // Corgi phải: phơi bụng trắng trước quạt
            (0.496, 0.522, 0.58, 1.0, 0), // Corgi em bé đeo nơ đỏ nằm giữa
            (0.462, 0.592, 0.88, 2.0, 2), // Corgi tam thể nằm nghiêng mép hiên
        ];
        self.corgis.clear();
        for &(nx, ny, sc, coat, pose) in &slots {
            let wait = 14.0 + self.rng.next() * 14.0;
            let ph = self.rng.next() * PI * 2.0;
            self.corgis.push(Corgi {
                nx,
                ny,
                scale: sc,
                coat,
                pose,
                next_pose: pose,
                roll_t: -1.0,
                nap_wait: wait,
                breath_ph: ph,
                happy: 0.0,
            });
        }
    }

    fn sync_bugs(&mut self) {
        let base_n = ((self.p.sunbeams * 5.0).round() as usize).clamp(1, 6);
        let perm = self.bugs.iter().filter(|b| b.ttl.is_none()).count();
        for _ in perm..base_n {
            let x = self.rng.next() * self.b.w * 0.65;
            let y = self.b.h * (0.20 + self.rng.next() * 0.68);
            let ang = self.rng.next() * PI * 2.0;
            self.bugs.push(Bug {
                x,
                y,
                vx: ang.cos() * 24.0,
                vy: ang.sin() * 14.0,
                ph: self.rng.next() * PI * 2.0,
                ttl: None,
            });
        }
        while self.bugs.iter().filter(|b| b.ttl.is_none()).count() > base_n {
            if let Some(idx) = self.bugs.iter().position(|b| b.ttl.is_none()) {
                self.bugs.remove(idx);
            }
        }
    }

    fn push_fx(&mut self, x: f32, y: f32, kind: f32) {
        if self.fx.len() >= 6 {
            self.fx.remove(0);
        }
        self.fx.push([x, y, 0.0, kind]);
    }

    fn eff_fan_speed(&self) -> f32 {
        (self.p.fan_speed * self.fan_mult).clamp(0.0, 2.4)
    }

    fn step(&mut self, dt: f32) {
        let (w, h) = (self.b.w, self.b.h);
        let fan_spd = self.eff_fan_speed();

        // Quạt bàn quay đầu êm ái & cánh quạt xoay tít
        if fan_spd > 0.02 {
            self.fan_osc_ph = (self.fan_osc_ph + dt * (0.55 + 0.25 * fan_spd)) % (PI * 2.0);
            self.fan_blade_ph = (self.fan_blade_ph + dt * (12.0 + 16.0 * fan_spd)) % (PI * 2.0);
        }

        // Chuông gió đồng đung đưa theo gió tự nhiên + quán tính khi người dùng gõ chuông
        let wind_target = (self.fan_osc_ph * 0.7).sin() * (0.05 + 0.04 * fan_spd);
        let spring = (wind_target - self.chime_ang) * 14.0 - self.chime_vel * 2.6;
        self.chime_vel += spring * dt;
        self.chime_ang = (self.chime_ang + self.chime_vel * dt).clamp(-0.65, 0.65);

        if self.bird_hop > 0.0 {
            self.bird_hop += dt * 2.4;
            if self.bird_hop >= 1.0 {
                self.bird_hop = 0.0;
            }
        }

        if self.melon_bites > 0 {
            self.melon_refill -= dt;
            if self.melon_refill <= 0.0 {
                self.melon_bites = 0;
            }
        }

        // Cập nhật đàn Corgi ngủ trưa & lăn mình đổi tư thế
        let active_n = (self.p.corgis.round() as usize).clamp(1, 4);
        for i in 0..active_n {
            let r_roll = self.rng.next();
            let r_wait = self.rng.next();
            let c = &mut self.corgis[i];
            c.breath_ph = (c.breath_ph + dt * 1.85) % (PI * 2.0);
            c.happy = (c.happy - dt * 0.75).max(0.0);

            if c.roll_t >= 0.0 {
                let prev = c.roll_t;
                c.roll_t += dt * 1.85;
                if prev < 0.5 && c.roll_t >= 0.5 {
                    c.pose = c.next_pose;
                }
                if c.roll_t >= 1.0 {
                    c.roll_t = -1.0;
                }
            } else {
                c.nap_wait -= dt;
                if c.nap_wait <= 0.0 {
                    c.next_pose = if r_roll < 0.55 { (c.pose + 1) % 3 } else { (c.pose + 2) % 3 };
                    c.roll_t = 0.0;
                    c.nap_wait = 16.0 + r_wait * 18.0;
                }
            }
        }

        // Bướm hè / đom đóm bay lượn quanh khu vườn & sân trước
        for i in 0..self.bugs.len() {
            let r1 = self.rng.next();
            let r2 = self.rng.next();
            let bg = &mut self.bugs[i];
            bg.ph = (bg.ph + dt * 11.5) % (PI * 2.0);
            bg.vx = (bg.vx + (r1 - 0.5) * 90.0 * dt).clamp(-52.0, 52.0);
            bg.vy = (bg.vy + (r2 - 0.5) * 70.0 * dt).clamp(-36.0, 36.0);
            bg.x += bg.vx * dt;
            bg.y += bg.vy * dt;
            if bg.x < 16.0 {
                bg.vx = bg.vx.abs();
            } else if bg.x > w * 0.72 {
                bg.vx = -bg.vx.abs();
            }
            if bg.y < h * 0.14 {
                bg.vy = bg.vy.abs();
            } else if bg.y > h - TASKBAR - 24.0 {
                bg.vy = -bg.vy.abs();
            }
            if let Some(t) = bg.ttl.as_mut() {
                *t -= dt;
            }
        }
        self.bugs.retain(|b| b.ttl.map_or(true, |t| t > 0.0));

        self.fx.retain_mut(|f| {
            f[2] += dt;
            f[2] < 1.65
        });
    }

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        let (w, h) = (self.b.w, self.b.h);
        let active_n = (self.p.corgis.round() as usize).clamp(1, 4);

        let mut ca = [0f32; 16];
        let mut cb = [0f32; 16];
        for (i, c) in self.corgis.iter().take(active_n).enumerate() {
            ca[i * 4..i * 4 + 4].copy_from_slice(&[c.nx * w, c.ny * h, c.pose as f32, c.roll_t.max(0.0)]);
            cb[i * 4..i * 4 + 4].copy_from_slice(&[c.scale, c.coat, c.breath_ph, c.happy]);
        }

        let mut bugs_u = [0f32; 40];
        let bug_n = self.bugs.len().min(10);
        for (i, b) in self.bugs.iter().take(bug_n).enumerate() {
            bugs_u[i * 4..i * 4 + 4].copy_from_slice(&[b.x, b.y, if b.vx >= 0.0 { 1.0 } else { -1.0 }, b.ph]);
        }

        let mut fx_u = [-1f32; 24];
        for (i, f) in self.fx.iter().take(6).enumerate() {
            fx_u[i * 4..i * 4 + 4].copy_from_slice(f);
        }

        let fan_pos_x = 0.704 * w;
        let fan_pos_y = 0.518 * h;
        let fan_yaw = self.fan_osc_ph.sin() * 0.85;

        let p = &mut sh.prog;
        unsafe { glow::HasContext::use_program(gl, Some(p.prog)) };
        self.b.bind_common(gl, p, sh.noise_tex, sh.seed, t);
        p.f1(gl, "uDayMode", resolve_day_mode(self.p.time_of_day));
        p.f1(gl, "uSunbeams", self.p.sunbeams);
        p.f1(gl, "uFanSpeed", self.eff_fan_speed());
        p.f4v(gl, "uFanState", &[fan_pos_x, fan_pos_y, fan_yaw, self.fan_blade_ph]);
        p.f4v(gl, "uProps", &[self.chime_ang, -1.0, self.melon_bites as f32, self.bird_hop]);
        p.f1(gl, "uCorgiN", active_n as f32);
        p.f4v(gl, "uCorgiA", &ca);
        p.f4v(gl, "uCorgiB", &cb);
        p.f1(gl, "uBugN", bug_n as f32);
        p.f4v(gl, "uBugs", &bugs_u);
        p.f4v(gl, "uFx", &fx_u);
        unsafe { glow::HasContext::draw_arrays(gl, glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for SummerPorch {
    fn set_icons(&mut self, gl: &glow::Context, icons: Vec<IconBox>, first: bool, now: f64) {
        self.b.icons = icons;
        let rects = icon_rects(&self.b.icons);
        self.b.commit(gl, &rects, self.p.sigma, self.p.warp, first, now);
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, now: f64) {
        self.b.update_mix(now);
        self.step(dt.min(0.1));
        self.draw(gl, sh, now);
    }

    fn settle(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        self.b.settle();
        self.draw(gl, sh, now);
    }

    fn handle_click(&mut self, _gl: &glow::Context, x: f32, y: f32, _now: f64) {
        let (w, h) = (self.b.w, self.b.h);
        let s = self.ref_scale();
        let active_n = (self.p.corgis.round() as usize).clamp(1, 4);

        // 1. Click trúng chú Corgi trên hiên gỗ -> gãi bụng, lăn mình đổi tư thế + bắn tim hồng
        for i in 0..active_n {
            let cx = self.corgis[i].nx * w;
            let cy = self.corgis[i].ny * h;
            let hit_r = 78.0 * self.corgis[i].scale * s;
            if (x - cx).hypot(y - cy) <= hit_r {
                let c = &mut self.corgis[i];
                c.next_pose = (c.pose + 1) % 3;
                c.roll_t = 0.0;
                c.happy = 1.0;
                c.nap_wait = 18.0;
                self.push_fx(cx, cy - 28.0 * s, 0.0);
                return;
            }
        }

        // 2. Click vào chiếc quạt bàn -> đổi nấc gió quạt (Mặc định -> Mạnh -> Nhẹ -> Tắt)
        let fan_cx = 0.704 * w;
        let fan_cy = 0.518 * h - 58.0 * s;
        if (x - fan_cx).hypot(y - fan_cy) <= 68.0 * s {
            self.fan_mult = if (self.fan_mult - 1.0).abs() < 0.1 {
                1.8
            } else if self.fan_mult > 1.4 {
                0.4
            } else if self.fan_mult > 0.1 {
                0.0
            } else {
                1.0
            };
            if self.eff_fan_speed() > 0.05 {
                self.push_fx(fan_cx - 24.0 * s, fan_cy, 1.0);
            }
            return;
        }

        // 3. Click vào chuông gió đồng dưới mái hiên -> leng keng + chim xanh nhảy lên
        let chime_cx = 0.346 * w;
        let chime_cy = 0.095 * h + 58.0 * s;
        if (x - chime_cx).hypot(y - chime_cy) <= 52.0 * s {
            self.chime_vel = if self.chime_ang >= 0.0 { -4.2 } else { 4.2 };
            self.bird_hop = 0.01;
            self.push_fx(chime_cx, chime_cy, 2.0);
            return;
        }

        // 4. Click vào đĩa dưa hấu đỏ -> cắn 1 miếng dưa mát lạnh!
        let melon_cx = 0.702 * w + 32.0 * s;
        let melon_cy = 0.592 * h - 6.0 * s;
        if (x - melon_cx).hypot(y - melon_cy) <= 52.0 * s {
            self.melon_bites = (self.melon_bites + 1) % 4;
            self.melon_refill = 6.5;
            self.push_fx(melon_cx, melon_cy - 6.0 * s, 3.0);
            return;
        }

        // 5. Click vào chim non màu xanh bên mẹt rau -> chim nhảy lích chích
        let bird_cx = 0.848 * w;
        let bird_cy = 0.676 * h;
        if (x - bird_cx).hypot(y - bird_cy) <= 36.0 * s {
            self.bird_hop = 0.01;
            self.push_fx(bird_cx, bird_cy - 12.0 * s, 0.0);
            return;
        }

        // 6. Click vùng sân nắng / khu vườn -> thả bướm hè (hoặc đom đóm đêm) bay lượn
        if self.bugs.len() < 10 {
            let ang = self.rng.next() * PI * 2.0;
            self.bugs.push(Bug {
                x,
                y,
                vx: ang.cos() * 34.0,
                vy: -18.0 + ang.sin() * 14.0,
                ph: self.rng.next() * PI * 2.0,
                ttl: Some(9.0),
            });
        }
        self.push_fx(x, y, 4.0);
    }

    fn is_interacting(&self, _now: f64) -> bool {
        !self.fx.is_empty()
            || self.bird_hop > 0.0
            || self.chime_vel.abs() > 0.35
            || self.corgis.iter().any(|c| c.roll_t >= 0.0 || c.happy > 0.05)
    }

    fn destroy(&mut self, gl: &glow::Context) {
        self.b.destroy(gl);
    }

    fn field_ms(&self) -> f64 {
        self.b.field_ms
    }
}
