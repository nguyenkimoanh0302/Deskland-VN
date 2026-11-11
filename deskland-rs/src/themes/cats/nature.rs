//! Vật thể trên đồi (port từ prototype/cats-nature.js): cây lớn và lùm cây (đung đưa theo gió), đá xám, nấm đỏ.
//! Chúng được vẽ xen kẽ với mèo theo vị trí chân nên mèo có thể đi khuất sau tán cây.

use super::canvas::{hex, rgba, Canvas, Rgba};
use super::meadow::{horizon, SeededRandom};
use super::scenery::{paint_foliage_layers, FOLIAGE_TONES};
use crate::engine::IconBox;

const TREE_SHARE: f32 = 0.5; // tỉ lệ cây lớn trong tổng số "cây & lùm cây"
const ROCK_COUNT: usize = 3;
const MUSHROOM_COUNT: usize = 3;
const FOOTPRINT_H: f32 = 18.0;
const TREE_TRUNK_TOP: f32 = 0.66; // ngọn thân cây (theo chiều cao cây) nằm bên trong tán lá
const TREE_CANOPY_BASE: f32 = 0.58; // độ cao lớp lá đáy tán, phủ lên ngọn thân cây
const TRUNK_LIGHT: Rgba = hex(0x94602f);
const TRUNK_DARK: Rgba = hex(0x6d4421);
const ROCK_TONES: [Rgba; 3] = [hex(0x8f918a), hex(0xb4b6ad), hex(0xcfd1c8)];
const MUSHROOM_CAP: Rgba = hex(0xe5483d);
const SHAKE_DECAY: f32 = 1.2; // cây bị bấm rung mạnh rồi dịu dần trong ~0,8 giây

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Tree,
    Bush,
    Rock,
    Mushroom,
}

/// Vật cản trên mặt đất (gốc cây, đá) hoặc biểu tượng; `ground` = chỉ chặn chỗ đặt chân.
#[derive(Clone, Copy)]
pub struct Obstacle {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub ground: bool,
}

impl From<&IconBox> for Obstacle {
    fn from(ic: &IconBox) -> Self {
        Obstacle { x: ic.x, y: ic.y, w: ic.w, h: ic.h, ground: false }
    }
}

pub struct Item {
    pub kind: Kind,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    phase: f32,
    /// Độ rung khi bị bấm (1 → 0)
    pub shake: f32,
    /// Thùy tán lá đặt tương đối với gốc: [x, y, r, 0]
    lobes: Vec<[f32; 4]>,
    pub footprint: Option<Obstacle>,
}

pub struct Layout<'a> {
    pub w: f32,
    pub h: f32,
    pub seed: i32,
    pub icons: &'a [IconBox],
    pub taskbar: f32,
    pub count: f32,
    pub mound: f32,
}

pub fn place_nature(o: &Layout) -> Vec<Item> {
    let mut rand = SeededRandom::new(o.seed ^ 0xb054);
    let mut items: Vec<Item> = Vec::new();
    let trees = (o.count * TREE_SHARE).round() as usize;
    let bushes = (o.count.round() as usize).saturating_sub(trees);
    for (kind, n) in [(Kind::Tree, trees), (Kind::Bush, bushes), (Kind::Rock, ROCK_COUNT), (Kind::Mushroom, MUSHROOM_COUNT)] {
        for _ in 0..n {
            if let Some(item) = find_spot(kind, o, &items, &mut rand) {
                items.push(item);
            }
        }
    }
    items
}

fn nature_size(kind: Kind, rand: &mut SeededRandom) -> (f32, f32) {
    match kind {
        Kind::Tree => {
            let w = 150.0 + rand.next() * 60.0;
            (w, w * 1.3)
        }
        Kind::Bush => {
            let w = 80.0 + rand.next() * 40.0;
            (w, w * 0.62)
        }
        Kind::Rock => {
            let w = 50.0 + rand.next() * 30.0;
            (w, w * 0.55)
        }
        Kind::Mushroom => (16.0, 18.0),
    }
}

fn find_spot(kind: Kind, o: &Layout, items: &[Item], rand: &mut SeededRandom) -> Option<Item> {
    let (top, bottom, clear) = (horizon(o.h) + 80.0, o.h - o.taskbar - 90.0, o.mound + 24.0);
    for _ in 0..50 {
        let (w, h) = nature_size(kind, rand);
        let x = o.w * 0.06 + rand.next() * o.w * 0.88;
        let y = top + h * 0.6 + rand.next() * (bottom - top - h * 0.6).max(1.0);
        let hits_icon = o
            .icons
            .iter()
            .any(|ic| x + w / 2.0 > ic.x - clear && x - w / 2.0 < ic.x + ic.w + clear && y > ic.y - clear && y - h < ic.y + ic.h + clear);
        let gap = if kind == Kind::Mushroom { 40.0 } else { (w + 90.0) / 2.0 };
        let crowded = items.iter().any(|it| (it.x - x).hypot(it.y - y) < gap + it.w / 2.0);
        if hits_icon || crowded {
            continue;
        }
        return Some(make_item(kind, x, y, w, h, rand));
    }
    None
}

