'use strict';
// Các tư thế của chú mèo mập: đi, ngồi, ngồi liếm lông, ngủ cuộn tròn và nằm nghiêng ngủ.
// Gốc toạ độ là điểm giữa hai chân trên mặt cỏ, hình quay mặt sang phải; lật bằng scale(-1) khi đi sang trái.

const CAT_HEART = '#ff6f91';
const CAT_SHADOW_RX = { walk: 32, sit: 26, groom: 26, play: 26, sleep: 34 };
const CAT_HOP_HEIGHT = 16;
const CAT_CUSHION = ['#ef8fa8', '#f8b7c8'];

function drawCat(ctx, cat) {
  const k = cat.size / 60;
  ctx.save();
  ctx.translate(cat.x, cat.y);
  if (cat.state === 'sleep' && cat.cushion) drawCushion(ctx, k);
  ctx.fillStyle = 'rgba(30, 60, 15, .30)';
  const lift = catHopLift(cat);
  ctx.beginPath(); ctx.ellipse(0, -1, CAT_SHADOW_RX[cat.state] * k * (1 - lift * 0.02), 6 * k, 0, 0, Math.PI * 2); ctx.fill();
  ctx.translate(0, -lift * k);
  ctx.scale(cat.dir * k, k);
  if (cat.state === 'walk') drawMotionMarks(ctx, cat);
  ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  catOutlinePass = true;
  drawCatPose(ctx, cat);
  catOutlinePass = false;
  drawCatPose(ctx, cat);
  ctx.restore();
  if (cat.state === 'sleep') drawSnore(ctx, cat, k);
  if (cat.heart > 0) drawHeartPop(ctx, cat, k);
  if (cat.love > 0) drawLoveBurst(ctx, cat, k);
}

// Nhảy cẫng khi được vuốt ve: một cú bật lên rồi rơi xuống (px, chưa nhân tỉ lệ mèo).
function catHopLift(cat) {
  return cat.hop > 0 ? Math.sin((1 - cat.hop / CAT_HOP_SECONDS) * Math.PI) * CAT_HOP_HEIGHT : 0;
}

function drawCatPose(ctx, cat) {
  if (cat.state === 'sleep') SLEEP_POSES[cat.sleepPose](ctx, cat);
  else if (cat.state === 'groom') drawGroomingCat(ctx, cat);
  else if (cat.state === 'play') drawPlayingCat(ctx, cat);
  else if (cat.state === 'sit') drawSittingCat(ctx, cat);
  else drawWalkingCat(ctx, cat);
}

function drawWalkingCat(ctx, cat) {
  const c = cat.coat, p = cat.phase, bob = Math.abs(Math.sin(p)) * 1.3;
  drawTail(ctx, c.tail, -27, -29 - bob, [-38, -33, -41 + cat.sway * 5, -48, -33 + cat.sway * 8, -57]);
  drawWalkLeg(ctx, c.shade, c.shade, 14, -10 - bob, p + Math.PI);
  drawWalkLeg(ctx, c.shade, c.shade, -16, -10 - bob, p);
  drawBody(ctx, c, 0, -24 - bob, 30, 20);
  drawWalkLeg(ctx, legColor(c), c.paw, 9, -10 - bob, p);
  drawWalkLeg(ctx, legColor(c), c.paw, -21, -10 - bob, p + Math.PI);
  drawHead(ctx, cat, 27, -43 - bob);
}

function drawSittingBase(ctx, cat) {
  const c = cat.coat;
  drawTail(ctx, c.tail, -18, -5, [-8, 7, 10, 7, 23 + cat.sway * 3, -1 - Math.abs(cat.sway) * 4]);
  fillEllipse(ctx, c.body, -2, -22, 24, 22, 0);
  if (c.kind !== 'solid') detailEllipse(ctx, c.belly, 7, -19, 11, 14, -0.1);
  if (c.kind === 'calico') detailEllipse(ctx, c.mark, -11, -28, 9, 10, 0.3);
}

function drawSittingCat(ctx, cat) {
  const c = cat.coat;
  drawSittingBase(ctx, cat);
  drawLegLine(ctx, legColor(c), c.paw, [4, -14], [4, -5]);
  drawLegLine(ctx, legColor(c), c.paw, [13, -14], [13, -5]);
  drawHead(ctx, cat, 5, -49);
}

// Ngồi liếm lông: một chân trước đưa lên miệng, đầu cúi theo nhịp liếm, mắt lim dim, lưỡi thè ra.
function drawGroomingCat(ctx, cat) {
  const c = cat.coat, lick = Math.sin(cat.clock * 6);
  drawSittingBase(ctx, cat);
  drawLegLine(ctx, legColor(c), c.paw, [4, -14], [4, -5]);
  drawHead(ctx, cat, 9, -47 + lick * 0.8, { asleep: true, tilt: 0.22 + lick * 0.06, tongue: Math.max(0, lick) });
  drawLegLine(ctx, legColor(c), c.paw, [13, -24], [19, -37 - lick * 2.5]);
}

// Vờn cuộn len: chân trước khều xuống đất theo nhịp, đầu cúi nhìn cuộn len.
function drawPlayingCat(ctx, cat) {
  const c = cat.coat, tap = Math.max(0, Math.sin(cat.clock * 7));
  drawSittingBase(ctx, cat);
  drawLegLine(ctx, legColor(c), c.paw, [4, -14], [4, -5]);
  drawHead(ctx, cat, 8, -47 + tap * 0.6, { tilt: 0.2 });
  drawLegLine(ctx, legColor(c), c.paw, [13, -22], [27 + tap * 3, -8 - tap * 9]);
}

