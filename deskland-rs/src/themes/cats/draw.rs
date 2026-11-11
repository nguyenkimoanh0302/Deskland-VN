//! Bộ phận cơ thể của chú mèo mập (port từ prototype/cats-draw.js). Mỗi tư thế được vẽ hai lượt: lượt viền
//! (tô cả hình bằng màu viền, nét dày) rồi lượt màu phủ lên, nhờ vậy cả dáng mèo có một đường viền liền mạch.

use super::behavior::Cat;
use super::canvas::{hex, rgba, Canvas, Rgba};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, PartialEq)]
pub enum CoatKind {
    Tabby,
    Tux,
    Calico,
    Solid,
    Point,
    Patch,
}

pub struct Coat {
    pub body: Rgba,
    pub shade: Rgba,
    pub belly: Rgba,
    pub mark: Rgba,
    pub tail: Rgba,
    pub paw: Rgba,
    pub kind: CoatKind,
    pub eye: Rgba,
}

const fn coat(c: [u32; 7], kind: CoatKind) -> Coat {
    Coat { body: hex(c[0]), shade: hex(c[1]), belly: hex(c[2]), mark: hex(c[3]), tail: hex(c[4]), paw: hex(c[5]), kind, eye: hex(c[6]) }
}

use CoatKind::*;
pub static COATS: [Coat; 10] = [
    coat([0xf6a85a, 0xdc8a3e, 0xfee6c8, 0xd4742c, 0xf6a85a, 0xfee6c8, 0x6fb83f], Tabby),
    coat([0xa6acb4, 0x838990, 0xe6e9ed, 0x6c7178, 0xa6acb4, 0xe6e9ed, 0xe8b730], Tabby),
    coat([0x3b3635, 0x272322, 0xf7f3ed, 0x3b3635, 0x3b3635, 0xf7f3ed, 0xe8b730], Tux),
    coat([0xfdf8f0, 0xe2d9cc, 0xffffff, 0xea9f55, 0x433a3c, 0xffffff, 0x6fb83f], Calico),
    coat([0x433d3f, 0x2c2729, 0x433d3f, 0x433d3f, 0x433d3f, 0x433d3f, 0xf2c43a], Solid),
    coat([0xf6e6cb, 0x755646, 0xfff8ec, 0x755646, 0x755646, 0x755646, 0x58a8e0], Point),
    coat([0xf3dcb0, 0xdcbf8c, 0xfff6e6, 0xd9b27a, 0xf3dcb0, 0xfff6e6, 0x58a8e0], Tabby),
    coat([0xffffff, 0xe6e6ea, 0xffffff, 0xffffff, 0xffffff, 0xffffff, 0x58a8e0], Solid),
    coat([0xfdfdfb, 0xdedcd8, 0xffffff, 0x8d8f94, 0x8d8f94, 0xffffff, 0x6fb83f], Patch),
    coat([0xfdfdfb, 0xdedcd8, 0xffffff, 0x5e6066, 0x5e6066, 0xffffff, 0xe8b730], Patch),
];

pub const OUTLINE: Rgba = rgba(128, 82, 52, 0.95);
const OUTLINE_WIDTH: f32 = 2.8;
const EAR_PINK: Rgba = hex(0xf7aaaa);
const BLUSH: Rgba = rgba(255, 130, 145, 0.6);
const LINE: Rgba = hex(0x3a2a22);
const TONGUE: Rgba = hex(0xff8a9a);
const LEG_WIDTH: f32 = 10.0;
const TAIL_WIDTH: f32 = 9.0;
const COLLAR: Rgba = hex(0xe2403f);
const BELL: Rgba = hex(0xf7c531);

/// Tuỳ chọn vẽ đầu: nhắm mắt, nghiêng đầu (radian), thè lưỡi (0..1) khi liếm lông.
#[derive(Default, Clone, Copy)]
pub struct HeadOpts {
    pub asleep: bool,
    pub tilt: f32,
    pub tongue: f32,
}

/// Bút vẽ mèo: `outline = true` là lượt viền (mọi hình tô bằng màu viền + nét dày), chi tiết bên trong bị bỏ qua.
pub struct Pen<'a> {
    pub ctx: &'a mut Canvas,
    pub outline: bool,
}

