//! Chủ đề Mèo con trên đồi (port từ prototype/cats.html + cats-*.js). Khác các chủ đề shader khác: cảnh được vẽ
//! bằng tiny-skia trên CPU giống hệt Canvas 2D của bản web (nền tĩnh vẽ một lần vào bộ đệm, mỗi khung chỉ vẽ cây,
//! mèo, lá, bướm), rồi đưa lên texture để shader `cats.frag` chép ra framebuffer. Tương tác chuột: interact.rs.

mod behavior;
mod butterfly;
mod canvas;
mod draw;
mod interact;
mod meadow;
mod nature;
mod poses;
mod scenery;
mod wind;

use crate::engine::*;
use behavior::{Cat, Tune, World};
use canvas::Canvas;
use glow::HasContext;
use nature::{Item, Obstacle};
use tiny_skia::Pixmap;

pub const FRAG: &str = include_str!("../../../assets/shaders/cats.frag");
const MAX_CATS: usize = 14;
const CAT_TOP_BELOW_HORIZON: f32 = 70.0;

pub struct Params {
    pub mound: f32,
    pub cats: f32,
    pub speed: f32,
    pub naps: f32,
    pub sun: f32,
    pub flowers: f32,
    pub bushes: f32,
    pub wind: f32,
}

impl Params {
    /// Mặc định giống `P` trong prototype/cats.html và prototype/themes.json.
    pub fn from_json(v: Option<&serde_json::Value>) -> Self {
        let g = |k: &str, d: f32| v.and_then(|o| o.get(k)).and_then(|x| x.as_f64()).map(|x| x as f32).unwrap_or(d);
        Params {
            mound: g("mound", 28.0),
            cats: g("cats", 8.0),
            speed: g("speed", 1.0),
            naps: g("naps", 0.35),
            sun: g("sun", 0.6),
            flowers: g("flowers", 0.6),
            bushes: g("bushes", 4.0),
            wind: g("wind", 1.0),
        }
    }
}

pub struct Cats {
    w: f32,
    h: f32,
    scale: f32,
    seed: i32,
    p: Params,
    rng: Rng,
    icons: Vec<IconBox>,
    items: Vec<Item>,
    cats: Vec<Cat>,
    wind: wind::Wind,
    interact: interact::Interact,
    frame: Canvas,
    /// Nền tĩnh đã vẽ sẵn; None = cần vẽ lại (biểu tượng đổi chỗ)
    backdrop: Option<Pixmap>,
    tex: glow::Texture,
}

