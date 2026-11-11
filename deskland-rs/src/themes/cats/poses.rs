//! Các tư thế của chú mèo mập (port từ prototype/cats-poses.js): đi, ngồi, ngồi liếm lông, ngủ cuộn tròn,
//! nằm nghiêng và nằm ngửa. Gốc toạ độ là điểm giữa hai chân trên mặt cỏ, hình quay mặt sang phải.

use super::behavior::{Cat, CatState, SleepPose};
use super::canvas::{hex, rgba, Canvas, Rgba};
use super::draw::CoatKind::{Calico, Solid};
use super::draw::{leg_color, HeadOpts, Pen, OUTLINE};
use super::interact::{hop_lift, LOVE_SECONDS};
use std::f32::consts::PI;

const HEART: Rgba = hex(0xff6f91);
const CUSHION: [Rgba; 2] = [hex(0xef8fa8), hex(0xf8b7c8)];
const SNORE: Rgba = hex(0x4f6f86);

fn shadow_rx(state: CatState) -> f32 {
    match state {
        CatState::Walk => 32.0,
        CatState::Sit | CatState::Groom | CatState::Play => 26.0,
        CatState::Sleep => 34.0,
    }
}

pub fn draw_cat(ctx: &mut Canvas, cat: &Cat) {
    let k = cat.size / 60.0;
    ctx.save();
    ctx.translate(cat.x, cat.y);
    if cat.state == CatState::Sleep && cat.cushion {
        draw_cushion(ctx, k);
    }
    let lift = hop_lift(cat);
    ctx.begin();
    ctx.oval(0.0, -1.0, shadow_rx(cat.state) * k * (1.0 - lift * 0.02), 6.0 * k, 0.0);
    ctx.fill(rgba(30, 60, 15, 0.30));
    ctx.translate(0.0, -lift * k);
    ctx.scale(cat.dir * k, k);
    if cat.state == CatState::Walk {
        draw_motion_marks(ctx, cat);
    }
    draw_pose(&mut Pen { ctx: &mut *ctx, outline: true }, cat);
    draw_pose(&mut Pen { ctx: &mut *ctx, outline: false }, cat);
    ctx.restore();
    if cat.state == CatState::Sleep {
        draw_snore(ctx, cat, k);
    }
    if cat.heart > 0.0 {
        draw_heart_pop(ctx, cat, k);
    }
    if cat.fun.love > 0.0 {
        draw_love_burst(ctx, cat, k);
    }
}

fn draw_pose(pen: &mut Pen, cat: &Cat) {
    match cat.state {
        CatState::Sleep => match cat.sleep_pose {
            SleepPose::Loaf => loaf_sleeping(pen, cat),
            SleepPose::Side => side_sleeping(pen, cat),
            SleepPose::Back => back_sleeping(pen, cat),
        },
        CatState::Groom => grooming(pen, cat),
        CatState::Play => playing(pen, cat),
        CatState::Sit => sitting(pen, cat),
        CatState::Walk => walking(pen, cat),
    }
}

fn walking(pen: &mut Pen, cat: &Cat) {
    let (c, p) = (cat.coat(), cat.phase);
    let bob = p.sin().abs() * 1.3;
    pen.tail(c.tail, -27.0, -29.0 - bob, [-38.0, -33.0, -41.0 + cat.sway * 5.0, -48.0, -33.0 + cat.sway * 8.0, -57.0]);
    pen.walk_leg(c.shade, c.shade, (14.0, -10.0 - bob), p + PI);
    pen.walk_leg(c.shade, c.shade, (-16.0, -10.0 - bob), p);
    pen.body(c, 0.0, -24.0 - bob, 30.0, 20.0);
    pen.walk_leg(leg_color(c), c.paw, (9.0, -10.0 - bob), p);
    pen.walk_leg(leg_color(c), c.paw, (-21.0, -10.0 - bob), p + PI);
    pen.head(cat, 27.0, -43.0 - bob, HeadOpts::default());
}

fn sitting_base(pen: &mut Pen, cat: &Cat) {
    let c = cat.coat();
    pen.tail(c.tail, -18.0, -5.0, [-8.0, 7.0, 10.0, 7.0, 23.0 + cat.sway * 3.0, -1.0 - cat.sway.abs() * 4.0]);
    pen.fill_ellipse(c.body, -2.0, -22.0, 24.0, 22.0, 0.0);
    if c.kind != Solid {
        pen.detail_ellipse(c.belly, 7.0, -19.0, 11.0, 14.0, -0.1);
    }
    if c.kind == Calico {
        pen.detail_ellipse(c.mark, -11.0, -28.0, 9.0, 10.0, 0.3);
    }
}

