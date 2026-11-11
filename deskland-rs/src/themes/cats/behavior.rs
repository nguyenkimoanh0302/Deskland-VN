//! Hành vi của mèo (port từ prototype/cats-behavior.js, chỉ logic, không vẽ): đi tới một điểm ngẫu nhiên, ngồi
//! cạnh biểu tượng hoặc bạn mèo, liếm lông, ngủ. Mèo luôn tránh đứng dưới biểu tượng vì biểu tượng nằm đè lên hình nền.

use super::draw::{Coat, COATS};
use super::interact::{Fun, YARN_SECONDS};
use super::nature::Obstacle;
use crate::engine::Rng;

const WALK_SPEED: f32 = 42.0; // px/giây ở tốc độ ×1
const SIT_SECONDS: [f32; 2] = [3.0, 9.0];
const SLEEP_SECONDS: [f32; 2] = [10.0, 26.0];
const SNUGGLE_CHANCE: f32 = 0.45; // tỉ lệ chọn chỗ ngồi sát cạnh một biểu tượng
const FRIEND_DISTANCE: f32 = 120.0; // dừng gần bạn trong khoảng này thì quay sang và thả tim
const HEART_SECONDS: f32 = 1.8;
const VISIT_CHANCE: f32 = 0.25; // tỉ lệ mèo chủ động đi tới ngồi cạnh một bạn mèo khác
const BODY_HALF: f32 = 44.0; // nửa chiều dài thân mèo, dùng để né biểu tượng
const BODY_HEIGHT: f32 = 82.0;
const SIZE_RANGE: [f32; 2] = [72.0, 86.0];
const GROUND_MARGIN: f32 = 26.0; // khoảng né gốc cây (mèo được phép đi phía trước/sau tán lá)
const GROOM_CHANCE: f32 = 0.45; // khi không ngủ: tỉ lệ ngồi liếm lông thay vì ngồi ngắm
const CUSHION_CHANCE: f32 = 0.35;
const HAPPY_CHANCE: f32 = 0.65; // tỉ lệ mèo có mắt cười híp "^ ^"
const COLLAR_CHANCE: f32 = 0.3;
pub const MARGIN_X: f32 = 70.0;
pub const MARGIN_BOTTOM: f32 = 60.0; // chừa mép dưới cho lùm cây và mây khung viền

#[derive(Clone, Copy, PartialEq)]
pub enum CatState {
    Walk,
    Sit,
    Groom,
    Sleep,
    /// Đang vờn cuộn len (tương tác chuột)
    Play,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SleepPose {
    Loaf,
    Side,
    Back,
}

pub struct Cat {
    pub x: f32,
    pub y: f32,
    pub dir: f32,
    coat: usize,
    pub size: f32,
    pace: f32,
    pub state: CatState,
    pub(super) timer: f32,
    pub(super) target: Option<(f32, f32)>,
    pub phase: f32,
    pub clock: f32,
    pub sway: f32,
    pub blink: f32,
    blink_in: f32,
    pub ear_twitch: f32,
    pub heart: f32,
    pub sleep_pose: SleepPose,
    pub cushion: bool,
    pub happy: bool,
    pub collar: bool,
    /// Hiệu ứng tương tác chuột (nhảy cẫng, chùm tim, vờn len)
    pub fun: Fun,
}

impl Cat {
    pub fn coat(&self) -> &'static Coat {
        &COATS[self.coat % COATS.len()]
    }
}

/// Khung cảnh để mèo chọn đường; `others` là vị trí mọi chú mèo (kể cả chính nó ở chỉ số `me`).
pub struct World<'a> {
    pub w: f32,
    pub top: f32,
    pub floor: f32,
    pub obstacles: &'a [Obstacle],
    pub others: &'a [(f32, f32)],
    pub me: usize,
}

#[derive(Clone, Copy)]
pub struct Tune {
    pub speed: f32,
    pub naps: f32,
}

/// Biểu tượng chặn cả vùng thân mèo; vật cản "ground" (gốc cây, đá) chỉ chặn chỗ đặt chân.
pub(super) fn blocked_at(x: f32, y: f32, obstacles: &[Obstacle]) -> bool {
    obstacles.iter().any(|ob| {
        if ob.ground {
            x > ob.x - GROUND_MARGIN && x < ob.x + ob.w + GROUND_MARGIN && y > ob.y - 8.0 && y < ob.y + ob.h + 8.0
        } else {
            x > ob.x - BODY_HALF && x < ob.x + ob.w + BODY_HALF && y > ob.y - 4.0 && y < ob.y + ob.h + BODY_HEIGHT
        }
    })
}