impl Pen<'_> {
    pub fn paint_fill(&mut self, color: Rgba) {
        if !self.outline {
            return self.ctx.fill(color);
        }
        self.ctx.fill(OUTLINE);
        self.ctx.stroke(OUTLINE, OUTLINE_WIDTH);
    }

    pub fn paint_stroke(&mut self, color: Rgba, width: f32) {
        if self.outline {
            self.ctx.stroke(OUTLINE, width + OUTLINE_WIDTH)
        } else {
            self.ctx.stroke(color, width)
        }
    }

    pub fn fill_ellipse(&mut self, color: Rgba, x: f32, y: f32, rx: f32, ry: f32, rot: f32) {
        self.ctx.begin();
        self.ctx.oval(x, y, rx, ry, rot);
        self.paint_fill(color);
    }

    /// Chi tiết bên trong (hoa văn, bụng, mặt) không cần viền nên bỏ qua ở lượt viền.
    pub fn detail_ellipse(&mut self, color: Rgba, x: f32, y: f32, rx: f32, ry: f32, rot: f32) {
        if self.outline {
            return;
        }
        self.ctx.begin();
        self.ctx.oval(x, y, rx, ry, rot);
        self.ctx.fill(color);
    }

    pub fn tail(&mut self, color: Rgba, x: f32, y: f32, curve: [f32; 6]) {
        self.ctx.begin();
        self.ctx.move_to(x, y);
        self.ctx.bezier_to(curve);
        self.paint_stroke(color, TAIL_WIDTH);
    }

    pub fn leg_line(&mut self, color: Rgba, paw: Rgba, from: (f32, f32), to: (f32, f32)) {
        self.ctx.begin();
        self.ctx.move_to(from.0, from.1);
        self.ctx.line_to(to.0, to.1);
        self.paint_stroke(color, LEG_WIDTH);
        self.fill_ellipse(paw, to.0 + 1.0, to.1 + 1.2, 6.0, 3.8, 0.0);
    }

    pub fn walk_leg(&mut self, color: Rgba, paw: Rgba, hip: (f32, f32), phase: f32) {
        let lift = phase.cos().max(0.0) * 2.5;
        self.leg_line(color, paw, hip, (hip.0 + phase.sin() * 4.0, -5.0 - lift));
    }

    pub fn body(&mut self, c: &Coat, cx: f32, cy: f32, rx: f32, ry: f32) {
        self.fill_ellipse(c.body, cx, cy, rx, ry, 0.0);
        if self.outline {
            return;
        }
        self.ctx.save();
        self.ctx.begin();
        self.ctx.oval(cx, cy, rx, ry, 0.0);
        self.ctx.clip();
        if c.kind != Solid {
            self.detail_ellipse(c.belly, cx + 4.0, cy + ry * 0.62, rx * 0.72, ry * 0.5, 0.0);
        }
        self.coat_marks(c, cx, cy, rx, ry);
        self.ctx.restore();
    }

    fn coat_marks(&mut self, c: &Coat, cx: f32, cy: f32, rx: f32, ry: f32) {
        match c.kind {
            Tabby => {
                for i in -1..=1 {
                    let sx = cx + i as f32 * rx * 0.34;
                    self.ctx.begin();
                    self.ctx.move_to(sx - 3.0, cy - ry);
                    self.ctx.quad_to(sx + 2.0, cy - ry * 0.45, sx - 1.0, cy);
                    self.ctx.stroke(c.mark, 3.6);
                }
            }
            Calico | Patch => {
                self.detail_ellipse(c.mark, cx - rx * 0.35, cy - ry * 0.45, rx * 0.42, ry * 0.6, 0.3);
                self.detail_ellipse(c.tail, cx + rx * 0.4, cy - ry * 0.6, rx * 0.3, ry * 0.45, -0.2);
            }
            _ => {}
        }
    }

    pub fn head(&mut self, cat: &Cat, hx: f32, hy: f32, opts: HeadOpts) {
        let c = cat.coat();
        self.ctx.save();
        self.ctx.translate(hx, hy);
        self.ctx.rotate(opts.tilt);
        self.ear(c, -9.0, -8.0, -1.0, cat.ear_twitch * 2.0);
        self.ear(c, 9.0, -8.0, 1.0, 0.0);
        self.fill_ellipse(c.body, 0.0, 0.0, 17.5, 14.5, 0.0);
        self.fill_ellipse(c.body, 0.0, 4.5, 19.5, 11.0, 0.0);
        if !self.outline {
            match c.kind {
                Point => self.detail_ellipse(c.mark, 1.0, 4.0, 10.0, 7.5, 0.0),
                Tux => self.detail_ellipse(c.belly, 1.0, 7.0, 10.0, 6.5, 0.0),
                Calico | Patch => self.detail_ellipse(c.mark, -8.0, -5.0, 7.5, 6.0, 0.4),
                Tabby => self.forehead_stripes(c.mark),
                Solid => {}
            }
            self.face(cat, 1.5, 1.5, opts);
            if cat.collar {
                self.collar();
            }
        }
        self.ctx.restore();
    }

    fn collar(&mut self) {
        self.ctx.begin();
        self.ctx.arc(1.0, -2.0, 17.0, 0.55, PI - 0.55);
        self.ctx.stroke(COLLAR, 3.4);
        self.detail_ellipse(OUTLINE, 3.0, 16.5, 3.8, 3.8, 0.0);
        self.detail_ellipse(BELL, 3.0, 16.5, 3.0, 3.0, 0.0);
    }

    fn ear(&mut self, c: &Coat, x: f32, y: f32, side: f32, tilt: f32) {
        let (tip_x, tip_y) = (x + side * 3.5 + tilt, y - 11.0);
        self.ctx.begin();
        self.ctx.move_to(x - 7.0, y + 4.0);
        self.ctx.quad_to(tip_x - side, tip_y - 2.0, tip_x, tip_y);
        self.ctx.line_to(x + 7.0, y + 4.0);
        self.ctx.close();
        self.paint_fill(if c.kind == Point { c.mark } else { c.body });
        if self.outline {
            return;
        }
        self.ctx.begin();
        self.ctx.move_to(x - 3.5, y + 2.0);
        self.ctx.line_to(tip_x, tip_y + 5.0);
        self.ctx.line_to(x + 3.5, y + 2.0);
        self.ctx.close();
        self.ctx.fill(EAR_PINK);
    }

    fn forehead_stripes(&mut self, color: Rgba) {
        for dx in [-4.0, 0.0, 4.0] {
            self.ctx.begin();
            self.ctx.move_to(dx, -14.0);
            self.ctx.line_to(dx * 0.8, -8.5);
            self.ctx.stroke(color, 2.2);
        }
    }

    fn face(&mut self, cat: &Cat, fx: f32, fy: f32, opts: HeadOpts) {
        for ex in [fx - 7.0, fx + 7.0] {
            self.eye(cat, ex, fy, opts.asleep);
        }
        self.detail_ellipse(BLUSH, fx - 11.0, fy + 5.0, 4.0, 2.5, 0.0);
        self.detail_ellipse(BLUSH, fx + 11.0, fy + 5.0, 4.0, 2.5, 0.0);
        if opts.tongue > 0.0 {
            self.detail_ellipse(TONGUE, fx, fy + 7.5, 2.4, 2.6 * opts.tongue, 0.0);
        }
        let ctx = &mut *self.ctx;
        ctx.begin();
        ctx.move_to(fx - 2.2, fy + 3.0);
        ctx.line_to(fx + 2.2, fy + 3.0);
        ctx.line_to(fx, fy + 5.0);
        ctx.close();
        ctx.fill(hex(0xe8868c));
        ctx.begin();
        ctx.arc(fx - 1.8, fy + 5.2, 1.8, 0.0, PI);
        ctx.arc(fx + 1.8, fy + 5.2, 1.8, 0.0, PI);
        ctx.stroke(LINE, 1.1);
        for side in [-1.0, 1.0] {
            for dy in [-1.0, 1.5] {
                ctx.begin();
                ctx.move_to(fx + side * 10.0, fy + 4.0 + dy * 0.5);
                ctx.line_to(fx + side * 21.0, fy + 3.0 + dy * 1.7);
                ctx.stroke(rgba(255, 255, 255, 0.85), 0.9);
            }
        }
    }

    fn eye(&mut self, cat: &Cat, ex: f32, fy: f32, asleep: bool) {
        if asleep || cat.blink > 0.0 {
            self.ctx.begin();
            self.ctx.arc(ex, fy - 2.0, 3.2, 0.2, PI - 0.2);
            return self.ctx.stroke(LINE, 1.7);
        }
        if cat.happy || cat.fun.joy > 0.0 {
            // Mắt cười híp "^ ^" như tranh mẫu.
            self.ctx.begin();
            self.ctx.arc(ex, fy, 3.4, PI + 0.35, TAU - 0.35);
            return self.ctx.stroke(LINE, 2.0);
        }
        self.detail_ellipse(hex(0x1b1514), ex, fy - 1.5, 3.9, 4.6, 0.0);
        self.detail_ellipse(cat.coat().eye, ex, fy - 0.3, 2.9, 3.0, 0.0);
        self.detail_ellipse(hex(0x1b1514), ex, fy - 0.8, 1.8, 2.7, 0.0);
        self.detail_ellipse(hex(0xffffff), ex + 1.4, fy - 3.4, 1.5, 1.5, 0.0);
        self.detail_ellipse(hex(0xffffff), ex - 1.2, fy + 0.9, 0.7, 0.7, 0.0);
    }
}

pub fn leg_color(c: &Coat) -> Rgba {
    if c.kind == Point {
        c.shade
    } else {
        c.body
    }
}
