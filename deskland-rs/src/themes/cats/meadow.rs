//! Nền tĩnh (port từ prototype/cats-meadow.js): trời và phong cảnh xa (scenery.rs), đồi cỏ xanh tươi, ngọn cỏ,
//! nắng, gò cỏ sáng quanh biểu tượng, hoa và khung viền. Vẽ một lần vào canvas nền, chỉ vẽ lại khi bố cục đổi.

use super::canvas::{hex, radial_paint, rgba, Canvas, Rgba};
use super::scenery::{paint_distant_land, paint_frame, paint_sky};
use crate::engine::IconBox;
use std::f32::consts::{PI, TAU};

const HORIZON: f32 = 0.24; // tỉ lệ chiều cao dành cho trời và núi xa
                           // Màu nền trang cats.html: lộ ra ở khe giữa dãy núi xa và mép phải, như trên trình duyệt
const PAGE_BG: Rgba = hex(0xb6da89);
const GRASS: [Rgba; 3] = [hex(0xc2e852), hex(0xa3d23f), hex(0x86bb33)];
const BLADE_LIGHT: Rgba = rgba(225, 252, 150, 0.6);
const BLADE_DARK: Rgba = rgba(85, 145, 30, 0.4);
const MOUND: Rgba = rgba(214, 245, 120, 0.9);
const MOUND_GLOW_STEPS: usize = 6; // thay cho shadowBlur 22px của Canvas: các lớp viền mờ dần
const SUN: (u8, u8, u8) = (255, 240, 170);
const FLOWERS: [(Rgba, Rgba); 4] =
    [(hex(0xffffff), hex(0xf6c430)), (hex(0xfff27a), hex(0xf0a020)), (hex(0xffffff), hex(0xf6c430)), (hex(0xffd6e0), hex(0xf7a8b8))];
const FLOWER_CLUSTER_AREA: f32 = 22000.0; // px² cho mỗi khóm hoa ở mật độ 100%
const BLADE_AREA: f32 = 1500.0;

pub fn horizon(h: f32) -> f32 {
    h * HORIZON
}

/// mulberry32 giống hệt catMakeRandom của bản web: cùng seed thì cùng bố cục với khung xem trước trong Cài đặt.
pub struct SeededRandom(u32);

impl SeededRandom {
    pub fn new(seed: i32) -> Self {
        SeededRandom(seed as u32)
    }
    pub fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let a = self.0;
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        ((t ^ (t >> 14)) as f64 / 4_294_967_296.0) as f32
    }
}

/// Thông số vẽ nền (toạ độ CSS).
pub struct Backdrop<'a> {
    pub w: f32,
    pub h: f32,
    pub taskbar: f32,
    pub seed: i32,
    pub icons: &'a [IconBox],
    pub mound: f32,
    pub sun: f32,
    pub flowers: f32,
}

pub fn paint_meadow(ctx: &mut Canvas, o: &Backdrop) {
    let hz = horizon(o.h);
    ctx.clear(PAGE_BG);
    let mut rand = SeededRandom::new(o.seed ^ 0x5eed);
    paint_sky(ctx, o.w, hz, &mut rand);
    paint_distant_land(ctx, o.w, hz, &mut rand);
    paint_hill(ctx, o.w, o.h, hz);
    paint_blades(ctx, o.w, o.h, hz, &mut rand);
    paint_sun_glow(ctx, o.w, o.h, o.sun);
    paint_mounds(ctx, o.icons, o.mound);
    paint_flowers(ctx, o, hz, &mut SeededRandom::new(o.seed ^ 0xf10e));
    paint_frame(ctx, o, &mut SeededRandom::new(o.seed ^ 0xf4a3));
}

/// Đồi cỏ chính hình vòm: giữa cao hơn hai bên, sáng ở giữa và đậm dần ra mép.
fn paint_hill(ctx: &mut Canvas, w: f32, h: f32, hz: f32) {
    ctx.begin();
    ctx.move_to(0.0, h);
    let mut x = 0.0;
    while x <= w + 20.0 {
        ctx.line_to(x, hz + 62.0 - (PI * x.min(w) / w).sin() * 42.0);
        x += 20.0;
    }
    ctx.line_to(w, h);
    ctx.close();
    let stops: Vec<(f32, Rgba)> = GRASS.iter().enumerate().map(|(i, &c)| (i as f32 / (GRASS.len() - 1) as f32, c)).collect();
    ctx.fill_paint(&radial_paint(w / 2.0, h * 0.6, w.max(h) * 0.75, &stops));
}

