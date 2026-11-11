//! Chủ đề Sân bóng sôi động (stadium): góc nhìn sân vận động từ trên cao (Top-Down Aerial View),
//! hai đội Đỏ - Xanh chạy chỗ chiến thuật, ban bật chuyền bóng và dứt điểm.
//! Click chuột trái vào mặt sân: quả bóng lập tức lăn/nảy tới điểm click và các cầu thủ bứt tốc đuổi theo bóng.

use crate::engine::*;
use glow::HasContext;
use std::f32::consts::PI;

pub const FRAG: &str = include_str!("../../assets/shaders/stadium.frag");

pub struct Params {
    pub players: f32,
    pub speed: f32,
    pub pass_freq: f32,
    pub lighting: f32,
    pub stripes: f32,
}

impl Params {
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        Params {
            players: g("players", 6.0).clamp(3.0, 11.0),
            speed: g("speed", 1.0).clamp(0.3, 2.5),
            pass_freq: g("passFreq", 1.0).clamp(0.3, 2.5),
            lighting: g("lighting", 0.0).clamp(0.0, 2.0),
            stripes: g("stripes", 0.65).clamp(0.0, 1.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    GK,
    Def,
    Mid,
    Fwd,
}

struct Player {
    team: u8, // 0 = Đội Đỏ (tấn công sang phải), 1 = Đội Xanh (tấn công sang trái)
    role: Role,
    hx: f32, // vị trí chiến thuật gốc (0..1 theo mặt sân)
    hy: f32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    dir: f32,
    stride: f32,
    hair: f32,
    wander_ph: f32,
    chase_rank: Option<u8>, // Some(0..3) khi đang bứt tốc đuổi bóng từ cú click của người dùng
}

struct Ball {
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    drag: f32,
    spin: f32,
    owner: Option<usize>,
    last_kicker: Option<usize>,
    pickup_lock: f32,
    hold_timer: f32,
    pass_target_player: Option<usize>,
    is_goal_shot: bool,
    trail_origin: (f32, f32),
    kick_age: f32,
}

pub struct Stadium {
    b: Base,
    p: Params,
    rng: Rng,
    players: Vec<Player>,
    ball: Ball,
    target_ping: Option<[f32; 4]>, // x, y, age, active
    goal_fx: Option<[f32; 4]>,     // x, y, age, side (-1.0 trái, +1.0 phải)
    chase_timer: f32,
    roam_t: f32,
    t0: f64,
}

fn turn_angle(cur: f32, target: f32, max_step: f32) -> f32 {
    let diff = (target - cur).sin().atan2((target - cur).cos());
    cur + diff.signum() * diff.abs().min(max_step)
}

impl Stadium {
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, now: f64) -> Self {
        let mut s = Stadium {
            b: Base::new(gl, w, h, buf_w, buf_h, scale, seed),
            p,
            rng: Rng::new(rng_seed),
            players: Vec::new(),
            ball: Ball {
                x: w * 0.5,
                y: (h - TASKBAR) * 0.5,
                z: 0.0,
                vx: 0.0,
                vy: 0.0,
                vz: 0.0,
                drag: 2.2,
                spin: 0.0,
                owner: None,
                last_kicker: None,
                pickup_lock: 0.0,
                hold_timer: 0.8,
                pass_target_player: None,
                is_goal_shot: false,
                trail_origin: (w * 0.5, (h - TASKBAR) * 0.5),
                kick_age: 10.0,
            },
            target_ping: None,
            goal_fx: None,
            chase_timer: 0.0,
            roam_t: 0.0,
            t0: now,
        };
        s.b.commit(gl, &[], 80.0, 28.0, true, now);
        s.build_match();
        s
    }

    fn ui_scale(&self) -> f32 {
        (self.b.w.min(self.b.h) / 900.0).max(0.5)
    }

    /// Trả về tọa độ mặt sân cỏ chính `[x0, y0, x1, y1]` đồng bộ tuyệt đối với `stadium.frag`.
    fn pitch_rect(&self) -> [f32; 4] {
        let s = self.ui_scale();
        let avail_h = (self.b.h - TASKBAR).max(220.0);
        let pad_x = (92.0 * s).max(self.b.w * 0.085);
        let pad_y = (58.0 * s).max(avail_h * 0.085);
        [pad_x, pad_y, (self.b.w - pad_x).max(pad_x + 200.0), (avail_h - pad_y).max(pad_y + 140.0)]
    }

    /// Chia số cầu thủ mỗi đội (3..=11) thành sơ đồ chiến thuật (Hậu vệ, Tiền vệ, Tiền đạo).
    fn split_formation(per_team: usize) -> (usize, usize, usize) {
        match per_team.clamp(3, 11) - 1 {
            2 => (1, 0, 1),
            3 => (1, 1, 1),
            4 => (2, 1, 1),
            5 => (2, 2, 1), // Mặc định 6v6: GK + 2 HV + 2 TV + 1 TĐ
            6 => (2, 2, 2),
            7 => (3, 2, 2),
            8 => (3, 3, 2),
            9 => (4, 3, 2),
            _ => (4, 3, 3), // 11v11 chuẩn: GK + 4-3-3
        }
    }

    fn line_ys(count: usize, spread: f32) -> Vec<f32> {
        if count == 0 {
            return Vec::new();
        }
        if count == 1 {
            return vec![0.50];
        }
        (0..count)
            .map(|i| 0.50 - spread * 0.5 + spread * (i as f32 / (count - 1) as f32))
            .collect()
    }

    fn build_match(&mut self) {
        let per_team = (self.p.players.round() as usize).clamp(3, 11);
        let (n_def, n_mid, n_fwd) = Self::split_formation(per_team);
        let [x0, y0, x1, y1] = self.pitch_rect();
        let (pw, ph) = (x1 - x0, y1 - y0);

        self.players.clear();
        for team in 0..2u8 {
            let mut slots: Vec<(Role, f32, f32)> = Vec::with_capacity(per_team);
            slots.push((Role::GK, 0.042, 0.50));
            for y in Self::line_ys(n_def, 0.62) {
                slots.push((Role::Def, 0.20, y));
            }
            for y in Self::line_ys(n_mid, 0.66) {
                slots.push((Role::Mid, 0.36, y));
            }
            for y in Self::line_ys(n_fwd, 0.54) {
                slots.push((Role::Fwd, 0.47, y));
            }

            for (role, base_hx, hy) in slots {
                let hx = if team == 0 { base_hx } else { 1.0 - base_hx };
                let px = x0 + hx * pw + (self.rng.next() - 0.5) * 14.0;
                let py = y0 + hy * ph + (self.rng.next() - 0.5) * 14.0;
                self.players.push(Player {
                    team,
                    role,
                    hx,
                    hy,
                    x: px,
                    y: py,
                    vx: 0.0,
                    vy: 0.0,
                    dir: if team == 0 { 0.0 } else { PI },
                    stride: self.rng.next() * PI * 2.0,
                    hair: self.rng.next(),
                    wander_ph: self.rng.next() * PI * 2.0,
                    chase_rank: None,
                });
            }
        }

        // Giao bóng đầu trận cho cầu thủ tuyến trên của Đội Đỏ
        let starter = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.team == 0 && p.role != Role::GK)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        let s = self.ui_scale();
        self.ball.owner = Some(starter);
        self.ball.hold_timer = 0.85;
        self.ball.x = self.players[starter].x + 10.6 * s;
        self.ball.y = self.players[starter].y;
        self.ball.z = 0.0;
        self.ball.vx = 0.0;
        self.ball.vy = 0.0;
        self.ball.vz = 0.0;
        self.ball.trail_origin = (self.ball.x, self.ball.y);
    }

