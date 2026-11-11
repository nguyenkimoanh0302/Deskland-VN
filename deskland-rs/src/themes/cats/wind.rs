//! Gió trên đồi (port từ prototype/cats-wind.js): sức gió thay đổi theo từng cơn, vệt gió trắng lướt qua,
//! lá bay / lá rụng đậu xuống cỏ rồi mờ dần, và vài chú bướm vàng bay lượn.

use super::butterfly::Butterfly;
use super::canvas::{hex, rgba, Canvas, Rgba};
use super::meadow::horizon;
use super::nature::{Item, Kind};
use crate::engine::Rng;
use std::f32::consts::{PI, TAU};

const LEAF_COLORS: [Rgba; 5] = [hex(0xf2c23a), hex(0xf5a524), hex(0xe8862a), hex(0x9fd24a), hex(0xd9e25a)];
const LEAF_MAX: usize = 70;
const LEAF_SPAWN_PER_SECOND: f32 = 2.0; // ở sức gió ×1
const LEAF_FADE_SECONDS: f32 = 5.0;
const STREAK_PER_SECOND: f32 = 0.7;
const STREAK_LIFE: f32 = 1.8;
const BUTTERFLY_COUNT: usize = 3;
const BURST_LEAVES: usize = 10; // lá rơi khi bấm rung cây
const SCATTER_DISTANCE: f32 = 220.0; // bướm trong khoảng này giật mình bay đi khi có cuộn len rơi xuống

/// Khung cảnh mà gió cần biết mỗi khung hình.
pub struct Scene<'a> {
    pub w: f32,
    pub h: f32,
    pub taskbar: f32,
    pub strength: f32,
    pub items: &'a [Item],
}

struct Leaf {
    x: f32,
    y: f32,
    vx: f32,
    ground_y: f32,
    rot: f32,
    spin: f32,
    flutter: f32,
    size: f32,
    color: Rgba,
    landed: bool,
    age: f32,
}
struct Streak {
    x: f32,
    y: f32,
    len: f32,
    curl: f32,
    age: f32,
}
pub struct Wind {
    rng: Rng,
    pub time: f32,
    gust: f32,
    gust_target: f32,
    gust_in: f32,
    leaf_debt: f32,
    streak_debt: f32,
    leaves: Vec<Leaf>,
    streaks: Vec<Streak>,
    butterflies: Vec<Butterfly>,
}

impl Wind {
    pub fn new(seed: u64) -> Self {
        Wind {
            rng: Rng::new(seed),
            time: 0.0,
            gust: 0.0,
            gust_target: 0.4,
            gust_in: 0.0,
            leaf_debt: 0.0,
            streak_debt: 0.0,
            leaves: Vec::new(),
            streaks: Vec::new(),
            butterflies: Vec::new(),
        }
    }

    pub fn wind_of(&self, strength: f32) -> f32 {
        strength * (0.5 + self.gust + 0.15 * (self.time * 0.7).sin())
    }

    pub fn step(&mut self, dt: f32, o: &Scene) {
        self.time += dt;
        self.gust_in -= dt;
        if self.gust_in <= 0.0 {
            self.gust_target = self.rng.next() * 1.2;
            self.gust_in = 3.0 + self.rng.next() * 5.0;
        }
        self.gust += (self.gust_target - self.gust) * (dt * 0.8).min(1.0);
        let w = self.wind_of(o.strength);
        self.leaf_debt += dt * LEAF_SPAWN_PER_SECOND * o.strength * (0.4 + self.gust);
        while self.leaf_debt >= 1.0 && self.leaves.len() < LEAF_MAX {
            let leaf = self.spawn_leaf(o);
            self.leaves.push(leaf);
            self.leaf_debt -= 1.0;
        }
        self.leaf_debt = self.leaf_debt.min(1.0);
        self.streak_debt += dt * STREAK_PER_SECOND * o.strength * (0.3 + self.gust);
        while self.streak_debt >= 1.0 {
            let streak = self.spawn_streak(o);
            self.streaks.push(streak);
            self.streak_debt -= 1.0;
        }
        self.leaves.retain_mut(|l| step_leaf(l, dt, w, o));
        self.streaks.retain_mut(|s| {
            s.age += dt;
            s.age < STREAK_LIFE
        });
        while self.butterflies.len() < BUTTERFLY_COUNT {
            let (x, y) = meadow_spot(&mut self.rng, o);
            let target = meadow_spot(&mut self.rng, o);
            let (flap, retarget) = (self.rng.next() * 6.0, 3.0 + self.rng.next() * 4.0);
            self.butterflies.push(Butterfly::new(x, y - 30.0, target, flap, retarget));
        }
        let rng = &mut self.rng;
        for b in &mut self.butterflies {
            b.step(dt, w, || (meadow_spot(rng, o), 3.0 + rng.next() * 4.0));
        }
    }

