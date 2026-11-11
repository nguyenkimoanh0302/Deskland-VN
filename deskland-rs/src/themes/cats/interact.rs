//! Tương tác chuột (port từ prototype/cats-interact.js + phần tương tác của cats-behavior.js): bấm vào mèo để
//! vuốt ve (mèo nhảy cẫng, thả chùm tim), bấm vào cây để rung lá, bấm xuống cỏ để thả cuộn len cho vài chú mèo
//! chạy tới vờn, bấm lên trời để gọi bướm. Mỗi lần bấm có một vòng sáng nhỏ lan ra.

use super::behavior::{blocked_at, start_walk, Cat, CatState, Tune, World, MARGIN_X};
use super::canvas::{hex, rgba, Canvas, Rgba};
use super::draw::OUTLINE;
use super::meadow::horizon;
use super::nature::Item;
use super::wind::Wind;
use crate::engine::Rng;
use std::f32::consts::PI;

pub const YARN_SECONDS: f32 = 9.0;
pub const HOP_SECONDS: f32 = 0.55; // nhảy cẫng khi được vuốt ve
pub const LOVE_SECONDS: f32 = 1.6; // chùm tim bay lên khi được vuốt ve
const JOY_SECONDS: f32 = 2.6; // mắt cười híp sau khi được vuốt ve
const RUSH: f32 = 2.2; // tốc độ chạy tới cuộn len so với đi dạo
const PET_RADIUS: f32 = 46.0;
/// Chỗ đứng quanh cuộn len (so với tâm len) cho tối đa 3 chú cùng vờn: trái, phải, phía sau.
const YARN_SLOTS: [(f32, f32); 3] = [(-40.0, -3.0), (40.0, -3.0), (-16.0, -20.0)];
const YARN_RADIUS: f32 = 11.0;
const YARN_COLORS: [Rgba; 2] = [hex(0xe2557a), hex(0xf6a3b9)];
const YARN_CHASE_DISTANCE: f32 = 560.0; // chỉ mèo trong khoảng này mới chạy tới vờn len
const YARN_BAT_SECONDS: f32 = 1.3;
const YARN_GRAVITY: f32 = 900.0;
const RING_SECONDS: f32 = 0.55;
const RING_MAX: usize = 6;
const SKY_CLICK_BELOW_HORIZON: f32 = 60.0;

/// Trạng thái tương tác của một chú mèo.
pub struct Fun {
    pub hop: f32,
    pub love: f32,
    pub joy: f32,
    pub play: bool,
    pub slot: usize,
    pub rush: f32,
    pub bat_in: f32,
}

impl Default for Fun {
    fn default() -> Self {
        Fun { hop: 0.0, love: 0.0, joy: 0.0, play: false, slot: 0, rush: 1.0, bat_in: 0.0 }
    }
}

impl Fun {
    pub fn tick(&mut self, dt: f32) {
        self.hop = (self.hop - dt).max(0.0);
        self.love = (self.love - dt).max(0.0);
        self.joy = (self.joy - dt).max(0.0);
    }
    fn busy(&self) -> bool {
        self.hop > 0.0 || self.love > 0.0
    }
}

struct Yarn {
    x: f32,
    y: f32,
    vx: f32,
    z: f32,
    vz: f32,
    age: f32,
    spin: f32,
}

/// Những gì một cú bấm có thể chạm tới.
pub struct Scene<'a> {
    pub h: f32,
    pub cats: &'a mut [Cat],
    pub items: &'a mut [Item],
    pub wind: &'a mut Wind,
    pub world: World<'a>,
    pub tune: Tune,
}

#[derive(Default)]
pub struct Interact {
    yarn: Option<Yarn>,
    rings: Vec<(f32, f32, f32)>,
}

impl Interact {
    pub fn yarn_y(&self) -> Option<f32> {
        self.yarn.as_ref().map(|y| y.y)
    }

    pub fn active(&self, cats: &[Cat], items: &[Item]) -> bool {
        self.yarn.is_some() || !self.rings.is_empty() || cats.iter().any(|c| c.fun.busy()) || items.iter().any(|i| i.shake > 0.0)
    }