    fn near_icon(&self, x: f32, y: f32) -> bool {
        self.b.icons.iter().any(|ic| {
            x >= ic.x - 18.0 && x <= ic.x + ic.w + 18.0 && y >= ic.y - 18.0 && y <= ic.y + ic.h + 18.0
        })
    }

    /// Đá quả bóng từ vị trí hiện tại tới đích `(tx, ty)` sao cho bóng lăn tới đúng đích với ma sát cỏ.
    fn kick_ball_to(&mut self, tx: f32, ty: f32, base_drag: f32, max_speed: f32, loft_factor: f32) {
        let dx = tx - self.ball.x;
        let dy = ty - self.ball.y;
        let dist = dx.hypot(dy).max(1.0);
        let mut drag = base_drag;
        let mut v0 = dist * drag;
        if v0 > max_speed {
            v0 = max_speed;
            drag = (v0 / dist).max(0.95);
        }
        self.ball.vx = (dx / dist) * v0;
        self.ball.vy = (dy / dist) * v0;
        self.ball.vz = (dist * loft_factor).clamp(0.0, 105.0);
        self.ball.drag = drag;
        self.ball.trail_origin = (self.ball.x, self.ball.y);
        self.ball.kick_age = 0.001;
    }

    /// Kích hoạt bàn thắng: rung lưới + pháo giấy ăn mừng!
    fn trigger_goal(&mut self, side: f32) {
        let [x0, y0, x1, y1] = self.pitch_rect();
        let s = self.ui_scale();
        let cy = 0.5 * (y0 + y1);
        let gd = 28.0 * s;
        let gx = if side < 0.0 { x0 - gd * 0.55 } else { x1 + gd * 0.55 };
        let gy = self.ball.y.clamp(cy - (y1 - y0) * 0.09, cy + (y1 - y0) * 0.09);

        self.ball.x = gx;
        self.ball.y = gy;
        self.ball.z = 0.0;
        self.ball.vx = 0.0;
        self.ball.vy = 0.0;
        self.ball.vz = 0.0;
        self.ball.owner = None;
        self.ball.pass_target_player = None;
        self.ball.is_goal_shot = false;
        self.chase_timer = 0.0;
        for pl in &mut self.players {
            pl.chase_rank = None;
        }
        self.goal_fx = Some([if side < 0.0 { x0 } else { x1 }, gy, 0.0, side]);
    }