    /// Lá bung ra từ tán cây bị rung, bay toả hai bên rồi rơi xuống quanh gốc.
    pub fn burst(&mut self, c: &Item) {
        for _ in 0..BURST_LEAVES {
            let r = &mut self.rng;
            let (x, y) = (c.x + (r.next() - 0.5) * c.w * 0.8, c.y - c.h * (0.4 + r.next() * 0.45));
            let (vx, ground_y) = ((r.next() - 0.5) * 70.0, c.y + 6.0 + r.next() * 110.0);
            let (rot, spin, flutter, size) = (r.next() * TAU, (r.next() - 0.5) * 6.0, r.next() * TAU, 9.0 + r.next() * 5.0);
            let color = LEAF_COLORS[((r.next() * 5.0) as usize).min(4)];
            self.leaves.push(Leaf { x, y, vx, ground_y, rot, spin, flutter, size, color, landed: false, age: 0.0 });
        }
    }

    /// Bướm gần chỗ cuộn len rơi giật mình bay ra xa.
    pub fn scatter(&mut self, x: f32, y: f32) {
        for b in &mut self.butterflies {
            let d = (b.x - x).hypot(b.y - y);
            if d < SCATTER_DISTANCE {
                let d = d.max(1.0);
                (b.target, b.retarget) = ((b.x + (b.x - x) / d * 220.0, b.y - 40.0), 2.5);
            }
        }
    }

    /// Bấm lên trời: cả đàn bướm bay tới chỗ bấm.
    pub fn lure(&mut self, x: f32, y: f32) {
        for b in &mut self.butterflies {
            (b.target, b.retarget) = ((x + (self.rng.next() - 0.5) * 40.0, y + 30.0), 4.0);
        }
    }

    pub fn draw_landed(&self, ctx: &mut Canvas) {
        for l in self.leaves.iter().filter(|l| l.landed) {
            draw_leaf(ctx, l);
        }
    }

    pub fn draw_air(&self, ctx: &mut Canvas, strength: f32) {
        let w = self.wind_of(strength);
        for s in &self.streaks {
            draw_streak(ctx, s, w);
        }
        for l in self.leaves.iter().filter(|l| !l.landed) {
            draw_leaf(ctx, l);
        }
        for b in &self.butterflies {
            b.draw(ctx);
        }
    }

    fn spawn_leaf(&mut self, o: &Scene) -> Leaf {
        let canopies: Vec<&Item> = o.items.iter().filter(|i| matches!(i.kind, Kind::Tree | Kind::Bush)).collect();
        let from_tree = !canopies.is_empty() && self.rng.next() < 0.65;
        let c = from_tree.then(|| canopies[((self.rng.next() * canopies.len() as f32) as usize).min(canopies.len() - 1)]);
        let (x, y) = match c {
            Some(c) => (c.x + (self.rng.next() - 0.5) * c.w * 0.8, c.y - c.h * (0.45 + self.rng.next() * 0.4)),
            None => (-20.0, self.rng.next() * (o.h - o.taskbar) * 0.85),
        };
        let r = &mut self.rng;
        Leaf {
            x,
            y,
            vx: 10.0,
            ground_y: c.map_or(y, |c| c.y) + 30.0 + r.next() * 180.0,
            rot: r.next() * TAU,
            spin: (r.next() - 0.5) * 4.0,
            flutter: r.next() * TAU,
            size: 9.0 + r.next() * 5.0,
            color: LEAF_COLORS[((r.next() * 5.0) as usize).min(4)],
            landed: false,
            age: 0.0,
        }
    }