fn paint_blades(ctx: &mut Canvas, w: f32, h: f32, hz: f32, rand: &mut SeededRandom) {
    let count = (w * (h - hz) / BLADE_AREA).round() as usize;
    for _ in 0..count {
        let (x, y, bh) = (rand.next() * w, hz + 40.0 + rand.next() * (h - hz), 4.0 + rand.next() * 4.0);
        let color = if rand.next() < 0.55 { BLADE_LIGHT } else { BLADE_DARK };
        ctx.begin();
        ctx.move_to(x - 2.5, y - bh * 0.8);
        ctx.line_to(x, y);
        ctx.line_to(x + 2.5, y - bh);
        ctx.stroke(color, 1.5);
    }
}

fn paint_sun_glow(ctx: &mut Canvas, w: f32, h: f32, strength: f32) {
    if strength <= 0.0 {
        return;
    }
    let (r, g, b) = SUN;
    let stops = [(0.0, rgba(r, g, b, 0.5 * strength)), (1.0, rgba(r, g, b, 0.0))];
    ctx.fill_rect(0.0, 0.0, w, h, &radial_paint(w * 0.9, 0.0, w.max(h) * 0.7, &stops));
}

/// Gò cỏ sáng mềm quanh biểu tượng (viền nhoè, không có đường kẻ) để biểu tượng nổi trên nền.
fn paint_mounds(ctx: &mut Canvas, icons: &[IconBox], pad: f32) {
    if icons.is_empty() {
        return;
    }
    let mound_path = |ctx: &mut Canvas, grow: f32| {
        ctx.begin();
        for ic in icons {
            let p = pad + grow;
            ctx.round_rect(ic.x - p, ic.y - pad * 0.6 - grow, ic.w + 2.0 * p, ic.h + 1.2 * pad + 2.0 * grow, pad + 18.0 + grow);
        }
    };
    for i in (1..=MOUND_GLOW_STEPS).rev() {
        mound_path(ctx, i as f32 * 2.5);
        ctx.fill(MOUND.with_alpha(0.13));
    }
    mound_path(ctx, 0.0);
    ctx.fill(MOUND);
}

fn paint_flowers(ctx: &mut Canvas, o: &Backdrop, hz: f32, rand: &mut SeededRandom) {
    let clusters = (o.flowers * o.w * (o.h - hz) / FLOWER_CLUSTER_AREA).round() as usize;
    for _ in 0..clusters {
        let cx = rand.next() * o.w;
        let cy = hz + 50.0 + rand.next() * (o.h - o.taskbar - hz - 50.0);
        let colors = FLOWERS[((rand.next() * FLOWERS.len() as f32) as usize).min(FLOWERS.len() - 1)];
        let n = 2 + (rand.next() * 4.0) as usize;
        for _ in 0..n {
            let x = cx + (rand.next() - 0.5) * 30.0;
            let y = cy + (rand.next() - 0.5) * 16.0;
            if o.icons.iter().any(|ic| x > ic.x - 6.0 && x < ic.x + ic.w + 6.0 && y > ic.y - 6.0 && y < ic.y + ic.h + 6.0) {
                continue;
            }
            let r = 3.0 + rand.next() * 1.8;
            paint_flower(ctx, x, y, r, colors);
        }
    }
}

fn paint_flower(ctx: &mut Canvas, x: f32, y: f32, r: f32, (petal, center): (Rgba, Rgba)) {
    ctx.begin();
    ctx.oval(x + 1.0, y + r * 1.4, r * 1.3, r * 0.45, 0.0);
    ctx.fill(rgba(70, 130, 30, 0.35));
    for i in 0..5 {
        let a = i as f32 / 5.0 * TAU;
        ctx.begin();
        ctx.circle(x + a.cos() * r, y + a.sin() * r, r * 0.78);
        ctx.fill(petal);
    }
    ctx.begin();
    ctx.circle(x, y, r * 0.62);
    ctx.fill(center);
}
