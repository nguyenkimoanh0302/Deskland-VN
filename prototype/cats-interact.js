'use strict';
// Tương tác chuột của chủ đề Mèo con: bấm vào mèo để vuốt ve (mèo nhảy cẫng, thả chùm tim), bấm vào cây để rung
// lá, bấm xuống cỏ để thả cuộn len cho vài chú mèo chạy tới vờn, bấm lên trời để gọi bướm. Mỗi lần bấm có vòng sáng nhỏ.

const YARN_SECONDS = 9;
const YARN_RADIUS = 11;
const YARN_COLORS = ['#e2557a', '#f6a3b9'];
const YARN_CHASERS = 3;
const YARN_CHASE_DISTANCE = 560;       // chỉ mèo trong khoảng này mới chạy tới vờn len
const YARN_BAT_SECONDS = 1.3;
const YARN_GRAVITY = 900;
const RING_SECONDS = 0.55;
const RING_MAX = 6;
const SKY_CLICK_BELOW_HORIZON = 60;

function createCatInteract() {
  let yarn = null, rings = [];

  function dropYarn(x, y, s) {
    const w = s.world;
    yarn = { x: Math.max(CAT_MARGIN_X, Math.min(w.W - CAT_MARGIN_X, x)), y: Math.max(w.top, Math.min(w.floor, y)),
      vx: 0, z: 46, vz: 0, age: 0, spin: 0 };
    s.wind.scatter(x, y);
    const near = s.cats.filter(c => c.state !== 'sleep' && Math.hypot(c.x - yarn.x, c.y - yarn.y) < YARN_CHASE_DISTANCE)
      .sort((a, b) => Math.hypot(a.x - yarn.x, a.y - yarn.y) - Math.hypot(b.x - yarn.x, b.y - yarn.y));
    near.slice(0, YARN_CHASERS).forEach((cat, slot) => catChase(cat, yarn, w, slot));
  }

  function release(cats, w, t) {
    for (const cat of cats) {
      if (!cat.play) continue;
      cat.play = false; cat.rush = 1;
      if (cat.state === 'play' || cat.state === 'walk') catStartWalk(cat, w, t);
    }
  }

  function stepYarn(dt, s) {
    yarn.age += dt;
    yarn.vz -= YARN_GRAVITY * dt;
    yarn.z += yarn.vz * dt;
    if (yarn.z <= 0) { yarn.z = 0; yarn.vz = yarn.vz < -60 ? -yarn.vz * 0.45 : 0; }
    yarn.x += yarn.vx * dt;
    yarn.vx *= Math.exp(-2.2 * dt);
    yarn.x = Math.max(CAT_MARGIN_X, Math.min(s.world.W - CAT_MARGIN_X, yarn.x));
    yarn.spin += yarn.vx * dt / YARN_RADIUS;
    for (const cat of s.cats) if (cat.play) stepPlayer(cat, dt, s.world);
  }

  // Mèo đang vờn: quay về phía cuộn len, thỉnh thoảng khều cho len lăn đi; len lăn xa thì đuổi theo.
  function stepPlayer(cat, dt, w) {
    if (cat.state !== 'play') return;
    cat.dir = yarn.x >= cat.x ? 1 : -1;
    if (Math.abs(yarn.x - cat.x) > 70 && Math.abs(yarn.vx) < 15) { catChase(cat, yarn, w, cat.slot); return; }
    cat.batIn -= dt;
    if (cat.batIn > 0) return;
    yarn.vx += cat.dir * (60 + Math.random() * 50);
    yarn.vz = 140;
    cat.batIn = YARN_BAT_SECONDS * (0.7 + Math.random() * 0.6);
  }

  return {
    get yarn() { return yarn; },
    active() { return !!yarn || rings.length > 0; },
    // s = { cats, nature, wind, world, tune, H }
    click(x, y, s) {
      if (rings.length >= RING_MAX) rings.shift();
      rings.push({ x, y, age: 0 });
      const cat = catAt(s.cats, x, y);
      if (cat) { catPet(cat, x); return; }
      const item = s.nature.itemAt(x, y);
      if (item) { s.nature.shake(item); s.wind.burst(item); return; }
      if (y < meadowHorizon(s.H) + SKY_CLICK_BELOW_HORIZON) { s.wind.lure(x, y); return; }
      if (yarn) release(s.cats, s.world, s.tune);
      dropYarn(x, y, s);
    },
    step(dt, s) {
      for (const r of rings) r.age += dt;
      rings = rings.filter(r => r.age < RING_SECONDS);
      if (!yarn) return;
      stepYarn(dt, s);
      if (yarn.age >= YARN_SECONDS) { yarn = null; release(s.cats, s.world, s.tune); }
    },
    drawYarn(ctx) { if (yarn) drawYarnBall(ctx, yarn); },
    drawRings(ctx) { for (const r of rings) drawClickRing(ctx, r); },
  };
}

function drawYarnBall(ctx, b) {
  const fade = Math.min(1, (YARN_SECONDS - b.age) / 0.6), cy = b.y - YARN_RADIUS - b.z;
  ctx.save();
  ctx.globalAlpha = fade;
  ctx.fillStyle = 'rgba(30, 60, 15, .28)';
  const shadow = YARN_RADIUS * (1 - Math.min(0.5, b.z / 120));
  ctx.beginPath(); ctx.ellipse(b.x, b.y - 1, shadow * 1.2, shadow * 0.4, 0, 0, Math.PI * 2); ctx.fill();
  ctx.strokeStyle = YARN_COLORS[0]; ctx.lineWidth = 1.6; ctx.lineCap = 'round';
  ctx.beginPath(); ctx.moveTo(b.x - YARN_RADIUS * 0.6, b.y - 2);
  ctx.bezierCurveTo(b.x - 18, b.y + 2, b.x - 24, b.y - 6, b.x - 32, b.y - 1); ctx.stroke();
  ctx.fillStyle = CAT_OUTLINE;
  ctx.beginPath(); ctx.arc(b.x, cy, YARN_RADIUS + 1.6, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = YARN_COLORS[0];
  ctx.beginPath(); ctx.arc(b.x, cy, YARN_RADIUS, 0, Math.PI * 2); ctx.fill();
  ctx.strokeStyle = YARN_COLORS[1]; ctx.lineWidth = 1.4;
  for (let i = 0; i < 3; i++) {
    const a = b.spin + i * 1.05;
    ctx.beginPath(); ctx.ellipse(b.x, cy, YARN_RADIUS * 0.8, YARN_RADIUS * 0.32, a, 0, Math.PI * 2); ctx.stroke();
  }
  ctx.restore();
}

function drawClickRing(ctx, r) {
  const t = r.age / RING_SECONDS;
  ctx.save();
  ctx.globalAlpha = (1 - t) * 0.8;
  ctx.strokeStyle = '#ffffff'; ctx.lineWidth = 2.6;
  ctx.beginPath(); ctx.arc(r.x, r.y, 6 + t * 30, 0, Math.PI * 2); ctx.stroke();
  ctx.lineWidth = 1.6;
  ctx.beginPath(); ctx.arc(r.x, r.y, 3 + t * 16, 0, Math.PI * 2); ctx.stroke();
  ctx.restore();
}
