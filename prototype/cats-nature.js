'use strict';
// Vật thể trên đồi: cây lớn và lùm cây (đung đưa theo gió), đá xám, nấm đỏ.
// Chúng được vẽ xen kẽ với mèo theo vị trí chân nên mèo có thể đi khuất sau tán cây.

const TREE_SHARE = 0.5;                // tỉ lệ cây lớn trong tổng số "cây & lùm cây"
const ROCK_COUNT = 3;
const MUSHROOM_COUNT = 3;
const NATURE_FOOTPRINT_H = 18;
const TREE_TRUNK_TOP = 0.66;           // ngọn thân cây (theo chiều cao cây) nằm bên trong tán lá
const TREE_CANOPY_BASE = 0.58;         // độ cao lớp lá đáy tán, phủ lên ngọn thân cây
const TRUNK_LIGHT = '#94602f';
const TRUNK_DARK = '#6d4421';
const ROCK_TONES = ['#8f918a', '#b4b6ad', '#cfd1c8'];
const MUSHROOM_CAP = '#e5483d';
const SHAKE_DECAY = 1.2;               // cây bị bấm rung mạnh rồi dịu dần trong ~0,8 giây

function createCatNature() {
  let items = [], key = '';
  return {
    get items() { return items; },
    get obstacles() { return items.filter(i => i.footprint).map(i => i.footprint); },
    get canopies() { return items.filter(i => i.kind === 'tree' || i.kind === 'bush'); },
    layout(o) {
      const next = [o.W, o.H, o.seed, o.count, o.mound, o.version].join('|');
      if (next === key) return;
      key = next;
      items = placeNature(o);
    },
    drawItem(ctx, item, wind, time) { NATURE_PAINTERS[item.kind](ctx, item, wind, time); },
    // Tán cây / lùm cây dưới con trỏ; ưu tiên cái vẽ sau cùng (gần người xem hơn).
    itemAt(x, y) {
      return items.filter(i => i.lobes && canopyHit(i, x, y)).sort((a, b) => b.y - a.y)[0] || null;
    },
    shake(item) { item.shake = 1; },
    step(dt) { for (const i of items) if (i.shake) i.shake = Math.max(0, i.shake - dt * SHAKE_DECAY); },
    get shaking() { return items.some(i => i.shake > 0); },
  };
}

function placeNature(o) {
  const rand = catMakeRandom(o.seed ^ 0xb054), items = [];
  const trees = Math.round(o.count * TREE_SHARE), bushes = Math.round(o.count) - trees;
  const plan = [['tree', trees], ['bush', bushes], ['rock', ROCK_COUNT], ['mushroom', MUSHROOM_COUNT]];
  for (const [kind, n] of plan) for (let i = 0; i < n; i++) {
    const item = findNatureSpot(kind, o, items, rand);
    if (item) items.push(item);
  }
  return items;
}

function natureSize(kind, rand) {
  if (kind === 'tree') { const w = 150 + rand() * 60; return { w, h: w * 1.3 }; }
  if (kind === 'bush') { const w = 80 + rand() * 40; return { w, h: w * 0.62 }; }
  if (kind === 'rock') { const w = 50 + rand() * 30; return { w, h: w * 0.55 }; }
  return { w: 16, h: 18 };
}

function findNatureSpot(kind, o, items, rand) {
  const top = meadowHorizon(o.H) + 80, bottom = o.H - o.taskbar - 90, clear = o.mound + 24;
  for (let tries = 0; tries < 50; tries++) {
    const { w, h } = natureSize(kind, rand);
    const x = o.W * 0.06 + rand() * o.W * 0.88, y = top + h * 0.6 + rand() * Math.max(1, bottom - top - h * 0.6);
    const hitsIcon = o.icons.some(ic => x + w / 2 > ic.x - clear && x - w / 2 < ic.x + ic.w + clear && y > ic.y - clear && y - h < ic.y + ic.h + clear);
    const gap = kind === 'mushroom' ? 40 : (w + 90) / 2;
    if (hitsIcon || items.some(it => Math.hypot(it.x - x, it.y - y) < gap + (it.w || 0) / 2)) continue;
    return makeNatureItem(kind, x, y, w, h, rand);
  }
  return null;
}