fn sitting(pen: &mut Pen, cat: &Cat) {
    let c = cat.coat();
    sitting_base(pen, cat);
    pen.leg_line(leg_color(c), c.paw, (4.0, -14.0), (4.0, -5.0));
    pen.leg_line(leg_color(c), c.paw, (13.0, -14.0), (13.0, -5.0));
    pen.head(cat, 5.0, -49.0, HeadOpts::default());
}

/// Ngồi liếm lông: một chân trước đưa lên miệng, đầu cúi theo nhịp liếm, mắt lim dim, lưỡi thè ra.
fn grooming(pen: &mut Pen, cat: &Cat) {
    let (c, lick) = (cat.coat(), (cat.clock * 6.0).sin());
    sitting_base(pen, cat);
    pen.leg_line(leg_color(c), c.paw, (4.0, -14.0), (4.0, -5.0));
    pen.head(cat, 9.0, -47.0 + lick * 0.8, HeadOpts { asleep: true, tilt: 0.22 + lick * 0.06, tongue: lick.max(0.0) });
    pen.leg_line(leg_color(c), c.paw, (13.0, -24.0), (19.0, -37.0 - lick * 2.5));
}

/// Vờn cuộn len: chân trước khều xuống đất theo nhịp, đầu cúi nhìn cuộn len.
fn playing(pen: &mut Pen, cat: &Cat) {
    let (c, tap) = (cat.coat(), (cat.clock * 7.0).sin().max(0.0));
    sitting_base(pen, cat);
    pen.leg_line(leg_color(c), c.paw, (4.0, -14.0), (4.0, -5.0));
    pen.head(cat, 8.0, -47.0 + tap * 0.6, HeadOpts { asleep: false, tilt: 0.2, tongue: 0.0 });
    pen.leg_line(leg_color(c), c.paw, (13.0, -22.0), (27.0 + tap * 3.0, -8.0 - tap * 9.0));
}

fn loaf_sleeping(pen: &mut Pen, cat: &Cat) {
    let (c, breath) = (cat.coat(), (cat.clock * 1.6).sin());
    pen.body(c, -2.0, -15.0 - breath * 0.6, 31.0, 16.0 + breath * 1.8);
    pen.tail(c.tail, -29.0, -6.0, [-23.0, 9.0, 8.0, 8.0, 25.0, 0.0]);
    pen.head(cat, 21.0, -18.0 - breath * 0.9, HeadOpts { asleep: true, tilt: 0.12, tongue: 0.0 });
}

/// Nằm nghiêng duỗi người: bụng phồng lên xẹp xuống rõ theo nhịp thở.
fn side_sleeping(pen: &mut Pen, cat: &Cat) {
    let (c, breath) = (cat.coat(), (cat.clock * 1.6).sin());
    pen.tail(c.tail, -30.0, -8.0, [-42.0, -5.0, -50.0, 0.0, -60.0, -6.0]);
    pen.leg_line(leg_color(c), c.paw, (-15.0, -10.0), (-8.0, -4.0));
    pen.leg_line(leg_color(c), c.paw, (13.0, -10.0), (21.0, -4.0));
    pen.body(c, 0.0, -15.0 - breath * 0.9, 33.0, 13.0 + breath * 2.0);
    pen.head(cat, 33.0, -13.0, HeadOpts { asleep: true, tilt: 0.32, tongue: 0.0 });
}

/// Nằm ngửa phơi bụng, bốn chân co lên trời, bụng phồng xẹp theo nhịp thở.
fn back_sleeping(pen: &mut Pen, cat: &Cat) {
    let (c, breath) = (cat.coat(), (cat.clock * 1.6).sin());
    pen.tail(c.tail, -28.0, -8.0, [-40.0, -4.0, -46.0, 2.0, -56.0, -2.0]);
    for (x, lean) in [(-16.0, -3.0), (-4.0, -1.0), (8.0, 2.0), (18.0, 4.0)] {
        pen.leg_line(leg_color(c), c.paw, (x, -24.0), (x + lean, -39.0 - breath * 0.6));
    }
    pen.fill_ellipse(c.body, 0.0, -15.0, 29.0, 15.0 + breath * 1.2, 0.0);
    let belly = if c.kind == Solid { c.shade } else { c.belly };
    pen.detail_ellipse(belly, 2.0, -18.0 - breath * 0.6, 20.0, 9.0 + breath * 1.4, 0.0);
    pen.head(cat, 31.0, -14.0, HeadOpts { asleep: true, tilt: -0.45, tongue: 0.0 });
}