fn path_clear(from: (f32, f32), to: (f32, f32), obstacles: &[Obstacle]) -> bool {
    let steps = ((to.0 - from.0).hypot(to.1 - from.1) / 12.0).ceil() as usize;
    (1..=steps).all(|i| {
        let t = i as f32 / steps as f32;
        !blocked_at(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t, obstacles)
    })
}

fn pick<T: Copy>(items: &[T], rng: &mut Rng) -> T {
    items[((rng.next() * items.len() as f32) as usize).min(items.len() - 1)]
}

fn random_spot(world: &World, rng: &mut Rng) -> (f32, f32) {
    if world.others.len() > 1 && rng.next() < VISIT_CHANCE {
        let friend = pick(world.others, rng);
        let side = if rng.next() < 0.5 { -1.0 } else { 1.0 };
        let x = friend.0 + side * (95.0 + rng.next() * 20.0);
        return (x, friend.1 + (rng.next() - 0.5) * 20.0);
    }
    if !world.obstacles.is_empty() && rng.next() < SNUGGLE_CHANCE {
        let ic = pick(world.obstacles, rng);
        let left = rng.next() < 0.5;
        let x = if left { ic.x - BODY_HALF - 4.0 } else { ic.x + ic.w + BODY_HALF + 4.0 };
        return (x, ic.y + ic.h * (0.55 + rng.next() * 0.4));
    }
    let x = MARGIN_X + rng.next() * (world.w - MARGIN_X * 2.0);
    (x, world.top + rng.next() * (world.floor - world.top))
}

fn in_bounds(p: (f32, f32), world: &World) -> bool {
    p.0 >= MARGIN_X && p.0 <= world.w - MARGIN_X && p.1 >= world.top && p.1 <= world.floor
}

fn pick_target(cat: &Cat, world: &World, rng: &mut Rng) -> Option<(f32, f32)> {
    for _ in 0..40 {
        let spot = random_spot(world, rng);
        if !in_bounds(spot, world) || blocked_at(spot.0, spot.1, world.obstacles) {
            continue;
        }
        if (spot.0 - cat.x).hypot(spot.1 - cat.y) < 60.0 {
            continue;
        }
        if path_clear((cat.x, cat.y), spot, world.obstacles) {
            return Some(spot);
        }
    }
    None
}

fn free_spot(world: &World, rng: &mut Rng) -> Option<(f32, f32)> {
    (0..60).map(|_| random_spot(world, rng)).find(|&p| in_bounds(p, world) && !blocked_at(p.0, p.1, world.obstacles))
}

pub fn create_cat(world: &World, coat: usize, rng: &mut Rng) -> Cat {
    let (x, y) = free_spot(world, rng).unwrap_or((world.w / 2.0, world.floor));
    Cat {
        x,
        y,
        dir: if rng.next() < 0.5 { -1.0 } else { 1.0 },
        coat,
        size: SIZE_RANGE[0] + rng.next() * (SIZE_RANGE[1] - SIZE_RANGE[0]),
        pace: 0.85 + rng.next() * 0.3,
        state: CatState::Sit,
        timer: 1.0 + rng.next() * 4.0,
        target: None,
        phase: 0.0,
        clock: rng.next() * 10.0,
        sway: 0.0,
        blink: 0.0,
        blink_in: 1.0 + rng.next() * 4.0,
        ear_twitch: 0.0,
        heart: 0.0,
        sleep_pose: SleepPose::Loaf,
        cushion: false,
        happy: rng.next() < HAPPY_CHANCE,
        collar: rng.next() < COLLAR_CHANCE,
        fun: Fun::default(),
    }
}

