'use strict';
// Gió trên đồi: sức gió thay đổi theo từng cơn, vệt gió trắng lướt qua, lá bay/lá rụng đậu xuống cỏ rồi mờ dần,
// và vài chú bướm vàng bay lượn. Mọi thứ ở đây được cập nhật và vẽ lại mỗi khung hình.

const LEAF_COLORS = ['#f2c23a', '#f5a524', '#e8862a', '#9fd24a', '#d9e25a'];
const LEAF_MAX = 70;
const LEAF_SPAWN_PER_SECOND = 2;       // ở sức gió ×1
const LEAF_FADE_SECONDS = 5;
const STREAK_PER_SECOND = 0.7;
const STREAK_LIFE = 1.8;
const BUTTERFLY_COUNT = 3;
const BUTTERFLY_WING = '#ffd23a';
const BUTTERFLY_EDGE = '#e09a12';
const BURST_LEAVES = 10;               // lá rơi khi bấm rung cây
const SCATTER_DISTANCE = 220;          // bướm trong khoảng này giật mình bay đi khi có cuộn len rơi xuống

function createCatWind() {
  let time = 0, gust = 0, gustTarget = 0.4, gustIn = 0, leafDebt = 0, streakDebt = 0;
  let leaves = [], streaks = [], butterflies = [];
  const windOf = strength => strength * (0.5 + gust + 0.15 * Math.sin(time * 0.7));
  return {
    get time() { return time; },
    windOf,
    step(dt, o) {
      time += dt;
      gustIn -= dt;
      if (gustIn <= 0) { gustTarget = Math.random() * 1.2; gustIn = 3 + Math.random() * 5; }
      gust += (gustTarget - gust) * Math.min(1, dt * 0.8);
      const w = windOf(o.strength);
      leafDebt += dt * LEAF_SPAWN_PER_SECOND * o.strength * (0.4 + gust);
      for (; leafDebt >= 1 && leaves.length < LEAF_MAX; leafDebt--) leaves.push(spawnLeaf(o));
      leafDebt = Math.min(leafDebt, 1);
      streakDebt += dt * STREAK_PER_SECOND * o.strength * (0.3 + gust);
      for (; streakDebt >= 1; streakDebt--) streaks.push(spawnStreak(o));
      leaves = leaves.filter(l => stepLeaf(l, dt, w, o));
      streaks = streaks.filter(s => (s.age += dt) < STREAK_LIFE);
      while (butterflies.length < BUTTERFLY_COUNT) butterflies.push(spawnButterfly(o));
      for (const b of butterflies) stepButterfly(b, dt, w, o);
    },
    burst(c) { for (let i = 0; i < BURST_LEAVES; i++) leaves.push(canopyLeaf(c)); },
    scatter(x, y) {
      for (const b of butterflies) {
        const d = Math.hypot(b.x - x, b.y - y);
        if (d < SCATTER_DISTANCE) { b.target = { x: b.x + (b.x - x) / (d || 1) * 220, y: b.y - 40 }; b.retarget = 2.5; }
      }
    },
    lure(x, y) { for (const b of butterflies) { b.target = { x: x + (Math.random() - 0.5) * 40, y: y + 30 }; b.retarget = 4; } },
    drawLanded(ctx) { for (const l of leaves) if (l.landed) drawLeaf(ctx, l); },
    drawAir(ctx, strength) {
      const w = windOf(strength);
      for (const s of streaks) drawStreak(ctx, s, w);
      for (const l of leaves) if (!l.landed) drawLeaf(ctx, l);
      for (const b of butterflies) drawButterfly(ctx, b);
    },
  };
}

function meadowSpot(o) {
  const top = meadowHorizon(o.H) + 60;
  return { x: 60 + Math.random() * (o.W - 120), y: top + Math.random() * (o.H - o.taskbar - top - 60) };
}

function spawnLeaf(o) {
  const canopies = o.canopies, fromTree = canopies.length && Math.random() < 0.65;
  const c = fromTree ? canopies[Math.floor(Math.random() * canopies.length)] : null;
  const x = c ? c.x + (Math.random() - 0.5) * c.w * 0.8 : -20;
  const y = c ? c.y - c.h * (0.45 + Math.random() * 0.4) : Math.random() * (o.H - o.taskbar) * 0.85;
  return {
    x, y, vx: 10, groundY: (c ? c.y : y) + 30 + Math.random() * 180,
    rot: Math.random() * 6.28, spin: (Math.random() - 0.5) * 4, flutter: Math.random() * 6.28,
    size: 9 + Math.random() * 5, color: LEAF_COLORS[Math.floor(Math.random() * LEAF_COLORS.length)],
    landed: false, age: 0,
  };
}

// Lá bung ra từ tán cây bị rung, bay toả hai bên rồi rơi xuống quanh gốc.
function canopyLeaf(c) {
  return {
    x: c.x + (Math.random() - 0.5) * c.w * 0.8, y: c.y - c.h * (0.4 + Math.random() * 0.45), vx: (Math.random() - 0.5) * 70,
    groundY: c.y + 6 + Math.random() * 110, rot: Math.random() * 6.28, spin: (Math.random() - 0.5) * 6,
    flutter: Math.random() * 6.28, size: 9 + Math.random() * 5,
    color: LEAF_COLORS[Math.floor(Math.random() * LEAF_COLORS.length)], landed: false, age: 0,
  };
}