fn draw_cushion(ctx: &mut Canvas, k: f32) {
    for (color, y, rx, ry) in [(OUTLINE, -4.0, 46.0, 14.0), (CUSHION[0], -4.0, 44.5, 12.5), (CUSHION[1], -7.0, 36.0, 8.0)] {
        ctx.begin();
        ctx.oval(0.0, y * k, rx * k, ry * k, 0.0);
        ctx.fill(color);
    }
}

/// Vài vệt trắng nhỏ phía sau khi mèo đang đi, giống nét "chuyển động" trong tranh hoạt hình.
fn draw_motion_marks(ctx: &mut Canvas, cat: &Cat) {
    let wiggle = (cat.phase * 2.0).sin() * 2.0;
    for (y, len) in [(-34.0, 9.0), (-24.0, 12.0), (-14.0, 8.0)] {
        ctx.begin();
        ctx.move_to(-40.0 + wiggle, y);
        ctx.line_to(-40.0 - len + wiggle, y + 2.0);
        ctx.stroke(rgba(255, 255, 255, 0.85), 2.2);
    }
}

/// Chữ "z" bay lên khi mèo ngủ (bản web dùng fillText; ở đây vẽ bằng nét để không cần font).
fn draw_snore(ctx: &mut Canvas, cat: &Cat, k: f32) {
    let cycle = (cat.clock * 0.45) % 1.0;
    let size = ((10.0 + cycle * 6.0) * k).round();
    let x = cat.x + cat.dir * (26.0 + cycle * 9.0) * k;
    let y = cat.y - (36.0 + cycle * 24.0) * k;
    let (hw, top) = (size * 0.28, y - size * 0.55);
    ctx.save();
    ctx.set_alpha((cycle * PI).sin() * 0.85);
    ctx.begin();
    ctx.move_to(x, top);
    ctx.line_to(x + hw * 2.0, top);
    ctx.line_to(x, y);
    ctx.line_to(x + hw * 2.0, y);
    ctx.stroke(SNORE, (size * 0.14).max(1.2));
    ctx.restore();
}

/// Trái tim nhỏ bay lên khi hai chú mèo ngồi cạnh nhau.
fn draw_heart_pop(ctx: &mut Canvas, cat: &Cat, k: f32) {
    let life = cat.heart.min(1.0);
    let rise = (1.0 - life) * 14.0;
    draw_heart(ctx, cat.x + cat.dir * 6.0 * k, cat.y - (80.0 + rise) * k, 5.0 * k, (life * 2.0).min(1.0));
}

/// Chùm ba trái tim toả lên khi được vuốt ve.
fn draw_love_burst(ctx: &mut Canvas, cat: &Cat, k: f32) {
    let t = 1.0 - cat.fun.love / LOVE_SECONDS;
    for i in 0..3 {
        let p = ((t - i as f32 * 0.15) / 0.7).clamp(0.0, 1.0);
        if p <= 0.0 || p >= 1.0 {
            continue;
        }
        let x = cat.x + ((i as f32 - 1.0) * 18.0 * p + cat.dir * 4.0) * k;
        let y = cat.y - (68.0 + p * 42.0 + if i == 1 { 8.0 } else { 0.0 }) * k;
        draw_heart(ctx, x, y, (4.0 + p * 2.5) * k, (p * PI).sin());
    }
}

fn draw_heart(ctx: &mut Canvas, x: f32, y: f32, r: f32, alpha: f32) {
    ctx.save();
    ctx.set_alpha(alpha);
    ctx.begin();
    ctx.arc(x - r, y, r, PI, 0.0);
    ctx.arc(x + r, y, r, PI, 0.0);
    ctx.line_to(x, y + r * 2.4);
    ctx.close();
    ctx.fill(HEART);
    ctx.restore();
}
