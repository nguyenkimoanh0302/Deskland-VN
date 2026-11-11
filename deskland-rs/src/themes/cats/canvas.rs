//! Lớp mỏng mô phỏng Canvas 2D trên tiny-skia, để mã vẽ của prototype/cats-*.js được port gần như nguyên văn
//! (beginPath / arc / ellipse / fill / stroke / save / restore / clip / globalAlpha).

use std::f32::consts::{FRAC_PI_2, TAU};
use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap, Point, RadialGradient, Shader, SpreadMode, Stroke, Transform,
};

/// Màu kiểu CSS: r, g, b (0..255) và độ trong suốt a (0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub f32);

pub const fn hex(v: u32) -> Rgba {
    Rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, 1.0)
}

pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Rgba {
    Rgba(r, g, b, a)
}

impl Rgba {
    pub fn with_alpha(self, a: f32) -> Rgba {
        Rgba(self.0, self.1, self.2, a)
    }
    fn to_color(self, alpha: f32) -> Color {
        Color::from_rgba8(self.0, self.1, self.2, ((self.3 * alpha).clamp(0.0, 1.0) * 255.0).round() as u8)
    }
}

#[derive(Clone, Copy)]
enum Seg {
    Move(f32, f32),
    Line(f32, f32),
    Quad(f32, f32, f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

struct State {
    ts: Transform,
    alpha: f32,
    clip: Option<Mask>,
}

pub struct Canvas {
    pub pm: Pixmap,
    ts: Transform,
    alpha: f32,
    clip: Option<Mask>,
    stack: Vec<State>,
    segs: Vec<Seg>,
    /// Điểm đầu của đường con hiện tại (để closePath / arc nối tiếp đúng như Canvas)
    start: Option<(f32, f32)>,
}

impl Canvas {
    pub fn new(w: u32, h: u32) -> Option<Self> {
        Some(Canvas {
            pm: Pixmap::new(w.max(1), h.max(1))?,
            ts: Transform::identity(),
            alpha: 1.0,
            clip: None,
            stack: Vec::new(),
            segs: Vec::new(),
            start: None,
        })
    }

    pub fn width(&self) -> u32 {
        self.pm.width()
    }
    pub fn height(&self) -> u32 {
        self.pm.height()
    }

    // ------------------------------------------------------------ trạng thái
    pub fn save(&mut self) {
        self.stack.push(State { ts: self.ts, alpha: self.alpha, clip: self.clip.clone() });
    }
    pub fn restore(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.ts = s.ts;
            self.alpha = s.alpha;
            self.clip = s.clip;
        }
    }
    pub fn set_transform(&mut self, s: f32) {
        self.ts = Transform::from_scale(s, s);
    }
    pub fn translate(&mut self, x: f32, y: f32) {
        self.ts = self.ts.pre_translate(x, y);
    }
    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.ts = self.ts.pre_scale(sx, sy);
    }
    pub fn rotate(&mut self, rad: f32) {
        self.ts = self.ts.pre_rotate(rad.to_degrees());
    }
    pub fn set_alpha(&mut self, a: f32) {
        self.alpha = a.clamp(0.0, 1.0);
    }

