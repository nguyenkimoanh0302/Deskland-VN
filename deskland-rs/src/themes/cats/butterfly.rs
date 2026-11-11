//! Bướm vàng bay lượn trên đồi (tách từ phần bướm của prototype/cats-wind.js): bay tới một điểm rồi đổi hướng,
//! giật mình bay đi khi cuộn len rơi gần, bay tới chỗ bấm khi bấm lên trời.

use super::canvas::{hex, Canvas, Rgba};

const WING: Rgba = hex(0xffd23a);
const EDGE: Rgba = hex(0xe09a12);
const SPEED: f32 = 34.0;

pub struct Butterfly {
    pub x: f32,
    pub y: f32,
    pub target: (f32, f32),
    pub flap: f32,
    pub retarget: f32,
    dir: f32,
}

impl Butterfly {
    pub fn new(x: f32, y: f32, target: (f32, f32), flap: f32, retarget: f32) -> Self {
        Butterfly { x, y, target, flap, retarget, dir: 1.0 }
    }

    /// Bay một bước; hết giờ hoặc tới đích thì gọi `pick` lấy (đích mới, số giây trước khi đổi hướng).
    pub fn step(&mut self, dt: f32, wind: f32, pick: impl FnOnce() -> ((f32, f32), f32)) {
        self.flap += dt * 16.0;
        self.retarget -= dt;
        let (dx, dy) = (self.target.0 - self.x, self.target.1 - 30.0 - self.y);
        let dist = dx.hypot(dy);
        if self.retarget <= 0.0 || dist < 12.0 {
            (self.target, self.retarget) = pick();
        }
        let d = if dist > 0.0 { dist } else { 1.0 };
        self.x += dx / d * SPEED * dt + wind * 6.0 * dt;
        self.y += dy / d * SPEED * dt + (self.flap * 0.25).sin() * 18.0 * dt;
        self.dir = if dx >= 0.0 { 1.0 } else { -1.0 };
    }

    pub fn draw(&self, ctx: &mut Canvas) {
        let open = 0.25 + 0.75 * self.flap.sin().abs();
        ctx.save();
        ctx.translate(self.x, self.y);
        ctx.scale(self.dir, 1.0);
        for side in [-1.0, 1.0] {
            ctx.save();
            ctx.scale(side * open, 1.0);
            for (color, k) in [(EDGE, 0.0), (WING, 1.0)] {
                ctx.begin();
                ctx.oval(5.5, -4.0, 6.5 - 1.5 * k, 5.5 - 1.3 * k, -0.5);
                ctx.oval(4.5, 4.0, 4.5 - 1.1 * k, 3.8 - 1.0 * k, 0.5);
                ctx.fill(color);
            }
            ctx.restore();
        }
        ctx.begin();
        ctx.move_to(0.0, -6.0);
        ctx.line_to(0.0, 6.0);
        ctx.stroke(hex(0x4a3020), 1.8);
        ctx.restore();
    }
}