fn rest(cat: &mut Cat, world: &World, tune: Tune, rng: &mut Rng) {
    cat.fun.rush = 1.0;
    if cat.fun.play {
        (cat.state, cat.timer, cat.target, cat.fun.bat_in) = (CatState::Play, YARN_SECONDS, None, 0.3);
        return;
    }
    let sleeping = rng.next() < tune.naps;
    let range = if sleeping { SLEEP_SECONDS } else { SIT_SECONDS };
    cat.state = if sleeping {
        CatState::Sleep
    } else if rng.next() < GROOM_CHANCE {
        CatState::Groom
    } else {
        CatState::Sit
    };
    if sleeping {
        cat.sleep_pose = pick(&[SleepPose::Loaf, SleepPose::Side, SleepPose::Back], rng);
        cat.cushion = rng.next() < CUSHION_CHANCE;
    }
    cat.timer = range[0] + rng.next() * (range[1] - range[0]);
    cat.target = None;
    let friend = world.others.iter().enumerate().find(|&(i, o)| i != world.me && (o.0 - cat.x).hypot(o.1 - cat.y) < FRIEND_DISTANCE);
    if let Some((_, f)) = friend {
        cat.heart = HEART_SECONDS;
        cat.dir = if f.0 >= cat.x { 1.0 } else { -1.0 };
    }
}

pub(super) fn start_walk(cat: &mut Cat, world: &World, tune: Tune, rng: &mut Rng) {
    let Some(target) = pick_target(cat, world, rng) else {
        rest(cat, world, Tune { naps: 0.0, ..tune }, rng);
        cat.timer = 2.0;
        return;
    };
    cat.state = CatState::Walk;
    cat.target = Some(target);
    if tune.speed <= 0.0 {
        rest(cat, world, tune, rng);
    }
}

fn step_walk(cat: &mut Cat, dt: f32, world: &World, tune: Tune, rng: &mut Rng) {
    let Some(target) = cat.target else {
        return rest(cat, world, tune, rng);
    };
    let (dx, dy) = (target.0 - cat.x, target.1 - cat.y);
    let dist = dx.hypot(dy);
    let step = WALK_SPEED * tune.speed.max(0.3) * cat.pace * cat.fun.rush * dt;
    if dx.abs() > 2.0 {
        cat.dir = if dx > 0.0 { 1.0 } else { -1.0 };
    }
    if dist <= step {
        (cat.x, cat.y) = target;
        return rest(cat, world, tune, rng);
    }
    cat.x += dx / dist * step;
    cat.y += dy / dist * step;
    cat.phase += step * 0.16;
}

fn step_face(cat: &mut Cat, dt: f32, rng: &mut Rng) {
    cat.clock += dt;
    cat.sway = (cat.clock * if cat.state == CatState::Walk { 3.2 } else { 1.4 }).sin();
    cat.blink = (cat.blink - dt).max(0.0);
    cat.heart = (cat.heart - dt).max(0.0);
    cat.fun.tick(dt);
    cat.blink_in -= dt;
    if cat.blink_in <= 0.0 {
        cat.blink = 0.14;
        cat.blink_in = 2.0 + rng.next() * 5.0;
    }
    cat.ear_twitch = if cat.state == CatState::Sit && cat.clock % 7.0 < 0.25 { 1.0 } else { 0.0 };
}

/// Sau khi biểu tượng di chuyển, mèo đang đứng/đi vào chỗ bị che sẽ chọn đường mới.
pub fn adapt_to_icons(cat: &mut Cat, world: &World, tune: Tune, rng: &mut Rng) {
    let blocked = blocked_at(cat.x, cat.y, world.obstacles);
    let path_broken =
        cat.state == CatState::Walk && cat.target.is_none_or(|t| !in_bounds(t, world) || !path_clear((cat.x, cat.y), t, world.obstacles));
    let outside = !in_bounds((cat.x, cat.y), world);
    if !blocked && !path_broken && !outside {
        return;
    }
    // Biểu tượng vừa được thả đè lên mèo: đặt mèo sang một chỗ trống khác thay vì để bị che.
    if blocked || outside {
        if let Some((x, y)) = free_spot(world, rng) {
            (cat.x, cat.y) = (x, y);
        }
    }
    start_walk(cat, world, tune, rng);
}

pub fn step_cat(cat: &mut Cat, dt: f32, world: &World, tune: Tune, rng: &mut Rng) {
    step_face(cat, dt, rng);
    if cat.state == CatState::Walk {
        return step_walk(cat, dt, world, tune, rng);
    }
    cat.timer -= dt;
    if cat.timer <= 0.0 {
        start_walk(cat, world, tune, rng);
    }
}