fn make_item(kind: Kind, x: f32, y: f32, w: f32, h: f32, rand: &mut SeededRandom) -> Item {
    let phase = rand.next() * 6.0;
    let lobes = if matches!(kind, Kind::Tree | Kind::Bush) { make_lobes(kind, w, h, rand) } else { Vec::new() };
    let footprint = (kind != Kind::Mushroom).then(|| {
        let fw = if kind == Kind::Tree { w * 0.22 } else { w * 0.8 };
        Obstacle { x: x - fw / 2.0, y: y - FOOTPRINT_H, w: fw, h: FOOTPRINT_H, ground: true }
    });
    Item { kind, x, y, w, h, phase, shake: 0.0, lobes, footprint }
}

/// Tán lá: các thùy tròn đặt tương đối với gốc (0,0).
fn make_lobes(kind: Kind, w: f32, h: f32, rand: &mut SeededRandom) -> Vec<[f32; 4]> {
    let tree = kind == Kind::Tree;
    let n = if tree { 10 } else { 6 };
    let cy = if tree { -h * 0.66 } else { -h * 0.48 };
    let (spread_x, spread_y) = if tree { (w * 0.4, h * 0.2) } else { (w * 0.36, h * 0.22) };
    let mut lobes: Vec<[f32; 4]> = (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32 - 0.5;
            let arch = 1.0 - t.abs() * 2.0;
            let lx = t * spread_x * 2.0 + (rand.next() - 0.5) * w * 0.08;
            let ly = cy - arch * spread_y + (rand.next() - 0.5) * spread_y;
            [lx, ly, w * (0.19 + rand.next() * 0.08), 0.0]
        })
        .collect();
    if tree {
        // Lớp lá đáy tán nằm ngay trên ngọn thân cây để tán và thân liền một khối.
        for dx in [-0.17, 0.0, 0.17] {
            lobes.push([dx * w, -h * TREE_CANOPY_BASE, w * (0.17 + rand.next() * 0.04), 0.0]);
        }
    }
    lobes
}

/// Độ lệch theo gió của một điểm ở độ cao `height_ratio` (0 = gốc, 1 = ngọn); dùng chung cho thân và tán.
fn sway_shift(sway: f32, height_ratio: f32) -> f32 {
    sway * 8.0 * (height_ratio * 1.2).min(1.0)
}

fn nature_sway(item: &Item, wind: f32, time: f32) -> f32 {
    let shake = (time * 22.0 + item.phase).sin() * item.shake * 1.6;
    wind * 0.45 + (time * 1.8 + item.phase).sin() * 0.3 * (wind + 0.2).min(1.5) + shake
}

pub fn step_shake(items: &mut [Item], dt: f32) {
    for i in items.iter_mut() {
        i.shake = (i.shake - dt * SHAKE_DECAY).max(0.0);
    }
}

/// Tán cây / lùm cây dưới con trỏ; ưu tiên cái vẽ sau cùng (gần người xem hơn).
pub fn item_at(items: &mut [Item], x: f32, y: f32) -> Option<&mut Item> {
    let hit = |i: &Item| {
        let tree = i.kind == Kind::Tree;
        let (top, bottom) = (i.y - i.h * if tree { 0.98 } else { 1.0 }, i.y - if tree { i.h * 0.3 } else { 0.0 });
        !i.lobes.is_empty() && (x - i.x).abs() < i.w * 0.5 && y > top && y < bottom
    };
    items.iter_mut().filter(|i| hit(i)).max_by(|a, b| a.y.total_cmp(&b.y))
}

fn paint_shadow(ctx: &mut Canvas, x: f32, y: f32, rx: f32, ry: f32) {
    ctx.begin();
    ctx.oval(x, y, rx, ry, 0.0);
    ctx.fill(rgba(55, 110, 20, 0.28));
}

pub fn draw_item(ctx: &mut Canvas, item: &Item, wind: f32, time: f32) {
    match item.kind {
        Kind::Tree => paint_tree(ctx, item, wind, time),
        Kind::Bush => {
            paint_shadow(ctx, item.x + 4.0, item.y, item.w * 0.55, 8.0);
            paint_canopy(ctx, item, wind, time);
        }
        Kind::Rock => paint_rock(ctx, item),
        Kind::Mushroom => paint_mushroom(ctx, item),
    }
}

