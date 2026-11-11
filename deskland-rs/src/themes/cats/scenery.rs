//! Phong cảnh xa và khung viền tĩnh (port từ prototype/cats-scenery.js): bầu trời có mây, núi xa,
//! cùng lùm cây và mây trắng ở mép màn hình. Được meadow.rs vẽ vào canvas nền.

use super::canvas::{hex, vertical_paint, Canvas, Rgba};
use super::meadow::{Backdrop, SeededRandom};

const SKY: [Rgba; 3] = [hex(0x36a3ee), hex(0x8fd2f7), hex(0xd4f0fb)];
const CLOUD: Rgba = hex(0xffffff);
const CLOUD_SHADE: Rgba = hex(0xdcedf8);
const FAR_HILLS: Rgba = hex(0x86c3a2);
const NEAR_HILLS: Rgba = hex(0x74bd57);
const TINY_TREE: Rgba = hex(0x4f9a3a);
pub const FOLIAGE_TONES: [Rgba; 4] = [hex(0x3e8e2c), hex(0x66b63a), hex(0xa3da4c), hex(0xd6f27a)];
/// Vị trí khung viền: [x theo W, y theo chiều cao vùng cỏ, độ lớn].
const FRAME_BUSHES: [[f32; 3]; 3] = [[0.0, 1.0, 1.25], [0.9, 1.03, 1.05], [1.0, 0.6, 0.9]];
const FRAME_CLOUDS: [[f32; 3]; 3] = [[0.03, 0.98, 1.0], [0.45, 1.07, 1.1], [0.985, 0.74, 0.9]];
const CLOUD_PUFFS: [[f32; 3]; 5] = [[-1.6, 0.25, 0.7], [-0.8, -0.2, 0.95], [0.0, -0.45, 1.15], [0.85, -0.15, 0.95], [1.6, 0.25, 0.7]];

pub fn paint_sky(ctx: &mut Canvas, w: f32, hz: f32, rand: &mut SeededRandom) {
    let stops: Vec<(f32, Rgba)> = SKY.iter().enumerate().map(|(i, &c)| (i as f32 / (SKY.len() - 1) as f32, c)).collect();
    ctx.fill_rect(0.0, 0.0, w, hz + 40.0, &vertical_paint(0.0, hz + 40.0, &stops));
    let count = 3 + (w / 600.0).round() as usize;
    for i in 0..count {
        let x = (i as f32 + 0.2 + rand.next() * 0.6) / count as f32 * w;
        let y = hz * (0.2 + rand.next() * 0.45);
        paint_cloud(ctx, x, y, 26.0 + rand.next() * 18.0, rand);
    }
}

pub fn paint_cloud(ctx: &mut Canvas, x: f32, y: f32, r: f32, rand: &mut SeededRandom) {
    for (color, dy, k) in [(CLOUD_SHADE, r * 0.18, 1.0), (CLOUD, 0.0, 0.95)] {
        ctx.begin();
        for [px, py, pr] in CLOUD_PUFFS {
            let rr = pr * r * k * (1.0 + (rand.next() - 0.5) * 0.15);
            ctx.move_to(x + px * r + rr, y + py * r + dy);
            ctx.circle(x + px * r, y + py * r + dy, rr);
        }
        ctx.fill(color);
    }
}

pub fn paint_distant_land(ctx: &mut Canvas, w: f32, hz: f32, rand: &mut SeededRandom) {
    let bumps = 5.0 + rand.next() * 3.0;
    paint_ridge(ctx, w, hz + 20.0, hz * 0.45, bumps, FAR_HILLS, rand);
    let bumps = 7.0 + rand.next() * 4.0;
    paint_ridge(ctx, w, hz + 52.0, hz * 0.22, bumps, NEAR_HILLS, rand);
    let mut i = 0.0;
    while i < w / 70.0 {
        let (x, y, r) = (rand.next() * w, hz + 18.0 + rand.next() * 30.0, 4.0 + rand.next() * 4.0);
        ctx.begin();
        ctx.circle(x, y, r);
        ctx.circle(x + r * 0.9, y + 1.0, r * 0.8);
        ctx.fill(TINY_TREE);
        i += 1.0;
    }
}

fn paint_ridge(ctx: &mut Canvas, w: f32, base: f32, height: f32, bumps: f32, color: Rgba, rand: &mut SeededRandom) {
    ctx.begin();
    ctx.move_to(0.0, base + 60.0);
    let step = w / bumps;
    let mut i = 0.0;
    while i <= bumps {
        let peak = base - height * (0.5 + rand.next() * 0.5);
        let end = base - height * 0.25 * rand.next();
        ctx.quad_to(i * step - step / 2.0, peak, i * step, end);
        i += 1.0;
    }
    ctx.line_to(w, base + 60.0);
    ctx.close();
    ctx.fill(color);
}

fn scenery_blocked(o: &Backdrop, x: f32, y: f32, r: f32) -> bool {
    let clear = o.mound + 10.0;
    o.icons.iter().any(|ic| x + r > ic.x - clear && x - r < ic.x + ic.w + clear && y + r > ic.y - clear && y - r < ic.y + ic.h + clear)
}

pub fn paint_frame(ctx: &mut Canvas, o: &Backdrop, rand: &mut SeededRandom) {
    let ground_h = o.h - o.taskbar;
    for [fx, fy, size] in FRAME_BUSHES {
        let (x, y, r) = (fx * o.w, fy * ground_h, 70.0 * size);
        if !scenery_blocked(o, x, y, r) {
            paint_foliage_clump(ctx, x, y, r, rand);
        }
    }
    for [fx, fy, size] in FRAME_CLOUDS {
        let (x, y, r) = (fx * o.w, fy * ground_h, 40.0 * size);
        if !scenery_blocked(o, x, y, r * 2.0) {
            paint_cloud(ctx, x, y, r, rand);
        }
    }
}

fn paint_foliage_clump(ctx: &mut Canvas, x: f32, y: f32, r: f32, rand: &mut SeededRandom) {
    let lobes: Vec<[f32; 4]> = (0..8)
        .map(|_| {
            let a = std::f32::consts::PI + rand.next() * std::f32::consts::PI;
            let d = r * (0.25 + rand.next() * 0.5);
            [x + a.cos() * d, y + a.sin() * d * 0.8, r * (0.32 + rand.next() * 0.2), 0.0]
        })
        .collect();
    paint_foliage_layers(ctx, &lobes, |_| 0.0);
}

/// Tô một cụm tán lá theo 4 tông (tối → sáng) để có khối như tranh vẽ tay. lobe = [x, y, r, độ cao tương đối].
pub fn paint_foliage_layers(ctx: &mut Canvas, lobes: &[[f32; 4]], shift_of: impl Fn(&[f32; 4]) -> f32) {
    const LAYERS: [(usize, f32, f32, f32); 4] = [(0, 1.0, 0.0, 0.0), (1, 0.86, -0.1, -0.12), (2, 0.55, -0.26, -0.3), (3, 0.2, -0.42, -0.45)];
    for (tone, k, dx, dy) in LAYERS {
        ctx.begin();
        for lobe in lobes {
            let [lx, ly, lr, _] = *lobe;
            let (cx, cy) = (lx + shift_of(lobe) + dx * lr, ly + dy * lr);
            ctx.move_to(cx + lr * k, cy);
            ctx.circle(cx, cy, lr * k);
        }
        ctx.fill(FOLIAGE_TONES[tone]);
    }
}