    /// Chọn đường chuyền thông minh hoặc cú sút cầu môn cho cầu thủ `owner_idx`.
    fn decide_pass_or_shot(&mut self, owner_idx: usize) {
        let [x0, y0, x1, y1] = self.pitch_rect();
        let (pw, ph) = (x1 - x0, y1 - y0);
        let cy = 0.5 * (y0 + y1);
        let gh = ph * 0.22;
        let s = self.ui_scale();
        let tempo = self.p.speed.max(0.4);

        let team = self.players[owner_idx].team;
        let (ox, oy) = (self.players[owner_idx].x, self.players[owner_idx].y);
        let goal_x = if team == 0 { x1 } else { x0 };
        let dist_goal_x = (goal_x - ox).abs();

        // 1. Cơ hội tung cú sút về khung thành khi áp sát vòng 16m50!
        if self.players[owner_idx].role != Role::GK
            && dist_goal_x < pw * 0.25
            && (oy - cy).abs() < ph * 0.30
            && self.rng.next() < 0.58
        {
            let aim_y = cy + (self.rng.next() - 0.5) * gh * 0.72;
            let aim_x = if team == 0 { x1 + 18.0 * s } else { x0 - 18.0 * s };
            self.ball.owner = None;
            self.ball.last_kicker = Some(owner_idx);
            self.ball.pickup_lock = 0.26;
            self.ball.pass_target_player = None;
            self.ball.is_goal_shot = true;
            self.kick_ball_to(aim_x, aim_y, 1.55, 680.0 * s * tempo.sqrt(), 0.16);
            self.players[owner_idx].dir = (aim_y - oy).atan2(aim_x - ox);
            return;
        }

        // 2. Chấm điểm tìm đồng đội thoáng nhất để ban bật chuyền bóng
        let mut best: Option<(usize, f32)> = None;
        for i in 0..self.players.len() {
            if i == owner_idx || self.players[i].team != team {
                continue;
            }
            // Hạn chế chuyền ngược về thủ môn trừ khi bị áp sát ở sân nhà
            if self.players[i].role == Role::GK && self.rng.next() > 0.12 {
                continue;
            }
            let (tx, ty) = (self.players[i].x, self.players[i].y);
            let d = (tx - ox).hypot(ty - oy);
            if d < 42.0 * s || d > pw * 0.68 {
                continue;
            }

            // Ưu tiên chuyền hướng lên tấn công + đồng đội không bị đối phương kèm sát
            let forward_gain = if team == 0 { tx - ox } else { ox - tx };
            let nearest_opp = self
                .players
                .iter()
                .filter(|op| op.team != team)
                .map(|op| (op.x - tx).hypot(op.y - ty))
                .fold(9999.0f32, f32::min);
            let icon_pen = if self.near_icon(tx, ty) { -45.0 * s } else { 0.0 };
            let jitter = (self.rng.next() - 0.5) * 95.0 * s;
            let score = forward_gain * 0.55 + nearest_opp.min(140.0 * s) * 0.65 - (d - 180.0 * s).abs() * 0.18 + icon_pen + jitter;

            if best.map_or(true, |(_, bs)| score > bs) {
                best = Some((i, score));
            }
        }

        if let Some((target_idx, _)) = best {
            // Đón đầu hướng chạy của đồng đội
            let lead_x = (self.players[target_idx].x + self.players[target_idx].vx * 0.28).clamp(x0 + 14.0 * s, x1 - 14.0 * s);
            let lead_y = (self.players[target_idx].y + self.players[target_idx].vy * 0.28).clamp(y0 + 14.0 * s, y1 - 14.0 * s);
            let dist = (lead_x - ox).hypot(lead_y - oy);
            let loft = if dist > pw * 0.34 { 0.20 } else { 0.06 };
            self.ball.owner = None;
            self.ball.last_kicker = Some(owner_idx);
            self.ball.pickup_lock = 0.24;
            self.ball.pass_target_player = Some(target_idx);
            self.ball.is_goal_shot = false;
            self.kick_ball_to(lead_x, lead_y, 2.05, 560.0 * s * tempo.sqrt(), loft);
            self.players[owner_idx].dir = (lead_y - oy).atan2(lead_x - ox);
        } else {
            self.ball.hold_timer = 0.45;
        }
    }