// Trả về false khi lá cần bị loại (bay khỏi màn hình hoặc đã mờ hẳn trên cỏ).
function stepLeaf(l, dt, wind, o) {
  if (l.landed) { l.age += dt; return l.age < LEAF_FADE_SECONDS; }
  l.flutter += dt * 3.2;
  l.vx += (wind * 60 + 8 - l.vx) * Math.min(1, dt * 1.5);
  l.x += l.vx * dt;
  l.y += (15 + Math.sin(l.flutter) * 22) * dt;
  l.rot += l.spin * dt * (1 + wind);
  if (l.y >= l.groundY) { l.landed = true; l.flutter = 0; }
  return l.x < o.W + 40 && l.y < o.H + 40;
}

function drawLeaf(ctx, l) {
  const s = l.size;
  ctx.save();
  ctx.globalAlpha = l.landed ? Math.max(0, 1 - l.age / LEAF_FADE_SECONDS) * 0.9 : 1;
  ctx.translate(l.x, l.y); ctx.rotate(l.rot);
  ctx.scale(1, l.landed ? 0.75 : 0.3 + 0.7 * Math.abs(Math.cos(l.flutter)));
  ctx.fillStyle = l.color;
  ctx.beginPath(); ctx.moveTo(-s, 0); ctx.quadraticCurveTo(0, -s * 0.75, s, 0); ctx.quadraticCurveTo(0, s * 0.75, -s, 0); ctx.fill();
  ctx.strokeStyle = 'rgba(120, 70, 20, .45)'; ctx.lineWidth = 0.8;
  ctx.beginPath(); ctx.moveTo(-s * 1.15, 0); ctx.lineTo(s * 0.9, 0); ctx.stroke();
  ctx.restore();
}

function spawnStreak(o) {
  const p = meadowSpot(o);
  return { x: p.x - o.W * 0.3, y: p.y - 40, len: 70 + Math.random() * 70, curl: Math.random() < 0.5 ? -1 : 1, age: 0 };
}

// Vệt gió trắng hình móc câu lướt sang phải rồi tan dần.
function drawStreak(ctx, s, wind) {
  const t = s.age / STREAK_LIFE, x = s.x + s.age * (120 + wind * 90), y = s.y;
  ctx.save();
  ctx.globalAlpha = Math.sin(t * Math.PI) * 0.75;
  ctx.strokeStyle = '#ffffff'; ctx.lineWidth = 2.6; ctx.lineCap = 'round';
  ctx.beginPath(); ctx.moveTo(x, y);
  ctx.quadraticCurveTo(x + s.len * 0.6, y - 6 * s.curl, x + s.len, y + 2 * s.curl);
  ctx.quadraticCurveTo(x + s.len + 12, y + 8 * s.curl, x + s.len + 4, y + 13 * s.curl);
  ctx.stroke();
  ctx.restore();
}

function spawnButterfly(o) {
  const p = meadowSpot(o);
  return { x: p.x, y: p.y - 30, target: meadowSpot(o), flap: Math.random() * 6, retarget: 3 + Math.random() * 4 };
}

function stepButterfly(b, dt, wind, o) {
  b.flap += dt * 16;
  b.retarget -= dt;
  const dx = b.target.x - b.x, dy = b.target.y - 30 - b.y, dist = Math.hypot(dx, dy);
  if (b.retarget <= 0 || dist < 12) { b.target = meadowSpot(o); b.retarget = 3 + Math.random() * 4; }
  const speed = 34;
  b.x += (dx / (dist || 1)) * speed * dt + wind * 6 * dt;
  b.y += (dy / (dist || 1)) * speed * dt + Math.sin(b.flap * 0.25) * 18 * dt;
  b.dir = dx >= 0 ? 1 : -1;
}

function drawButterfly(ctx, b) {
  const open = 0.25 + 0.75 * Math.abs(Math.sin(b.flap));
  ctx.save();
  ctx.translate(b.x, b.y); ctx.scale(b.dir || 1, 1);
  for (const side of [-1, 1]) {
    ctx.save(); ctx.scale(side * open, 1);
    ctx.fillStyle = BUTTERFLY_EDGE;
    ctx.beginPath(); ctx.ellipse(5.5, -4, 6.5, 5.5, -0.5, 0, Math.PI * 2); ctx.ellipse(4.5, 4, 4.5, 3.8, 0.5, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = BUTTERFLY_WING;
    ctx.beginPath(); ctx.ellipse(5.5, -4, 5, 4.2, -0.5, 0, Math.PI * 2); ctx.ellipse(4.5, 4, 3.4, 2.8, 0.5, 0, Math.PI * 2); ctx.fill();
    ctx.restore();
  }
  ctx.strokeStyle = '#4a3020'; ctx.lineWidth = 1.8; ctx.lineCap = 'round';
  ctx.beginPath(); ctx.moveTo(0, -6); ctx.lineTo(0, 6); ctx.stroke();
  ctx.restore();
}