function drawLoafSleepingCat(ctx, cat) {
  const c = cat.coat, breath = Math.sin(cat.clock * 1.6);
  drawBody(ctx, c, -2, -15 - breath * 0.6, 31, 16 + breath * 1.8);
  drawTail(ctx, c.tail, -29, -6, [-23, 9, 8, 8, 25, 0]);
  drawHead(ctx, cat, 21, -18 - breath * 0.9, { asleep: true, tilt: 0.12 });
}

// Nằm nghiêng duỗi người: bụng phồng lên xẹp xuống rõ theo nhịp thở.
function drawSideSleepingCat(ctx, cat) {
  const c = cat.coat, breath = Math.sin(cat.clock * 1.6);
  drawTail(ctx, c.tail, -30, -8, [-42, -5, -50, 0, -60, -6]);
  drawLegLine(ctx, legColor(c), c.paw, [-15, -10], [-8, -4]);
  drawLegLine(ctx, legColor(c), c.paw, [13, -10], [21, -4]);
  drawBody(ctx, c, 0, -15 - breath * 0.9, 33, 13 + breath * 2);
  drawHead(ctx, cat, 33, -13, { asleep: true, tilt: 0.32 });
}

// Nằm ngửa phơi bụng, bốn chân co lên trời, bụng phồng xẹp theo nhịp thở.
function drawBackSleepingCat(ctx, cat) {
  const c = cat.coat, breath = Math.sin(cat.clock * 1.6);
  drawTail(ctx, c.tail, -28, -8, [-40, -4, -46, 2, -56, -2]);
  for (const [x, lean] of [[-16, -3], [-4, -1], [8, 2], [18, 4]]) {
    drawLegLine(ctx, legColor(c), c.paw, [x, -24], [x + lean, -39 - breath * 0.6]);
  }
  fillEllipse(ctx, c.body, 0, -15, 29, 15 + breath * 1.2, 0);
  detailEllipse(ctx, c.kind === 'solid' ? c.shade : c.belly, 2, -18 - breath * 0.6, 20, 9 + breath * 1.4, 0);
  drawHead(ctx, cat, 31, -14, { asleep: true, tilt: -0.45 });
}

const SLEEP_POSES = { loaf: drawLoafSleepingCat, side: drawSideSleepingCat, back: drawBackSleepingCat };

function drawCushion(ctx, k) {
  ctx.fillStyle = CAT_OUTLINE;
  ctx.beginPath(); ctx.ellipse(0, -4 * k, 46 * k, 14 * k, 0, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = CAT_CUSHION[0];
  ctx.beginPath(); ctx.ellipse(0, -4 * k, 44.5 * k, 12.5 * k, 0, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = CAT_CUSHION[1];
  ctx.beginPath(); ctx.ellipse(0, -7 * k, 36 * k, 8 * k, 0, 0, Math.PI * 2); ctx.fill();
}

// Vài vệt trắng nhỏ phía sau khi mèo đang đi, giống nét "chuyển động" trong tranh hoạt hình.
function drawMotionMarks(ctx, cat) {
  const wiggle = Math.sin(cat.phase * 2) * 2;
  ctx.strokeStyle = 'rgba(255, 255, 255, .85)'; ctx.lineWidth = 2.2; ctx.lineCap = 'round';
  for (const [y, len] of [[-34, 9], [-24, 12], [-14, 8]]) {
    ctx.beginPath(); ctx.moveTo(-40 + wiggle, y); ctx.lineTo(-40 - len + wiggle, y + 2); ctx.stroke();
  }
}

function drawSnore(ctx, cat, k) {
  const cycle = (cat.clock * 0.45) % 1;
  ctx.save();
  ctx.globalAlpha = Math.sin(cycle * Math.PI) * 0.85;
  ctx.fillStyle = '#4f6f86';
  ctx.font = `700 ${Math.round((10 + cycle * 6) * k)}px "Segoe UI", sans-serif`;
  ctx.fillText('z', cat.x + cat.dir * (26 + cycle * 9) * k, cat.y - (36 + cycle * 24) * k);
  ctx.restore();
}

// Trái tim nhỏ bay lên khi hai chú mèo ngồi cạnh nhau.
function drawHeartPop(ctx, cat, k) {
  const life = Math.min(1, cat.heart), rise = (1 - life) * 14;
  drawHeart(ctx, cat.x + cat.dir * 6 * k, cat.y - (80 + rise) * k, 5 * k, Math.min(1, life * 2));
}

// Chùm ba trái tim toả lên khi được vuốt ve.
function drawLoveBurst(ctx, cat, k) {
  const t = 1 - cat.love / CAT_LOVE_SECONDS;
  for (let i = 0; i < 3; i++) {
    const p = Math.min(1, Math.max(0, (t - i * 0.15) / 0.7));
    if (p <= 0 || p >= 1) continue;
    const x = cat.x + ((i - 1) * 18 * p + cat.dir * 4) * k, y = cat.y - (68 + p * 42 + (i === 1 ? 8 : 0)) * k;
    drawHeart(ctx, x, y, (4 + p * 2.5) * k, Math.sin(p * Math.PI));
  }
}

function drawHeart(ctx, x, y, r, alpha) {
  ctx.save();
  ctx.globalAlpha = alpha;
  ctx.fillStyle = CAT_HEART;
  ctx.beginPath();
  ctx.arc(x - r, y, r, Math.PI, 0); ctx.arc(x + r, y, r, Math.PI, 0);
  ctx.lineTo(x, y + r * 2.4); ctx.closePath(); ctx.fill();
  ctx.restore();
}