impl Cats {
    #[allow(clippy::too_many_arguments)]
    pub fn new(gl: &glow::Context, w: f32, h: f32, buf_w: i32, buf_h: i32, scale: f32, seed: i32, p: Params, rng_seed: u64, _now: f64) -> Self {
        let tex = unsafe {
            let t = gl.create_texture().expect("cats texture");
            gl.bind_texture(glow::TEXTURE_2D, Some(t));
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, buf_w, buf_h, 0, glow::RGBA, glow::UNSIGNED_BYTE, None);
            set_tex_params(gl, glow::NEAREST, glow::CLAMP_TO_EDGE);
            t
        };
        let frame = Canvas::new(buf_w as u32, buf_h as u32).expect("cats canvas");
        let mut s = Cats {
            w,
            h,
            scale,
            seed,
            p,
            rng: Rng::new(rng_seed),
            icons: Vec::new(),
            items: Vec::new(),
            cats: Vec::new(),
            wind: wind::Wind::new(rng_seed ^ 0x77),
            interact: interact::Interact::default(),
            frame,
            backdrop: None,
            tex,
        };
        s.commit();
        s.sync_cat_count();
        s
    }

    fn obstacles(&self) -> Vec<Obstacle> {
        self.icons.iter().map(Obstacle::from).chain(self.items.iter().filter_map(|i| i.footprint)).collect()
    }

    fn world<'a>(&self, obstacles: &'a [Obstacle], others: &'a [(f32, f32)], me: usize) -> World<'a> {
        let floor = self.h - TASKBAR - behavior::MARGIN_BOTTOM;
        World { w: self.w, top: meadow::horizon(self.h) + CAT_TOP_BELOW_HORIZON, floor, obstacles, others, me }
    }

    fn tune(&self) -> Tune {
        Tune { speed: self.p.speed, naps: self.p.naps }
    }

    /// Thứ tự bộ lông theo seed, giống coatOrder() của bản web.
    fn coat_order(&self) -> Vec<usize> {
        let mut rand = meadow::SeededRandom::new(self.seed);
        let mut order: Vec<usize> = (0..draw::COATS.len()).collect();
        for i in (1..order.len()).rev() {
            let j = ((rand.next() * (i + 1) as f32) as usize).min(i);
            order.swap(i, j);
        }
        order
    }

    fn sync_cat_count(&mut self) {
        let want = (self.p.cats.round().max(0.0) as usize).min(MAX_CATS);
        let coats = self.coat_order();
        self.cats.truncate(want);
        while self.cats.len() < want {
            let obstacles = self.obstacles();
            let others: Vec<(f32, f32)> = self.cats.iter().map(|c| (c.x, c.y)).collect();
            let world = self.world(&obstacles, &others, usize::MAX);
            let cat = behavior::create_cat(&world, coats[self.cats.len() % coats.len()], &mut self.rng);
            self.cats.push(cat);
        }
    }

    /// Bố cục đổi (biểu tượng di chuyển): đặt lại cây/đá, vẽ lại nền, mèo bị che thì tìm chỗ mới.
    fn commit(&mut self) {
        self.items = nature::place_nature(&nature::Layout {
            w: self.w,
            h: self.h,
            seed: self.seed,
            icons: &self.icons,
            taskbar: TASKBAR,
            count: self.p.bushes,
            mound: self.p.mound,
        });
        self.backdrop = None;
        let obstacles = self.obstacles();
        let others: Vec<(f32, f32)> = self.cats.iter().map(|c| (c.x, c.y)).collect();
        let tune = self.tune();
        for i in 0..self.cats.len() {
            let world = self.world(&obstacles, &others, i);
            behavior::adapt_to_icons(&mut self.cats[i], &world, tune, &mut self.rng);
        }
    }

    fn step(&mut self, dt: f32) {
        let scene = wind::Scene { w: self.w, h: self.h, taskbar: TASKBAR, strength: self.p.wind, items: &self.items };
        self.wind.step(dt, &scene);
        nature::step_shake(&mut self.items, dt);
        let obstacles = self.obstacles();
        let tune = self.tune();
        for i in 0..self.cats.len() {
            let others: Vec<(f32, f32)> = self.cats.iter().map(|c| (c.x, c.y)).collect();
            let world = self.world(&obstacles, &others, i);
            behavior::step_cat(&mut self.cats[i], dt, &world, tune, &mut self.rng);
        }
        let world = self.world(&obstacles, &[], usize::MAX);
        self.interact.step(dt, &mut self.cats, &world, tune, &mut self.rng);
    }

    fn paint_backdrop(&self) -> Option<Pixmap> {
        let mut c = Canvas::new(self.frame.width(), self.frame.height())?;
        c.set_transform(self.scale);
        let o = meadow::Backdrop {
            w: self.w,
            h: self.h,
            taskbar: TASKBAR,
            seed: self.seed,
            icons: &self.icons,
            mound: self.p.mound,
            sun: self.p.sun,
            flowers: self.p.flowers,
        };
        meadow::paint_meadow(&mut c, &o);
        Some(c.pm)
    }

    /// Vẽ một khung lên canvas CPU (giống draw() của bản web).
    fn paint(&mut self) {
        if self.backdrop.is_none() {
            self.backdrop = self.paint_backdrop();
        }
        let ctx = &mut self.frame;
        if let Some(bg) = &self.backdrop {
            ctx.copy_from(bg);
        }
        ctx.set_transform(self.scale);
        self.wind.draw_landed(ctx);
        // Vẽ cây/đá và mèo theo thứ tự chân từ trên xuống để mèo có thể đi khuất sau tán lá.
        let gust = self.wind.wind_of(self.p.wind);
        let mut order: Vec<(f32, Option<usize>, Option<usize>)> = self.items.iter().enumerate().map(|(i, it)| (it.y, Some(i), None)).collect();
        order.extend(self.cats.iter().enumerate().map(|(i, c)| (c.y, None, Some(i))));
        // Cuộn len (nếu có) xếp lớp cùng mèo: mục không có item lẫn cat là cuộn len.
        if let Some(y) = self.interact.yarn_y() {
            order.push((y, None, None));
        }
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, item, cat) in order {
            match (item, cat) {
                (Some(i), _) => nature::draw_item(ctx, &self.items[i], gust, self.wind.time),
                (_, Some(i)) => poses::draw_cat(ctx, &self.cats[i]),
                _ => self.interact.draw_yarn(ctx),
            }
        }
        self.wind.draw_air(ctx, self.p.wind);
        self.interact.draw_rings(ctx);
    }

    fn present(&mut self, gl: &glow::Context, sh: &mut Shared) {
        self.paint();
        let (bw, bh) = (self.frame.width() as i32, self.frame.height() as i32);
        let p = &mut sh.prog;
        unsafe {
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                bw,
                bh,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(self.frame.pm.data()),
            );
            gl.use_program(Some(p.prog));
        }
        p.i1(gl, "uTex", 0);
        p.f2(gl, "uBuf", bw as f32, bh as f32);
        unsafe { gl.draw_arrays(glow::TRIANGLES, 0, 3) };
    }
}

impl Theme for Cats {
    fn set_icons(&mut self, _gl: &glow::Context, icons: Vec<IconBox>, _first: bool, _now: f64) {
        self.icons = icons;
        self.commit();
    }

    fn frame(&mut self, gl: &glow::Context, sh: &mut Shared, dt: f32, _now: f64) {
        self.step(dt);
        self.present(gl, sh);
    }

    fn settle(&mut self, gl: &glow::Context, sh: &mut Shared, _now: f64) {
        self.present(gl, sh);
    }

    /// Bấm vào mèo: vuốt ve; vào cây: rung lá; xuống cỏ: thả cuộn len; lên trời: gọi bướm.
    fn handle_click(&mut self, _gl: &glow::Context, x: f32, y: f32, _now: f64) {
        let obstacles = self.obstacles();
        let world = self.world(&obstacles, &[], usize::MAX);
        let tune = self.tune();
        let scene = interact::Scene { h: self.h, cats: &mut self.cats, items: &mut self.items, wind: &mut self.wind, world, tune };
        self.interact.click(x, y, scene, &mut self.rng);
    }

    /// Đang có hiệu ứng tương tác thì vòng lặp chính chạy 50 khung/giây cho mượt.
    fn is_interacting(&self, _now: f64) -> bool {
        self.interact.active(&self.cats, &self.items)
    }

    fn destroy(&mut self, gl: &glow::Context) {
        unsafe { gl.delete_texture(self.tex) };
    }

    fn field_ms(&self) -> f64 {
        0.0
    }
}