    fn spawn_streak(&mut self, o: &Scene) -> Streak {
        let (x, y) = meadow_spot(&mut self.rng, o);
        let len = 70.0 + self.rng.next() * 70.0;
        let curl = if self.rng.next() < 0.5 { -1.0 } else { 1.0 };
        Streak { x: x - o.w * 0.3, y: y - 40.0, len, curl, age: 0.0 }
    }
}

fn meadow_spot(rng: &mut Rng, o: &Scene) -> (f32, f32) {
    let top = horizon(o.h) + 60.0;
    (60.0 + rng.next() * (o.w - 120.0), top + rng.next() * (o.h - o.taskbar - top - 60.0))
}

/// Trả về false khi lá cần bị loại (bay khỏi màn hình hoặc đã mờ hẳn trên cỏ).
fn step_leaf(l: &mut Leaf, dt: f32, wind: f32, o: &Scene) -> bool {
    if l.landed {
        l.age += dt;
        return l.age < LEAF_FADE_SECONDS;
    }
    l.flutter += dt * 3.2;
    l.vx += (wind * 60.0 + 8.0 - l.vx) * (dt * 1.5).min(1.0);
    l.x += l.vx * dt;
    l.y += (15.0 + l.flutter.sin() * 22.0) * dt;
    l.rot += l.spin * dt * (1.0 + wind);
    if l.y >= l.ground_y {
        l.landed = true;
        l.flutter = 0.0;
    }
    l.x < o.w + 40.0 && l.y < o.h + 40.0
}

fn draw_leaf(ctx: &mut Canvas, l: &Leaf) {
    let s = l.size;
    ctx.save();
    ctx.set_alpha(if l.landed { (1.0 - l.age / LEAF_FADE_SECONDS).max(0.0) * 0.9 } else { 1.0 });
    ctx.translate(l.x, l.y);
    ctx.rotate(l.rot);
    ctx.scale(1.0, if l.landed { 0.75 } else { 0.3 + 0.7 * l.flutter.cos().abs() });
    ctx.begin();
    ctx.move_to(-s, 0.0);
    ctx.quad_to(0.0, -s * 0.75, s, 0.0);
    ctx.quad_to(0.0, s * 0.75, -s, 0.0);
    ctx.fill(l.color);
    ctx.begin();
    ctx.move_to(-s * 1.15, 0.0);
    ctx.line_to(s * 0.9, 0.0);
    ctx.stroke(rgba(120, 70, 20, 0.45), 0.8);
    ctx.restore();
}

/// Vệt gió trắng hình móc câu lướt sang phải rồi tan dần.
fn draw_streak(ctx: &mut Canvas, s: &Streak, wind: f32) {
    let t = s.age / STREAK_LIFE;
    let (x, y) = (s.x + s.age * (120.0 + wind * 90.0), s.y);
    ctx.save();
    ctx.set_alpha((t * PI).sin() * 0.75);
    ctx.begin();
    ctx.move_to(x, y);
    ctx.quad_to(x + s.len * 0.6, y - 6.0 * s.curl, x + s.len, y + 2.0 * s.curl);
    ctx.quad_to(x + s.len + 12.0, y + 8.0 * s.curl, x + s.len + 4.0, y + 13.0 * s.curl);
    ctx.stroke(hex(0xffffff), 2.6);
    ctx.restore();
}
