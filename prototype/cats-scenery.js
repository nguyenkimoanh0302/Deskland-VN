'use strict';
// Phong cảnh xa và khung viền tĩnh của chủ đề Mèo con: bầu trời có mây, núi xa,
// cùng lùm cây và mây trắng ở mép màn hình. Được cats-meadow.js vẽ vào canvas đệm.

const SCENERY_SKY = ['#36a3ee', '#8fd2f7', '#d4f0fb'];
const SCENERY_CLOUD = '#ffffff';
const SCENERY_CLOUD_SHADE = '#dcedf8';
const SCENERY_FAR_HILLS = '#86c3a2';
const SCENERY_NEAR_HILLS = '#74bd57';
const SCENERY_TINY_TREE = '#4f9a3a';
const FOLIAGE_TONES = ['#3e8e2c', '#66b63a', '#a3da4c', '#d6f27a'];
// Vị trí khung viền: [x theo W, y theo chiều cao vùng cỏ, độ lớn].
const FRAME_BUSHES = [[0, 1, 1.25], [0.9, 1.03, 1.05], [1, 0.6, 0.9]];
const FRAME_CLOUDS = [[0.03, 0.98, 1], [0.45, 1.07, 1.1], [0.985, 0.74, 0.9]];

function paintSky(ctx, W, hz, rand) {
  const sky = ctx.createLinearGradient(0, 0, 0, hz + 40);
  SCENERY_SKY.forEach((c, i) => sky.addColorStop(i / (SCENERY_SKY.length - 1), c));
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, W, hz + 40);
  const count = 3 + Math.round(W / 600);
  for (let i = 0; i < count; i++) {
    paintCloud(ctx, (i + 0.2 + rand() * 0.6) / count * W, hz * (0.2 + rand() * 0.45), 26 + rand() * 18, rand);
  }
}

function paintCloud(ctx, x, y, r, rand) {
  const puffs = [[-1.6, 0.25, 0.7], [-0.8, -0.2, 0.95], [0, -0.45, 1.15], [0.85, -0.15, 0.95], [1.6, 0.25, 0.7]];
  const fill = (color, dy, k) => {
    ctx.fillStyle = color; ctx.beginPath();
    for (const [px, py, pr] of puffs) {
      const jitter = 1 + (rand() - 0.5) * 0.15;
      ctx.moveTo(x + px * r + pr * r * k * jitter, y + py * r + dy);
      ctx.arc(x + px * r, y + py * r + dy, pr * r * k * jitter, 0, Math.PI * 2);
    }
    ctx.fill();
  };
  fill(SCENERY_CLOUD_SHADE, r * 0.18, 1);
  fill(SCENERY_CLOUD, 0, 0.95);
}

function paintDistantLand(ctx, W, hz, rand) {
  paintRidge(ctx, W, hz + 20, hz * 0.45, 5 + rand() * 3, SCENERY_FAR_HILLS, rand);
  paintRidge(ctx, W, hz + 52, hz * 0.22, 7 + rand() * 4, SCENERY_NEAR_HILLS, rand);
  ctx.fillStyle = SCENERY_TINY_TREE;
  for (let i = 0; i < W / 70; i++) {
    const x = rand() * W, y = hz + 18 + rand() * 30, r = 4 + rand() * 4;
    ctx.beginPath(); ctx.arc(x, y, r, 0, Math.PI * 2); ctx.arc(x + r * 0.9, y + 1, r * 0.8, 0, Math.PI * 2); ctx.fill();
  }
}

function paintRidge(ctx, W, base, height, bumps, color, rand) {
  ctx.fillStyle = color;
  ctx.beginPath(); ctx.moveTo(0, base + 60);
  const step = W / bumps;
  for (let i = 0; i <= bumps; i++) {
    const peak = base - height * (0.5 + rand() * 0.5);
    ctx.quadraticCurveTo(i * step - step / 2, peak, i * step, base - height * 0.25 * rand());
  }
  ctx.lineTo(W, base + 60); ctx.closePath(); ctx.fill();
}

function sceneryBlocked(o, x, y, r) {
  const clear = o.mound + 10;
  return o.icons.some(ic => x + r > ic.x - clear && x - r < ic.x + ic.w + clear && y + r > ic.y - clear && y - r < ic.y + ic.h + clear);
}

function paintFrame(ctx, o, rand) {
  const groundH = o.H - o.taskbar;
  for (const [fx, fy, size] of FRAME_BUSHES) {
    const x = fx * o.W, y = fy * groundH, r = 70 * size;
    if (!sceneryBlocked(o, x, y, r)) paintFoliageClump(ctx, x, y, r, rand);
  }
  for (const [fx, fy, size] of FRAME_CLOUDS) {
    const x = fx * o.W, y = fy * groundH, r = 40 * size;
    if (!sceneryBlocked(o, x, y, r * 2)) paintCloud(ctx, x, y, r, rand);
  }
}

function paintFoliageClump(ctx, x, y, r, rand) {
  const lobes = Array.from({ length: 8 }, () => {
    const a = Math.PI + rand() * Math.PI, d = r * (0.25 + rand() * 0.5);
    return [x + Math.cos(a) * d, y + Math.sin(a) * d * 0.8, r * (0.32 + rand() * 0.2)];
  });
  paintFoliageLayers(ctx, lobes);
}

// Tô một cụm tán lá theo 4 tông (tối → sáng) để có khối như tranh vẽ tay.
function paintFoliageLayers(ctx, lobes, shiftOf = () => 0) {
  const layers = [[0, 1, 0, 0], [1, 0.86, -0.1, -0.12], [2, 0.55, -0.26, -0.3], [3, 0.2, -0.42, -0.45]];
  for (const [tone, k, dx, dy] of layers) {
    ctx.fillStyle = FOLIAGE_TONES[tone]; ctx.beginPath();
    for (const lobe of lobes) {
      const [lx, ly, lr] = lobe, cx = lx + shiftOf(lobe) + dx * lr, cy = ly + dy * lr;
      ctx.moveTo(cx + lr * k, cy); ctx.arc(cx, cy, lr * k, 0, Math.PI * 2);
    }
    ctx.fill();
  }
}