    fn step(&mut self, dt: f32) {
        let dt = dt.min(0.06);
        let s = self.ui_scale();
        let tempo = self.p.speed.max(0.35);
        self.roam_t += dt * tempo;

        let [x0, y0, x1, y1] = self.pitch_rect();
        let (pw, ph) = (x1 - x0, y1 - y0);
        let (cx, cy) = (0.5 * (x0 + x1), 0.5 * (y0 + y1));
        let gh = ph * 0.22;

        // Cập nhật vòng sáng mục tiêu Click
        if let Some(mut tp) = self.target_ping {
            tp[2] += dt;
            if tp[2] >= 1.8 {
                self.target_ping = None;
            } else {
                self.target_ping = Some(tp);
            }
        }
        if self.chase_timer > 0.0 {
            self.chase_timer = (self.chase_timer - dt).max(0.0);
            if self.chase_timer <= 0.0 {
                for pl in &mut self.players {
                    pl.chase_rank = None;
                }
            }
        }
        if self.ball.kick_age < 2.0 {
            self.ball.kick_age += dt;
        }

        // 1. Đang trong nhịp ăn mừng bàn thắng (Goal!)
        if let Some(mut gfx) = self.goal_fx {
            gfx[2] += dt;
            let scoring_team = if gfx[3] > 0.0 { 0u8 } else { 1u8 };
            for pl in &mut self.players {
                let target_x = x0 + pl.hx * pw;
                let target_y = y0 + pl.hy * ph;
                let (dx, dy) = (target_x - pl.x, target_y - pl.y);
                let dist = dx.hypot(dy);
                let spd = if pl.team == scoring_team { 75.0 * s } else { 95.0 * s };
                if dist > 6.0 * s {
                    pl.vx = (dx / dist) * spd;
                    pl.vy = (dy / dist) * spd;
                    pl.x += pl.vx * dt;
                    pl.y += pl.vy * dt;
                    pl.dir = turn_angle(pl.dir, dy.atan2(dx), 7.5 * dt);
                    pl.stride += (spd / (14.0 * s)) * dt;
                } else {
                    pl.vx = 0.0;
                    pl.vy = 0.0;
                }
            }
            if gfx[2] >= 2.15 {
                self.goal_fx = None;
                // Giao bóng lại từ chấm giữa sân cho đội vừa thủng lưới
                let restart_team = 1 - scoring_team;
                let kicker = self
                    .players
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.team == restart_team && p.role != Role::GK)
                    .min_by(|(_, a), (_, b)| {
                        let da = (a.x - cx).hypot(a.y - cy);
                        let db = (b.x - cx).hypot(b.y - cy);
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                self.players[kicker].x = cx + if restart_team == 0 { -8.0 * s } else { 8.0 * s };
                self.players[kicker].y = cy;
                self.ball.x = cx;
                self.ball.y = cy;
                self.ball.z = 0.0;
                self.ball.vx = 0.0;
                self.ball.vy = 0.0;
                self.ball.vz = 0.0;
                self.ball.owner = Some(kicker);
                self.ball.hold_timer = 0.8;
                self.ball.trail_origin = (cx, cy);
            } else {
                self.goal_fx = Some(gfx);
            }
            return;
        }

        // 2. Cập nhật quả bóng (khi đang tự do lăn trên cỏ hoặc trong chân cầu thủ)
        if self.ball.pickup_lock > 0.0 {
            self.ball.pickup_lock = (self.ball.pickup_lock - dt).max(0.0);
        }

        if let Some(owner_idx) = self.ball.owner {
            // Cầu thủ đang dẫn bóng trong chân
            let goal_dir = if self.players[owner_idx].team == 0 { 1.0 } else { -1.0 };
            let mut drib_vx = goal_dir * 72.0 * s * tempo;
            let mut drib_vy = (self.roam_t * 2.1 + self.players[owner_idx].wander_ph).sin() * 44.0 * s * tempo;

            // Lách nhẹ tránh cầu thủ đối phương trước mặt & vùng dưới icon
            for j in 0..self.players.len() {
                if self.players[j].team != self.players[owner_idx].team {
                    let dx = self.players[owner_idx].x - self.players[j].x;
                    let dy = self.players[owner_idx].y - self.players[j].y;
                    let d = dx.hypot(dy).max(1.0);
                    if d < 56.0 * s {
                        drib_vx += (dx / d) * 48.0 * s * tempo;
                        drib_vy += (dy / d) * 62.0 * s * tempo;
                    }
                }
            }
            // Giữ hướng dẫn bóng bên trong biên sân
            if self.players[owner_idx].y < y0 + 42.0 * s {
                drib_vy += 55.0 * s;
            } else if self.players[owner_idx].y > y1 - 42.0 * s {
                drib_vy -= 55.0 * s;
            }
            if self.players[owner_idx].role == Role::GK {
                drib_vx = goal_dir * 38.0 * s;
                drib_vy *= 0.25;
            }

            let d_spd = drib_vx.hypot(drib_vy).max(1.0);
            let cap = 82.0 * s * tempo;
            if d_spd > cap {
                drib_vx = drib_vx / d_spd * cap;
                drib_vy = drib_vy / d_spd * cap;
            }
            let pl = &mut self.players[owner_idx];
            pl.vx = drib_vx;
            pl.vy = drib_vy;
            pl.x = (pl.x + pl.vx * dt).clamp(x0 + 14.0 * s, x1 - 14.0 * s);
            pl.y = (pl.y + pl.vy * dt).clamp(y0 + 14.0 * s, y1 - 14.0 * s);
            pl.dir = turn_angle(pl.dir, pl.vy.atan2(pl.vx), 9.0 * dt);
            pl.stride += (pl.vx.hypot(pl.vy) / (12.5 * s)) * dt;

            self.ball.x = pl.x + pl.dir.cos() * 10.8 * s;
            self.ball.y = pl.y + pl.dir.sin() * 10.8 * s;
            self.ball.z = 0.0;
            self.ball.vx = pl.vx;
            self.ball.vy = pl.vy;
            self.ball.spin += (pl.vx.hypot(pl.vy) / (7.5 * s)) * dt;

            self.ball.hold_timer -= dt * self.p.pass_freq;
            // Nếu bị đối phương áp sát < 20px thì chuyền sớm hơn hoặc bị cướp bóng
            let pressed = self
                .players
                .iter()
                .enumerate()
                .find(|&(_, op)| op.team != self.players[owner_idx].team && (op.x - self.ball.x).hypot(op.y - self.ball.y) < 16.5 * s)
                .map(|(i, _)| i);

            if let Some(tackler) = pressed {
                if self.rng.next() < 0.35 {
                    self.ball.owner = Some(tackler);
                    self.ball.hold_timer = 0.75 + self.rng.next() * 0.65;
                } else {
                    self.decide_pass_or_shot(owner_idx);
                }
            } else if self.ball.hold_timer <= 0.0 {
                self.decide_pass_or_shot(owner_idx);
            }
        } else {
            // Quả bóng đang lăn / bay tự do trên mặt cỏ
            self.ball.x += self.ball.vx * dt;
            self.ball.y += self.ball.vy * dt;
            let damp = (-self.ball.drag * dt).exp();
            self.ball.vx *= damp;
            self.ball.vy *= damp;
            let b_spd = self.ball.vx.hypot(self.ball.vy);
            if b_spd < 4.0 {
                self.ball.vx = 0.0;
                self.ball.vy = 0.0;
            }
            self.ball.spin += (b_spd / (6.8 * s)) * dt;

            // Độ cao nảy 3D trên mặt cỏ
            if self.ball.z > 0.0 || self.ball.vz != 0.0 {
                self.ball.z += self.ball.vz * dt;
                self.ball.vz -= 295.0 * dt;
                if self.ball.z <= 0.0 {
                    self.ball.z = 0.0;
                    if self.ball.vz < -22.0 {
                        self.ball.vz = -self.ball.vz * 0.42;
                    } else {
                        self.ball.vz = 0.0;
                    }
                }
            }

            // Kiểm tra bóng lăn qua vạch vôi cầu môn -> Ghi bàn!
            let in_goal_mouth_y = (self.ball.y - cy).abs() < gh * 0.48;
            if in_goal_mouth_y && self.ball.x < x0 - 3.5 * s {
                self.trigger_goal(-1.0);
                return;
            }
            if in_goal_mouth_y && self.ball.x > x1 + 3.5 * s {
                self.trigger_goal(1.0);
                return;
            }

            // Nảy nhẹ nếu chạm cột dọc hoặc lăn sát mép ngoài thảm cỏ biên
            if !in_goal_mouth_y && !self.ball.is_goal_shot {
                let min_bx = x0 - 22.0 * s;
                let max_bx = x1 + 22.0 * s;
                let min_by = y0 - 18.0 * s;
                let max_by = y1 + 18.0 * s;
                if self.ball.x < min_bx {
                    self.ball.x = min_bx;
                    self.ball.vx = self.ball.vx.abs() * 0.45;
                } else if self.ball.x > max_bx {
                    self.ball.x = max_bx;
                    self.ball.vx = -self.ball.vx.abs() * 0.45;
                }
                if self.ball.y < min_by {
                    self.ball.y = min_by;
                    self.ball.vy = self.ball.vy.abs() * 0.45;
                } else if self.ball.y > max_by {
                    self.ball.y = max_by;
                    self.ball.vy = -self.ball.vy.abs() * 0.45;
                }
            }
        }

        // 3. Di chuyển AI cho toàn bộ cầu thủ (trừ người đang dẫn bóng đã cập nhật ở trên)
        let ball_norm_x = ((self.ball.x - x0) / pw.max(1.0)).clamp(0.0, 1.0);
        let ball_norm_y = ((self.ball.y - y0) / ph.max(1.0)).clamp(0.0, 1.0);
        let current_owner = self.ball.owner;

        // Tìm cầu thủ gần bóng nhất của mỗi đội để áp sát tranh bóng tự nhiên
        let mut closest_per_team = [None::<usize>; 2];
        if current_owner.is_none() && self.chase_timer <= 0.0 {
            for t_id in 0..2usize {
                closest_per_team[t_id] = self
                    .players
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.team as usize == t_id && p.role != Role::GK)
                    .min_by(|(_, a), (_, b)| {
                        let da = (a.x - self.ball.x).hypot(a.y - self.ball.y);
                        let db = (b.x - self.ball.x).hypot(b.y - self.ball.y);
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i);
            }
        } else if let Some(own) = current_owner {
            let def_team = (1 - self.players[own].team) as usize;
            closest_per_team[def_team] = self
                .players
                .iter()
                .enumerate()
                .filter(|(_, p)| p.team as usize == def_team && p.role != Role::GK)
                .min_by(|(_, a), (_, b)| {
                    let da = (a.x - self.ball.x).hypot(a.y - self.ball.y);
                    let db = (b.x - self.ball.x).hypot(b.y - self.ball.y);
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i);
        }

        for i in 0..self.players.len() {
            if current_owner == Some(i) {
                continue;
            }
            let pl_team = self.players[i].team;
            let pl_role = self.players[i].role;
            let chase_rk = self.players[i].chase_rank;

            let (tx, ty, speed_mul) = if pl_role == Role::GK {
                // Thủ môn di chuyển dọc vạch cầu môn theo tọa độ Y của bóng
                let gk_x = if pl_team == 0 { x0 + 18.0 * s } else { x1 - 18.0 * s };
                let gk_y = cy + (self.ball.y - cy).clamp(-gh * 0.44, gh * 0.44);
                (gk_x, gk_y, 1.15)
            } else if let Some(rk) = chase_rk {
                // Đang bứt tốc chạy theo bóng từ cú Click chuột của người dùng!
                let (ox, oy) = if rk == 0 {
                    (0.0, 0.0)
                } else {
                    let ang = rk as f32 * 1.9 + 0.5;
                    (ang.cos() * 18.0 * s, ang.sin() * 16.0 * s)
                };
                (self.ball.x + ox, self.ball.y + oy, if rk == 0 { 1.52 } else { 1.38 })
            } else if self.ball.pass_target_player == Some(i) && current_owner.is_none() {
                // Cầu thủ nhận đường chuyền băng lên đón bóng
                (self.ball.x + self.ball.vx * 0.18, self.ball.y + self.ball.vy * 0.18, 1.28)
            } else if closest_per_team[pl_team as usize] == Some(i) {
                // Cầu thủ gần nhất băng vào tranh chấp / pressing
                (self.ball.x, self.ball.y, 1.22)
            } else {
                // Chạy chỗ đội hình chiến thuật dâng lên / lùi về nhịp nhàng theo bóng
                let shift_x = (ball_norm_x - 0.5) * 0.36;
                let shift_y = (ball_norm_y - 0.5) * 0.22;
                let attack_push = if current_owner.map_or(false, |o| self.players[o].team == pl_team) {
                    if pl_team == 0 { 0.12 } else { -0.12 }
                } else {
                    0.0
                };
                let nx = (self.players[i].hx + shift_x + attack_push).clamp(0.08, 0.92);
                let ny = (self.players[i].hy + shift_y).clamp(0.08, 0.92);
                let wx = (self.roam_t * 1.3 + self.players[i].wander_ph).cos() * 20.0 * s;
                let wy = (self.roam_t * 1.6 + self.players[i].wander_ph * 1.3).sin() * 18.0 * s;
                (x0 + nx * pw + wx, y0 + ny * ph + wy, 0.88)
            };

            let dx = tx - self.players[i].x;
            let dy = ty - self.players[i].y;
            let dist = dx.hypot(dy);
            let max_spd = 118.0 * s * tempo * speed_mul;

            if dist > 2.5 * s {
                let spd = (dist * 4.2).min(max_spd);
                self.players[i].vx = (dx / dist) * spd;
                self.players[i].vy = (dy / dist) * spd;
            } else {
                self.players[i].vx *= 0.75;
                self.players[i].vy *= 0.75;
            }

            // Đẩy tách nhẹ giữa các cầu thủ để không bao giờ giẫm chồng hình lên nhau
            let mut sep_x = 0.0f32;
            let mut sep_y = 0.0f32;
            for j in 0..self.players.len() {
                if i == j {
                    continue;
                }
                let rx = self.players[i].x - self.players[j].x;
                let ry = self.players[i].y - self.players[j].y;
                let rd = rx.hypot(ry);
                let min_sep = 23.0 * s;
                if rd > 0.01 && rd < min_sep {
                    let push = (min_sep - rd) / min_sep;
                    sep_x += (rx / rd) * push * 75.0 * s;
                    sep_y += (ry / rd) * push * 75.0 * s;
                }
            }

            let pl = &mut self.players[i];
            pl.vx += sep_x;
            pl.vy += sep_y;
            pl.x = (pl.x + pl.vx * dt).clamp(x0 - 20.0 * s, x1 + 20.0 * s);
            pl.y = (pl.y + pl.vy * dt).clamp(y0 - 16.0 * s, y1 + 16.0 * s);

            let actual_spd = pl.vx.hypot(pl.vy);
            if actual_spd > 6.0 * s {
                pl.dir = turn_angle(pl.dir, pl.vy.atan2(pl.vx), 9.5 * dt);
                pl.stride += (actual_spd / (12.2 * s)) * dt;
            } else {
                // Đứng nghỉ thì quay mặt nhìn về phía quả bóng
                let look = (self.ball.y - pl.y).atan2(self.ball.x - pl.x);
                pl.dir = turn_angle(pl.dir, look, 5.0 * dt);
            }
        }

        // 4. Kiểm tra cầu thủ chạm & khống chế bóng (khi bóng đang lăn tự do)
        if self.ball.owner.is_none() && self.ball.z < 18.0 {
            let b_spd = self.ball.vx.hypot(self.ball.vy);
            // Khi đang chạy theo cú Click của người dùng, đợi bóng lăn gần tới điểm Click (b_spd giảm)
            // hoặc hết thời gian khóa ngắn rồi cầu thủ mới khống chế bóng để nối lại mạch chuyền
            let can_field_pickup = self.ball.pickup_lock <= 0.0 && (self.chase_timer <= 0.0 || b_spd < 115.0 * s);

            let mut catcher: Option<(usize, f32)> = None;
            for (i, pl) in self.players.iter().enumerate() {
                if self.ball.pickup_lock > 0.0 && self.ball.last_kicker == Some(i) {
                    continue;
                }
                // Thủ môn luôn có thể bay người cản phá cú sút trong vòng cấm!
                let reach = if pl.role == Role::GK { 17.5 * s } else { 13.5 * s };
                if pl.role != Role::GK && !can_field_pickup {
                    continue;
                }
                let d = (pl.x - self.ball.x).hypot(pl.y - self.ball.y);
                if d <= reach && catcher.map_or(true, |(_, bd)| d < bd) {
                    catcher = Some((i, d));
                }
            }

            if let Some((idx, _)) = catcher {
                self.ball.owner = Some(idx);
                self.ball.vx = 0.0;
                self.ball.vy = 0.0;
                self.ball.vz = 0.0;
                self.ball.z = 0.0;
                self.ball.pass_target_player = None;
                self.ball.is_goal_shot = false;
                self.ball.hold_timer = if self.players[idx].role == Role::GK {
                    0.55
                } else {
                    (0.65 + self.rng.next() * 0.75) / self.p.pass_freq.max(0.4)
                };
                self.chase_timer = 0.0;
                for pl in &mut self.players {
                    pl.chase_rank = None;
                }
            }
        }
    }

    fn draw(&mut self, gl: &glow::Context, sh: &mut Shared, now: f64) {
        let t = ((now - self.t0) / 1000.0) as f32;
        let s = self.ui_scale();
        let pitch = self.pitch_rect();

        let n = self.players.len().min(22);
        let mut pl_pos = [0f32; 88];
        let mut pl_meta = [0f32; 88];
        for (i, pl) in self.players.iter().take(22).enumerate() {
            let kit = match (pl.team, pl.role) {
                (0, Role::GK) => 2.0,
                (1, Role::GK) => 3.0,
                (0, _) => 0.0,
                _ => 1.0,
            };
            let spd_norm = (pl.vx.hypot(pl.vy) / (125.0 * s)).clamp(0.0, 1.0);
            let ring_glow = if self.ball.owner == Some(i) {
                0.85
            } else if pl.chase_rank == Some(0) {
                1.0
            } else if pl.chase_rank.is_some() {
                0.55
            } else {
                0.0
            };
            pl_pos[i * 4..i * 4 + 4].copy_from_slice(&[pl.x, pl.y, pl.dir, pl.stride]);
            pl_meta[i * 4..i * 4 + 4].copy_from_slice(&[kit, spd_norm, ring_glow, pl.hair]);
        }

        let b_spd = if self.ball.owner.is_some() {
            0.0
        } else {
            (self.ball.vx.hypot(self.ball.vy) / (420.0 * s)).clamp(0.0, 1.0)
        };
        let ball_u = [self.ball.x, self.ball.y, self.ball.z, self.ball.spin];
        let trail_u = [self.ball.trail_origin.0, self.ball.trail_origin.1, b_spd, self.ball.kick_age];
        let target_u = self.target_ping.unwrap_or([0.0, 0.0, 0.0, 0.0]);
        let goal_u = self.goal_fx.unwrap_or([0.0, 0.0, 0.0, 0.0]);

        let p = &mut sh.prog;
        unsafe { gl.use_program(Some(p.prog)) };
        self.b.bind_common(gl, p, sh.noise_tex, sh.seed, t);
        p.f4v(gl, "uPitch", &pitch);
        p.f1(gl, "uStripes", self.p.stripes);
        p.f1(gl, "uLighting", self.p.lighting.round());
        p.f1(gl, "uPlayerN", n as f32);
        p.f4v(gl, "uPlPos", &pl_pos);
        p.f4v(gl, "uPlMeta", &pl_meta);
        p.f4v(gl, "uBall", &ball_u);
        p.f4v(gl, "uBallTrail", &trail_u);
        p.f4v(gl, "uTarget", &target_u);
        p.f4v(gl, "uGoalFx", &goal_u);
        unsafe { gl.draw_arrays(glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for Stadium {
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

    fn handle_click(&mut self, _gl: &glow::Context, x: f32, y: f32, _now: f64) {
        let s = self.ui_scale();
        let [x0, y0, x1, y1] = self.pitch_rect();
        let cy = 0.5 * (y0 + y1);
        let gh = (y1 - y0) * 0.22;
        let gd = 28.0 * s;

        // Nếu đang ăn mừng bàn thắng mà người dùng click tiếp -> vào giao bóng mới luôn
        if self.goal_fx.is_some() {
            self.goal_fx = None;
        }

        // Kiểm tra người dùng click thẳng vào cầu môn Trái hoặc Phải -> Sút tung lưới!
        let click_left_goal = x < x0 + 10.0 * s && x >= x0 - gd - 26.0 * s && (y - cy).abs() < gh * 0.65;
        let click_right_goal = x > x1 - 10.0 * s && x <= x1 + gd + 26.0 * s && (y - cy).abs() < gh * 0.65;

        let (tx, ty, is_shot) = if click_left_goal {
            (x0 - gd * 0.55, y.clamp(cy - gh * 0.40, cy + gh * 0.40), true)
        } else if click_right_goal {
            (x1 + gd * 0.55, y.clamp(cy - gh * 0.40, cy + gh * 0.40), true)
        } else {
            (
                x.clamp(x0 - 18.0 * s, x1 + 18.0 * s),
                y.clamp(y0 - 14.0 * s, y1 + 14.0 * s),
                false,
            )
        };

        let prev_owner = self.ball.owner.take();
        if let Some(k) = prev_owner {
            self.players[k].dir = (ty - self.players[k].y).atan2(tx - self.players[k].x);
        }
        self.ball.last_kicker = prev_owner;
        self.ball.pass_target_player = None;
        self.ball.is_goal_shot = is_shot;
        self.ball.pickup_lock = if is_shot { 0.35 } else { 0.28 };

        // Phóng bóng mượt về đúng tọa độ người dùng vừa click
        let drag = if is_shot { 1.45 } else { 2.35 };
        let max_v = if is_shot { 860.0 * s } else { 780.0 * s };
        let loft = if is_shot { 0.18 } else { 0.20 };
        self.kick_ball_to(tx, ty, drag, max_v, loft);
        self.target_ping = Some([tx.clamp(x0 - gd, x1 + gd), ty, 0.0, 1.0]);

        // Gọi các cầu thủ gần nhất của cả 2 đội lập tức bứt tốc đuổi theo bóng!
        for pl in &mut self.players {
            pl.chase_rank = None;
        }
        let mut cands: Vec<(usize, f32)> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.role != Role::GK)
            .map(|(i, p)| (i, (p.x - tx).hypot(p.y - ty)))
            .collect();
        cands.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        for (rank, &(idx, _)) in cands.iter().take(4).enumerate() {
            self.players[idx].chase_rank = Some(rank as u8);
        }
        self.chase_timer = 3.8;
    }

    fn is_interacting(&self, _now: f64) -> bool {
        self.chase_timer > 0.01
            || self.target_ping.is_some()
            || self.goal_fx.is_some()
            || (self.ball.owner.is_none() && self.ball.vx.hypot(self.ball.vy) > 180.0)
    }

    fn destroy(&mut self, gl: &glow::Context) {
        self.b.destroy(gl);
    }

    fn field_ms(&self) -> f64 {
        0.0
    }
}