    pub fn click(&mut self, x: f32, y: f32, s: Scene, rng: &mut Rng) {
        if self.rings.len() >= RING_MAX {
            self.rings.remove(0);
        }
        self.rings.push((x, y, 0.0));
        if let Some(cat) = cat_at(s.cats, x, y) {
            return pet(cat, x, rng);
        }
        if let Some(item) = super::nature::item_at(s.items, x, y) {
            item.shake = 1.0;
            return s.wind.burst(item);
        }
        if y < horizon(s.h) + SKY_CLICK_BELOW_HORIZON {
            return s.wind.lure(x, y);
        }
        if self.yarn.is_some() {
            release(s.cats, &s.world, s.tune, rng);
        }
        let w = &s.world;
        let yarn = Yarn { x: x.clamp(MARGIN_X, w.w - MARGIN_X), y: y.clamp(w.top, w.floor), vx: 0.0, z: 46.0, vz: 0.0, age: 0.0, spin: 0.0 };
        s.wind.scatter(x, y);
        let dist = |c: &Cat| (c.x - yarn.x).hypot(c.y - yarn.y);
        let mut near: Vec<usize> =
            (0..s.cats.len()).filter(|&i| s.cats[i].state != CatState::Sleep && dist(&s.cats[i]) < YARN_CHASE_DISTANCE).collect();
        near.sort_by(|&a, &b| dist(&s.cats[a]).total_cmp(&dist(&s.cats[b])));
        for (slot, &i) in near.iter().take(YARN_SLOTS.len()).enumerate() {
            chase(&mut s.cats[i], &yarn, w, slot);
        }
        self.yarn = Some(yarn);
    }

    pub fn step(&mut self, dt: f32, cats: &mut [Cat], world: &World, tune: Tune, rng: &mut Rng) {
        for r in &mut self.rings {
            r.2 += dt;
        }
        self.rings.retain(|r| r.2 < RING_SECONDS);
        let Some(b) = self.yarn.as_mut() else { return };
        b.age += dt;
        b.vz -= YARN_GRAVITY * dt;
        b.z += b.vz * dt;
        if b.z <= 0.0 {
            b.z = 0.0;
            b.vz = if b.vz < -60.0 { -b.vz * 0.45 } else { 0.0 };
        }
        b.x = (b.x + b.vx * dt).clamp(MARGIN_X, world.w - MARGIN_X);
        b.vx *= (-2.2 * dt).exp();
        b.spin += b.vx * dt / YARN_RADIUS;
        for cat in cats.iter_mut().filter(|c| c.fun.play && c.state == CatState::Play) {
            step_player(cat, b, dt, world, rng);
        }
        if b.age >= YARN_SECONDS {
            self.yarn = None;
            release(cats, world, tune, rng);
        }
    }

    pub fn draw_yarn(&self, ctx: &mut Canvas) {
        if let Some(b) = &self.yarn {
            draw_yarn_ball(ctx, b);
        }
    }

    pub fn draw_rings(&self, ctx: &mut Canvas) {
        for &(x, y, age) in &self.rings {
            let t = age / RING_SECONDS;
            ctx.save();
            ctx.set_alpha((1.0 - t) * 0.8);
            ctx.begin();
            ctx.circle(x, y, 6.0 + t * 30.0);
            ctx.stroke(hex(0xffffff), 2.6);
            ctx.begin();
            ctx.circle(x, y, 3.0 + t * 16.0);
            ctx.stroke(hex(0xffffff), 1.6);
            ctx.restore();
        }
    }
}

/// Mèo dưới con trỏ: ưu tiên chú gần nhất trong vùng thân (từ chân lên tới tai).
fn cat_at(cats: &mut [Cat], x: f32, y: f32) -> Option<&mut Cat> {
    let d = |c: &Cat| (c.x - x).hypot(c.y - 32.0 * c.size / 60.0 - y);
    cats.iter_mut().filter(|c| d(c) < PET_RADIUS * c.size / 60.0).min_by(|a, b| d(a).total_cmp(&d(b)))
}