fn paint_canopy(ctx: &mut Canvas, item: &Item, wind: f32, time: f32) {
    let sway = nature_sway(item, wind, time);
    let lobes: Vec<[f32; 4]> = item.lobes.iter().map(|&[lx, ly, lr, _]| [item.x + lx, item.y + ly, lr, -ly / item.h]).collect();
    paint_foliage_layers(ctx, &lobes, |lobe| sway_shift(sway, lobe[3]));
}

/// Thân cây cắm sâu vào trong tán (TREE_TRUNK_TOP) và có hai cành chĩa lên, nên tán luôn liền với thân.
fn paint_tree(ctx: &mut Canvas, t: &Item, wind: f32, time: f32) {
    let (sway, tw) = (nature_sway(t, wind, time), t.w * 0.09);
    let (top_y, top_shift, mid_y) = (t.y - t.h * TREE_TRUNK_TOP, sway_shift(sway, TREE_TRUNK_TOP), t.y - t.h * 0.2);
    paint_shadow(ctx, t.x + t.w * 0.08, t.y, t.w * 0.55, t.w * 0.13);
    paint_branches(ctx, t, sway);
    ctx.begin();
    ctx.move_to(t.x - tw * 1.9, t.y);
    ctx.quad_to(t.x - tw, t.y - 10.0, t.x - tw * 0.8, mid_y);
    ctx.line_to(t.x - tw * 0.5 + top_shift, top_y);
    ctx.line_to(t.x + tw * 0.5 + top_shift, top_y);
    ctx.line_to(t.x + tw * 0.8, mid_y);
    ctx.quad_to(t.x + tw, t.y - 10.0, t.x + tw * 1.9, t.y);
    ctx.close();
    ctx.fill(TRUNK_DARK);
    ctx.begin();
    ctx.move_to(t.x - tw * 1.5, t.y - 1.0);
    ctx.quad_to(t.x - tw * 0.8, t.y - 10.0, t.x - tw * 0.6, mid_y);
    ctx.line_to(t.x - tw * 0.35 + top_shift, top_y);
    ctx.line_to(t.x + tw * 0.1 + top_shift, top_y);
    ctx.line_to(t.x + tw * 0.1, t.y - 1.0);
    ctx.close();
    ctx.fill(TRUNK_LIGHT);
    paint_canopy(ctx, t, wind, time);
}

fn paint_branches(ctx: &mut Canvas, t: &Item, sway: f32) {
    let (fork_y, reach) = (t.y - t.h * 0.45, t.h * 0.66);
    for side in [-1.0, 1.0] {
        ctx.begin();
        ctx.move_to(t.x, fork_y);
        ctx.quad_to(t.x + side * t.w * 0.12, fork_y - t.h * 0.08, t.x + side * t.w * 0.24 + sway_shift(sway, 0.66), t.y - reach);
        ctx.stroke(TRUNK_DARK, t.w * 0.05);
    }
}

fn paint_rock(ctx: &mut Canvas, r: &Item) {
    paint_shadow(ctx, r.x + 4.0, r.y, r.w * 0.58, r.h * 0.22);
    let [dark, mid, light] = ROCK_TONES;
    ctx.begin();
    ctx.oval(r.x, r.y - r.h * 0.45, r.w / 2.0, r.h * 0.55, 0.0);
    ctx.fill(dark);
    ctx.begin();
    ctx.oval(r.x - r.w * 0.06, r.y - r.h * 0.55, r.w * 0.42, r.h * 0.42, 0.0);
    ctx.fill(mid);
    ctx.begin();
    ctx.oval(r.x - r.w * 0.16, r.y - r.h * 0.72, r.w * 0.16, r.h * 0.14, -0.3);
    ctx.fill(light);
    for dx in [-0.42, 0.38] {
        ctx.begin();
        ctx.circle(r.x + dx * r.w, r.y - 3.0, 6.0);
        ctx.circle(r.x + dx * r.w + 6.0, r.y - 2.0, 5.0);
        ctx.fill(FOLIAGE_TONES[2]);
    }
}

fn paint_mushroom(ctx: &mut Canvas, m: &Item) {
    ctx.begin();
    ctx.move_to(m.x - 3.0, m.y - 10.0);
    ctx.line_to(m.x + 3.0, m.y - 10.0);
    ctx.line_to(m.x + 3.0, m.y);
    ctx.line_to(m.x - 3.0, m.y);
    ctx.close();
    ctx.fill(hex(0xf6ead2));
    ctx.begin();
    ctx.ellipse(m.x, m.y - 10.0, 9.0, 7.0, 0.0, std::f32::consts::PI, std::f32::consts::TAU);
    ctx.fill(MUSHROOM_CAP);
    for (dx, dy) in [(-4.0, -13.0), (2.0, -15.0), (4.0, -11.0)] {
        ctx.begin();
        ctx.circle(m.x + dx, m.y + dy, 1.4);
        ctx.fill(hex(0xffffff));
    }
}