function makeNatureItem(kind, x, y, w, h, rand) {
  const item = { kind, x, y, w, h, phase: rand() * 6 };
  if (kind === 'tree' || kind === 'bush') item.lobes = makeLobes(kind, w, h, rand);
  if (kind !== 'mushroom') {
    const fw = kind === 'tree' ? w * 0.22 : w * 0.8;
    item.footprint = { x: x - fw / 2, y: y - NATURE_FOOTPRINT_H, w: fw, h: NATURE_FOOTPRINT_H, ground: true };
  }
  return item;
}

// Tán lá: các thùy tròn đặt tương đối với gốc (0,0), kèm độ cao để tính độ đung đưa.
function makeLobes(kind, w, h, rand) {
  const n = kind === 'tree' ? 10 : 6, cy = kind === 'tree' ? -h * 0.66 : -h * 0.48;
  const spreadX = kind === 'tree' ? w * 0.4 : w * 0.36, spreadY = kind === 'tree' ? h * 0.2 : h * 0.22;
  const lobes = Array.from({ length: n }, (_, i) => {
    const t = n === 1 ? 0 : i / (n - 1) - 0.5, arch = 1 - Math.abs(t) * 2;
    return [t * spreadX * 2 + (rand() - 0.5) * w * 0.08, cy - arch * spreadY + (rand() - 0.5) * spreadY, w * (0.19 + rand() * 0.08)];
  });
  if (kind !== 'tree') return lobes;
  // Lớp lá đáy tán nằm ngay trên ngọn thân cây để tán và thân liền một khối.
  for (const dx of [-0.17, 0, 0.17]) lobes.push([dx * w, -h * TREE_CANOPY_BASE, w * (0.17 + rand() * 0.04)]);
  return lobes;
}

// Độ lệch theo gió của một điểm ở độ cao `heightRatio` (0 = gốc, 1 = ngọn); dùng chung cho thân và tán.
function swayShift(sway, heightRatio) {
  return sway * 8 * Math.min(1, heightRatio * 1.2);
}

function natureSway(item, wind, time) {
  const shake = item.shake ? Math.sin(time * 22 + item.phase) * item.shake * 1.6 : 0;
  return wind * 0.45 + Math.sin(time * 1.8 + item.phase) * 0.3 * Math.min(1.5, wind + 0.2) + shake;
}

function canopyHit(item, x, y) {
  const top = item.y - item.h * (item.kind === 'tree' ? 0.98 : 1), bottom = item.y - (item.kind === 'tree' ? item.h * 0.3 : 0);
  return Math.abs(x - item.x) < item.w * 0.5 && y > top && y < bottom;
}

function paintNatureShadow(ctx, x, y, rx, ry) {
  ctx.fillStyle = 'rgba(55, 110, 20, .28)';
  ctx.beginPath(); ctx.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2); ctx.fill();
}

function paintCanopy(ctx, item, wind, time) {
  const sway = natureSway(item, wind, time);
  const lobes = item.lobes.map(([lx, ly, lr]) => [item.x + lx, item.y + ly, lr, -ly / item.h]);
  paintFoliageLayers(ctx, lobes, lobe => swayShift(sway, lobe[3]));
}