/// Vuốt ve: mèo nhảy cẫng, thả chùm tim, cười híp mắt và ngồi lại một lúc (mèo đang ngủ thì tỉnh dậy).
fn pet(cat: &mut Cat, x: f32, rng: &mut Rng) {
    (cat.fun.hop, cat.fun.love, cat.fun.joy, cat.blink) = (HOP_SECONDS, LOVE_SECONDS, JOY_SECONDS, 0.0);
    cat.dir = if x >= cat.x { 1.0 } else { -1.0 };
    if matches!(cat.state, CatState::Sleep | CatState::Walk) {
        (cat.state, cat.target, cat.fun.play, cat.fun.rush) = (CatState::Sit, None, false, 1.0);
    }
    if cat.state != CatState::Play {
        cat.timer = 2.5 + rng.next() * 2.0;
    }
}

/// Chạy tới chỗ đứng `slot` cạnh cuộn len rồi chuyển sang vờn khi tới nơi.
fn chase(cat: &mut Cat, b: &Yarn, world: &World, slot: usize) {
    let (dx, dy) = YARN_SLOTS[slot % YARN_SLOTS.len()];
    let spot = (b.x + dx, b.y + dy);
    if blocked_at(spot.0, spot.1, world.obstacles) {
        return;
    }
    (cat.fun.play, cat.fun.slot, cat.fun.rush) = (true, slot, RUSH);
    (cat.state, cat.target) = (CatState::Walk, Some(spot));
}

/// Mèo đang vờn: quay về phía cuộn len, thỉnh thoảng khều cho len lăn đi; len lăn xa thì đuổi theo.
fn step_player(cat: &mut Cat, b: &mut Yarn, dt: f32, world: &World, rng: &mut Rng) {
    cat.dir = if b.x >= cat.x { 1.0 } else { -1.0 };
    if (b.x - cat.x).abs() > 70.0 && b.vx.abs() < 15.0 {
        return chase(cat, b, world, cat.fun.slot);
    }
    cat.fun.bat_in -= dt;
    if cat.fun.bat_in > 0.0 {
        return;
    }
    b.vx += cat.dir * (60.0 + rng.next() * 50.0);
    b.vz = 140.0;
    cat.fun.bat_in = YARN_BAT_SECONDS * (0.7 + rng.next() * 0.6);
}

fn release(cats: &mut [Cat], world: &World, tune: Tune, rng: &mut Rng) {
    for cat in cats.iter_mut().filter(|c| c.fun.play) {
        (cat.fun.play, cat.fun.rush) = (false, 1.0);
        if matches!(cat.state, CatState::Play | CatState::Walk) {
            start_walk(cat, world, tune, rng);
        }
    }
}

fn draw_yarn_ball(ctx: &mut Canvas, b: &Yarn) {
    let fade = ((YARN_SECONDS - b.age) / 0.6).min(1.0);
    let cy = b.y - YARN_RADIUS - b.z;
    ctx.save();
    ctx.set_alpha(fade);
    let shadow = YARN_RADIUS * (1.0 - (b.z / 120.0).min(0.5));
    ctx.begin();
    ctx.oval(b.x, b.y - 1.0, shadow * 1.2, shadow * 0.4, 0.0);
    ctx.fill(rgba(30, 60, 15, 0.28));
    ctx.begin();
    ctx.move_to(b.x - YARN_RADIUS * 0.6, b.y - 2.0);
    ctx.bezier_to([b.x - 18.0, b.y + 2.0, b.x - 24.0, b.y - 6.0, b.x - 32.0, b.y - 1.0]);
    ctx.stroke(YARN_COLORS[0], 1.6);
    ctx.begin();
    ctx.circle(b.x, cy, YARN_RADIUS + 1.6);
    ctx.fill(OUTLINE);
    ctx.begin();
    ctx.circle(b.x, cy, YARN_RADIUS);
    ctx.fill(YARN_COLORS[0]);
    for i in 0..3 {
        ctx.begin();
        ctx.oval(b.x, cy, YARN_RADIUS * 0.8, YARN_RADIUS * 0.32, b.spin + i as f32 * 1.05);
        ctx.stroke(YARN_COLORS[1], 1.4);
    }
    ctx.restore();
}

/// Độ nhấc khỏi mặt đất khi nhảy cẫng (px, chưa nhân tỉ lệ mèo).
pub fn hop_lift(cat: &Cat) -> f32 {
    const HOP_HEIGHT: f32 = 16.0;
    if cat.fun.hop > 0.0 {
        ((1.0 - cat.fun.hop / HOP_SECONDS) * PI).sin() * HOP_HEIGHT
    } else {
        0.0
    }
}
