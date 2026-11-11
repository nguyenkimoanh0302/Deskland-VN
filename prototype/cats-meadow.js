'use strict';
// Nền tĩnh của chủ đề Mèo con: trời và phong cảnh xa (cats-scenery.js), đồi cỏ xanh tươi, ngọn cỏ, hoa,
// gò cỏ sáng quanh biểu tượng và khung viền. Vẽ một lần vào canvas đệm, chỉ vẽ lại khi bố cục/tham số đổi.

const MEADOW_HORIZON = 0.24;           // tỉ lệ chiều cao dành cho trời và núi xa
const MEADOW_GRASS = ['#c2e852', '#a3d23f', '#86bb33'];
const MEADOW_BLADE_LIGHT = 'rgba(225, 252, 150, .6)';
const MEADOW_BLADE_DARK = 'rgba(85, 145, 30, .4)';
const MEADOW_MOUND = 'rgba(214, 245, 120, .9)';
const MEADOW_SUN = '255, 240, 170';
const MEADOW_FLOWERS = [
  { petal: '#ffffff', center: '#f6c430' },
  { petal: '#fff27a', center: '#f0a020' },
  { petal: '#ffffff', center: '#f6c430' },
  { petal: '#ffd6e0', center: '#f7a8b8' },
];
const MEADOW_FLOWER_CLUSTER_AREA = 22000;   // px² cho mỗi khóm hoa ở mật độ 100%
const MEADOW_BLADE_AREA = 1500;

function meadowHorizon(H) { return H * MEADOW_HORIZON; }

function catMakeRandom(seed) {
  let a = seed | 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function createCatMeadow() {
  const canvas = document.createElement('canvas');
  let key = '';
  return {
    canvas,
    update(o) {
      const next = [o.pixelW, o.pixelH, o.W, o.H, o.seed, o.mound, o.sun, o.flowers, o.version].join('|');
      if (next === key) return;
      key = next;
      paintMeadow(canvas, o);
    },
  };
}

function paintMeadow(canvas, o) {
  canvas.width = o.pixelW; canvas.height = o.pixelH;
  const ctx = canvas.getContext('2d');
  ctx.setTransform(o.scale, 0, 0, o.scale, 0, 0);
  const hz = meadowHorizon(o.H), rand = catMakeRandom(o.seed ^ 0x5eed);
  paintSky(ctx, o.W, hz, rand);
  paintDistantLand(ctx, o.W, hz, rand);
  paintHill(ctx, o.W, o.H, hz);
  paintBlades(ctx, o.W, o.H, hz, rand);
  paintSunGlow(ctx, o.W, o.H, o.sun);
  paintMounds(ctx, o.icons, o.mound);
  paintFlowers(ctx, o, hz, catMakeRandom(o.seed ^ 0xf10e));
  paintFrame(ctx, o, catMakeRandom(o.seed ^ 0xf4a3));
}

// Đồi cỏ chính hình vòm: giữa cao hơn hai bên, sáng ở giữa và đậm dần ra mép.
function paintHill(ctx, W, H, hz) {
  ctx.beginPath(); ctx.moveTo(0, H);
  for (let x = 0; x <= W + 20; x += 20) ctx.lineTo(x, hz + 62 - Math.sin(Math.PI * Math.min(x, W) / W) * 42);
  ctx.lineTo(W, H); ctx.closePath();
  const grass = ctx.createRadialGradient(W / 2, H * 0.6, 0, W / 2, H * 0.6, Math.max(W, H) * 0.75);
  MEADOW_GRASS.forEach((c, i) => grass.addColorStop(i / (MEADOW_GRASS.length - 1), c));
  ctx.fillStyle = grass; ctx.fill();
}

function paintBlades(ctx, W, H, hz, rand) {
  ctx.lineWidth = 1.5; ctx.lineCap = 'round';
  const count = Math.round(W * (H - hz) / MEADOW_BLADE_AREA);
  for (let i = 0; i < count; i++) {
    const x = rand() * W, y = hz + 40 + rand() * (H - hz), h = 4 + rand() * 4;
    ctx.strokeStyle = rand() < 0.55 ? MEADOW_BLADE_LIGHT : MEADOW_BLADE_DARK;
    ctx.beginPath(); ctx.moveTo(x - 2.5, y - h * 0.8); ctx.lineTo(x, y); ctx.lineTo(x + 2.5, y - h); ctx.stroke();
  }
}

function paintSunGlow(ctx, W, H, strength) {
  if (strength <= 0) return;
  const r = Math.max(W, H) * 0.7;
  const glow = ctx.createRadialGradient(W * 0.9, 0, 0, W * 0.9, 0, r);
  glow.addColorStop(0, `rgba(${MEADOW_SUN}, ${0.5 * strength})`);
  glow.addColorStop(1, `rgba(${MEADOW_SUN}, 0)`);
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, W, H);
}

// Gò cỏ sáng mềm quanh biểu tượng (viền nhoè, không có đường kẻ) để biểu tượng nổi trên nền.
function paintMounds(ctx, icons, pad) {
  if (!icons.length) return;
  ctx.save();
  ctx.shadowColor = MEADOW_MOUND; ctx.shadowBlur = 22;
  ctx.fillStyle = MEADOW_MOUND;
  ctx.beginPath();
  for (const ic of icons) {
    ctx.roundRect(ic.x - pad, ic.y - pad * 0.6, ic.w + 2 * pad, ic.h + 1.2 * pad, pad + 18);
  }
  ctx.fill();
  ctx.restore();
}

function paintFlowers(ctx, o, hz, rand) {
  const clusters = Math.round(o.flowers * o.W * (o.H - hz) / MEADOW_FLOWER_CLUSTER_AREA);
  for (let i = 0; i < clusters; i++) {
    const cx = rand() * o.W, cy = hz + 50 + rand() * (o.H - o.taskbar - hz - 50);
    const colors = MEADOW_FLOWERS[Math.floor(rand() * MEADOW_FLOWERS.length)];
    for (let f = 0, n = 2 + Math.floor(rand() * 4); f < n; f++) {
      const x = cx + (rand() - 0.5) * 30, y = cy + (rand() - 0.5) * 16;
      if (o.icons.some(ic => x > ic.x - 6 && x < ic.x + ic.w + 6 && y > ic.y - 6 && y < ic.y + ic.h + 6)) continue;
      paintFlower(ctx, x, y, 3 + rand() * 1.8, colors);
    }
  }
}

function paintFlower(ctx, x, y, r, colors) {
  ctx.fillStyle = 'rgba(70, 130, 30, .35)';
  ctx.beginPath(); ctx.ellipse(x + 1, y + r * 1.4, r * 1.3, r * 0.45, 0, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = colors.petal;
  for (let i = 0; i < 5; i++) {
    const a = i / 5 * Math.PI * 2;
    ctx.beginPath(); ctx.arc(x + Math.cos(a) * r, y + Math.sin(a) * r, r * 0.78, 0, Math.PI * 2); ctx.fill();
  }
  ctx.fillStyle = colors.center;
  ctx.beginPath(); ctx.arc(x, y, r * 0.62, 0, Math.PI * 2); ctx.fill();
}