// Thân cây cắm sâu vào trong tán (TREE_TRUNK_TOP) và có hai cành chĩa lên, nên tán luôn liền với thân.
function paintTree(ctx, t, wind, time) {
  const sway = natureSway(t, wind, time), tw = t.w * 0.09;
  const topY = t.y - t.h * TREE_TRUNK_TOP, topShift = swayShift(sway, TREE_TRUNK_TOP), midY = t.y - t.h * 0.2;
  paintNatureShadow(ctx, t.x + t.w * 0.08, t.y, t.w * 0.55, t.w * 0.13);
  paintBranches(ctx, t, sway);
  ctx.fillStyle = TRUNK_DARK;
  ctx.beginPath();
  ctx.moveTo(t.x - tw * 1.9, t.y); ctx.quadraticCurveTo(t.x - tw, t.y - 10, t.x - tw * 0.8, midY);
  ctx.lineTo(t.x - tw * 0.5 + topShift, topY); ctx.lineTo(t.x + tw * 0.5 + topShift, topY);
  ctx.lineTo(t.x + tw * 0.8, midY); ctx.quadraticCurveTo(t.x + tw, t.y - 10, t.x + tw * 1.9, t.y);
  ctx.closePath(); ctx.fill();
  ctx.fillStyle = TRUNK_LIGHT;
  ctx.beginPath();
  ctx.moveTo(t.x - tw * 1.5, t.y - 1); ctx.quadraticCurveTo(t.x - tw * 0.8, t.y - 10, t.x - tw * 0.6, midY);
  ctx.lineTo(t.x - tw * 0.35 + topShift, topY); ctx.lineTo(t.x + tw * 0.1 + topShift, topY); ctx.lineTo(t.x + tw * 0.1, t.y - 1);
  ctx.closePath(); ctx.fill();
  paintCanopy(ctx, t, wind, time);
}

function paintBranches(ctx, t, sway) {
  const forkY = t.y - t.h * 0.45, reach = t.h * 0.66;
  ctx.strokeStyle = TRUNK_DARK; ctx.lineWidth = t.w * 0.05; ctx.lineCap = 'round';
  for (const side of [-1, 1]) {
    ctx.beginPath(); ctx.moveTo(t.x, forkY);
    ctx.quadraticCurveTo(t.x + side * t.w * 0.12, forkY - t.h * 0.08, t.x + side * t.w * 0.24 + swayShift(sway, 0.66), t.y - reach);
    ctx.stroke();
  }
}

function paintBush(ctx, b, wind, time) {
  paintNatureShadow(ctx, b.x + 4, b.y, b.w * 0.55, 8);
  paintCanopy(ctx, b, wind, time);
}

function paintRock(ctx, r) {
  paintNatureShadow(ctx, r.x + 4, r.y, r.w * 0.58, r.h * 0.22);
  const [dark, mid, light] = ROCK_TONES;
  ctx.fillStyle = dark;
  ctx.beginPath(); ctx.ellipse(r.x, r.y - r.h * 0.45, r.w / 2, r.h * 0.55, 0, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = mid;
  ctx.beginPath(); ctx.ellipse(r.x - r.w * 0.06, r.y - r.h * 0.55, r.w * 0.42, r.h * 0.42, 0, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = light;
  ctx.beginPath(); ctx.ellipse(r.x - r.w * 0.16, r.y - r.h * 0.72, r.w * 0.16, r.h * 0.14, -0.3, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = FOLIAGE_TONES[2];
  for (const dx of [-0.42, 0.38]) {
    ctx.beginPath(); ctx.arc(r.x + dx * r.w, r.y - 3, 6, 0, Math.PI * 2); ctx.arc(r.x + dx * r.w + 6, r.y - 2, 5, 0, Math.PI * 2); ctx.fill();
  }
}

function paintMushroom(ctx, m) {
  ctx.fillStyle = '#f6ead2';
  ctx.fillRect(m.x - 3, m.y - 10, 6, 10);
  ctx.fillStyle = MUSHROOM_CAP;
  ctx.beginPath(); ctx.ellipse(m.x, m.y - 10, 9, 7, 0, Math.PI, 0); ctx.fill();
  ctx.fillStyle = '#ffffff';
  for (const [dx, dy] of [[-4, -13], [2, -15], [4, -11]]) { ctx.beginPath(); ctx.arc(m.x + dx, m.y + dy, 1.4, 0, Math.PI * 2); ctx.fill(); }
}

const NATURE_PAINTERS = { tree: paintTree, bush: paintBush, rock: paintRock, mushroom: paintMushroom };