    // ------------------------------------------------------------ đường vẽ
    pub fn begin(&mut self) {
        self.segs.clear();
        self.start = None;
    }
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.segs.push(Seg::Move(x, y));
        self.start = Some((x, y));
    }
    pub fn line_to(&mut self, x: f32, y: f32) {
        if self.start.is_none() {
            return self.move_to(x, y);
        }
        self.segs.push(Seg::Line(x, y));
    }
    pub fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        if self.start.is_none() {
            self.move_to(cx, cy);
        }
        self.segs.push(Seg::Quad(cx, cy, x, y));
    }
    pub fn bezier_to(&mut self, c: [f32; 6]) {
        if self.start.is_none() {
            self.move_to(c[0], c[1]);
        }
        self.segs.push(Seg::Cubic(c[0], c[1], c[2], c[3], c[4], c[5]));
    }
    pub fn close(&mut self) {
        if self.start.is_some() {
            self.segs.push(Seg::Close);
        }
    }

    pub fn arc(&mut self, x: f32, y: f32, r: f32, a0: f32, a1: f32) {
        self.ellipse(x, y, r, r, 0.0, a0, a1);
    }
    pub fn circle(&mut self, x: f32, y: f32, r: f32) {
        self.ellipse(x, y, r, r, 0.0, 0.0, TAU);
    }
    pub fn oval(&mut self, x: f32, y: f32, rx: f32, ry: f32, rot: f32) {
        self.ellipse(x, y, rx, ry, rot, 0.0, TAU);
    }

    /// ctx.ellipse(x, y, rx, ry, rotation, a0, a1) theo chiều kim đồng hồ, xấp xỉ bằng các đoạn cubic ≤ 90°.
    /// Giữ đúng 7 tham số như Canvas 2D để mã port đọc giống bản web.
    #[allow(clippy::too_many_arguments)]
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, a0: f32, a1: f32) {
        let (rx, ry) = (rx.abs(), ry.abs());
        let sweep = if a1 - a0 >= TAU { TAU } else { (a1 - a0).rem_euclid(TAU) };
        let (sr, cr) = rot.sin_cos();
        let at = |t: f32| {
            let (ex, ey) = (rx * t.cos(), ry * t.sin());
            (cx + cr * ex - sr * ey, cy + sr * ex + cr * ey)
        };
        let tangent = |t: f32| {
            let (dx, dy) = (-rx * t.sin(), ry * t.cos());
            (cr * dx - sr * dy, sr * dx + cr * dy)
        };
        let p0 = at(a0);
        if self.start.is_none() {
            self.move_to(p0.0, p0.1)
        } else {
            self.segs.push(Seg::Line(p0.0, p0.1))
        }
        let n = (sweep / FRAC_PI_2).ceil().max(1.0) as usize;
        let step = sweep / n as f32;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        for i in 0..n {
            let (t0, t1) = (a0 + step * i as f32, a0 + step * (i + 1) as f32);
            let (p, q) = (at(t0), at(t1));
            let (d0, d1) = (tangent(t0), tangent(t1));
            self.segs.push(Seg::Cubic(p.0 + k * d0.0, p.1 + k * d0.1, q.0 - k * d1.0, q.1 - k * d1.1, q.0, q.1));
        }
    }

    /// ctx.roundRect(x, y, w, h, r): bán kính tự thu nhỏ khi khung quá hẹp như Canvas.
    pub fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32) {
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        self.start = None;
        self.move_to(x + r, y);
        self.line_to(x + w - r, y);
        self.ellipse(x + w - r, y + r, r, r, 0.0, -FRAC_PI_2, 0.0);
        self.line_to(x + w, y + h - r);
        self.ellipse(x + w - r, y + h - r, r, r, 0.0, 0.0, FRAC_PI_2);
        self.line_to(x + r, y + h);
        self.ellipse(x + r, y + h - r, r, r, 0.0, FRAC_PI_2, FRAC_PI_2 * 2.0);
        self.line_to(x, y + r);
        self.ellipse(x + r, y + r, r, r, 0.0, FRAC_PI_2 * 2.0, FRAC_PI_2 * 3.0);
        self.close();
        self.start = None;
    }

    fn path(&self) -> Option<tiny_skia::Path> {
        let mut pb = PathBuilder::new();
        for s in &self.segs {
            match *s {
                Seg::Move(x, y) => pb.move_to(x, y),
                Seg::Line(x, y) => pb.line_to(x, y),
                Seg::Quad(a, b, x, y) => pb.quad_to(a, b, x, y),
                Seg::Cubic(a, b, c, d, x, y) => pb.cubic_to(a, b, c, d, x, y),
                Seg::Close => pb.close(),
            }
        }
        pb.finish()
    }

    // ------------------------------------------------------------ tô / viền
    pub fn fill(&mut self, color: Rgba) {
        let mut paint = Paint::default();
        paint.set_color(color.to_color(self.alpha));
        self.fill_paint(&paint);
    }

    pub fn fill_paint(&mut self, paint: &Paint) {
        if let Some(path) = self.path() {
            self.pm.fill_path(&path, paint, FillRule::Winding, self.ts, self.clip.as_ref());
        }
    }

    pub fn stroke(&mut self, color: Rgba, width: f32) {
        let Some(path) = self.path() else { return };
        let mut paint = Paint::default();
        paint.set_color(color.to_color(self.alpha));
        let stroke = Stroke { width, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
        self.pm.stroke_path(&path, &paint, &stroke, self.ts, self.clip.as_ref());
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, paint: &Paint) {
        self.begin();
        self.move_to(x, y);
        self.line_to(x + w, y);
        self.line_to(x + w, y + h);
        self.line_to(x, y + h);
        self.close();
        self.fill_paint(paint);
    }

    /// ctx.clip(): giao vùng cắt hiện tại với đường đang vẽ.
    pub fn clip(&mut self) {
        let Some(path) = self.path() else { return };
        match self.clip.as_mut() {
            Some(m) => m.intersect_path(&path, FillRule::Winding, true, self.ts),
            None => {
                if let Some(mut m) = Mask::new(self.pm.width(), self.pm.height()) {
                    m.fill_path(&path, FillRule::Winding, true, self.ts);
                    self.clip = Some(m);
                }
            }
        }
    }

    /// Tô kín canvas một màu (như màu nền CSS của trang web nằm dưới canvas).
    pub fn clear(&mut self, color: Rgba) {
        self.pm.fill(color.to_color(1.0));
    }

    /// Chép nguyên khung nền đã vẽ sẵn (cùng kích thước) lên canvas.
    pub fn copy_from(&mut self, src: &Pixmap) {
        if src.width() == self.pm.width() && src.height() == self.pm.height() {
            self.pm.data_mut().copy_from_slice(src.data());
        }
    }
}

/// Paint tô màu chuyển sắc tròn (createRadialGradient với tâm trùng nhau).
pub fn radial_paint(cx: f32, cy: f32, r: f32, stops: &[(f32, Rgba)]) -> Paint<'static> {
    let stops = stops.iter().map(|&(p, c)| GradientStop::new(p, c.to_color(1.0))).collect();
    let shader = RadialGradient::new(Point::from_xy(cx, cy), Point::from_xy(cx, cy), r.max(0.01), stops, SpreadMode::Pad, Transform::identity());
    paint_of(shader)
}

/// Paint tô màu chuyển sắc theo chiều dọc (createLinearGradient(0, y0, 0, y1)).
pub fn vertical_paint(y0: f32, y1: f32, stops: &[(f32, Rgba)]) -> Paint<'static> {
    let stops = stops.iter().map(|&(p, c)| GradientStop::new(p, c.to_color(1.0))).collect();
    let shader = tiny_skia::LinearGradient::new(Point::from_xy(0.0, y0), Point::from_xy(0.0, y1), stops, SpreadMode::Pad, Transform::identity());
    paint_of(shader)
}

fn paint_of(shader: Option<Shader<'static>>) -> Paint<'static> {
    let mut paint = Paint::default();
    if let Some(s) = shader {
        paint.shader = s;
    }
    paint
}
