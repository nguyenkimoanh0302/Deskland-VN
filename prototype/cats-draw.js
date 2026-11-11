'use strict';
// Bộ phận cơ thể của chú mèo mập (thân, chân, đuôi, đầu, mặt) bằng Canvas 2D.
// Mỗi tư thế được vẽ hai lượt: lượt viền (tô cả hình bằng màu viền, nét dày) rồi lượt màu phủ lên,
// nhờ vậy cả dáng mèo có một đường viền liền mạch giúp nổi bật trên nền cỏ.

const CAT_COATS = [
  { body: '#f6a85a', shade: '#dc8a3e', belly: '#fee6c8', mark: '#d4742c', tail: '#f6a85a', paw: '#fee6c8', kind: 'tabby', eye: '#6fb83f' },
  { body: '#a6acb4', shade: '#838990', belly: '#e6e9ed', mark: '#6c7178', tail: '#a6acb4', paw: '#e6e9ed', kind: 'tabby', eye: '#e8b730' },
  { body: '#3b3635', shade: '#272322', belly: '#f7f3ed', mark: '#3b3635', tail: '#3b3635', paw: '#f7f3ed', kind: 'tux', eye: '#e8b730' },
  { body: '#fdf8f0', shade: '#e2d9cc', belly: '#ffffff', mark: '#ea9f55', tail: '#433a3c', paw: '#ffffff', kind: 'calico', eye: '#6fb83f' },
  { body: '#433d3f', shade: '#2c2729', belly: '#433d3f', mark: '#433d3f', tail: '#433d3f', paw: '#433d3f', kind: 'solid', eye: '#f2c43a' },
  { body: '#f6e6cb', shade: '#755646', belly: '#fff8ec', mark: '#755646', tail: '#755646', paw: '#755646', kind: 'point', eye: '#58a8e0' },
  { body: '#f3dcb0', shade: '#dcbf8c', belly: '#fff6e6', mark: '#d9b27a', tail: '#f3dcb0', paw: '#fff6e6', kind: 'tabby', eye: '#58a8e0' },
  { body: '#ffffff', shade: '#e6e6ea', belly: '#ffffff', mark: '#ffffff', tail: '#ffffff', paw: '#ffffff', kind: 'solid', eye: '#58a8e0' },
  { body: '#fdfdfb', shade: '#dedcd8', belly: '#ffffff', mark: '#8d8f94', tail: '#8d8f94', paw: '#ffffff', kind: 'patch', eye: '#6fb83f' },
  { body: '#fdfdfb', shade: '#dedcd8', belly: '#ffffff', mark: '#5e6066', tail: '#5e6066', paw: '#ffffff', kind: 'patch', eye: '#e8b730' },
];
const CAT_OUTLINE = 'rgba(128, 82, 52, .95)';
const CAT_OUTLINE_WIDTH = 2.8;
const CAT_EAR_PINK = '#f7aaaa';
const CAT_BLUSH = 'rgba(255, 130, 145, .6)';
const CAT_LINE = '#3a2a22';
const CAT_TONGUE = '#ff8a9a';
const CAT_LEG_WIDTH = 10;
const CAT_TAIL_WIDTH = 9;
const CAT_COLLAR = '#e2403f';
const CAT_BELL = '#f7c531';

let catOutlinePass = false;

function paintFill(ctx, color) {
  if (!catOutlinePass) { ctx.fillStyle = color; ctx.fill(); return; }
  ctx.fillStyle = CAT_OUTLINE; ctx.fill();
  ctx.strokeStyle = CAT_OUTLINE; ctx.lineWidth = CAT_OUTLINE_WIDTH; ctx.stroke();
}

function paintStroke(ctx, color, width) {
  ctx.strokeStyle = catOutlinePass ? CAT_OUTLINE : color;
  ctx.lineWidth = catOutlinePass ? width + CAT_OUTLINE_WIDTH : width;
  ctx.stroke();
}

function fillEllipse(ctx, color, x, y, rx, ry, rotation) {
  ctx.beginPath(); ctx.ellipse(x, y, rx, ry, rotation, 0, Math.PI * 2);
  paintFill(ctx, color);
}

// Chi tiết bên trong (hoa văn, bụng, mặt) không cần viền nên bỏ qua ở lượt viền.
function detailEllipse(ctx, color, x, y, rx, ry, rotation) {
  if (catOutlinePass) return;
  ctx.fillStyle = color;
  ctx.beginPath(); ctx.ellipse(x, y, rx, ry, rotation, 0, Math.PI * 2); ctx.fill();
}

function legColor(c) { return c.kind === 'point' ? c.shade : c.body; }

function drawTail(ctx, color, x, y, curve) {
  ctx.beginPath(); ctx.moveTo(x, y); ctx.bezierCurveTo(...curve);
  paintStroke(ctx, color, CAT_TAIL_WIDTH);
}

function drawLegLine(ctx, color, pawColor, from, to) {
  ctx.beginPath(); ctx.moveTo(from[0], from[1]); ctx.lineTo(to[0], to[1]);
  paintStroke(ctx, color, CAT_LEG_WIDTH);
  fillEllipse(ctx, pawColor, to[0] + 1, to[1] + 1.2, 6, 3.8, 0);
}

function drawWalkLeg(ctx, color, pawColor, hipX, hipY, phase) {
  const lift = Math.max(0, Math.cos(phase)) * 2.5;
  drawLegLine(ctx, color, pawColor, [hipX, hipY], [hipX + Math.sin(phase) * 4, -5 - lift]);
}

function drawBody(ctx, c, cx, cy, rx, ry) {
  fillEllipse(ctx, c.body, cx, cy, rx, ry, 0);
  if (catOutlinePass) return;
  ctx.save();
  ctx.beginPath(); ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2); ctx.clip();
  if (c.kind !== 'solid') detailEllipse(ctx, c.belly, cx + 4, cy + ry * 0.62, rx * 0.72, ry * 0.5, 0);
  drawCoatMarks(ctx, c, cx, cy, rx, ry);
  ctx.restore();
}

function drawCoatMarks(ctx, c, cx, cy, rx, ry) {
  if (c.kind === 'tabby') {
    ctx.strokeStyle = c.mark; ctx.lineWidth = 3.6;
    for (let i = -1; i <= 1; i++) {
      const sx = cx + i * rx * 0.34;
      ctx.beginPath(); ctx.moveTo(sx - 3, cy - ry); ctx.quadraticCurveTo(sx + 2, cy - ry * 0.45, sx - 1, cy); ctx.stroke();
    }
  } else if (c.kind === 'calico' || c.kind === 'patch') {
    detailEllipse(ctx, c.mark, cx - rx * 0.35, cy - ry * 0.45, rx * 0.42, ry * 0.6, 0.3);
    detailEllipse(ctx, c.tail, cx + rx * 0.4, cy - ry * 0.6, rx * 0.3, ry * 0.45, -0.2);
  }
}

// opts: asleep (nhắm mắt), tilt (nghiêng đầu, radian), tongue (0..1 thè lưỡi khi liếm lông).
function drawHead(ctx, cat, hx, hy, opts = {}) {
  const c = cat.coat;
  ctx.save();
  ctx.translate(hx, hy); ctx.rotate(opts.tilt || 0);
  drawEar(ctx, c, -9, -8, -1, cat.earTwitch * 2);
  drawEar(ctx, c, 9, -8, 1, 0);
  fillEllipse(ctx, c.body, 0, 0, 17.5, 14.5, 0);
  fillEllipse(ctx, c.body, 0, 4.5, 19.5, 11, 0);
  if (!catOutlinePass) {
    if (c.kind === 'point') detailEllipse(ctx, c.mark, 1, 4, 10, 7.5, 0);
    if (c.kind === 'tux') detailEllipse(ctx, c.belly, 1, 7, 10, 6.5, 0);
    if (c.kind === 'calico' || c.kind === 'patch') detailEllipse(ctx, c.mark, -8, -5, 7.5, 6, 0.4);
    if (c.kind === 'tabby') drawForeheadStripes(ctx, c.mark);
    drawFace(ctx, cat, 1.5, 1.5, opts);
    if (cat.collar) drawCollar(ctx);
  }
  ctx.restore();
}

function drawCollar(ctx) {
  ctx.strokeStyle = CAT_COLLAR; ctx.lineWidth = 3.4;
  ctx.beginPath(); ctx.arc(1, -2, 17, 0.55, Math.PI - 0.55); ctx.stroke();
  detailEllipse(ctx, CAT_OUTLINE, 3, 16.5, 3.8, 3.8, 0);
  detailEllipse(ctx, CAT_BELL, 3, 16.5, 3, 3, 0);
}

function drawEar(ctx, c, x, y, side, tilt) {
  const tipX = x + side * 3.5 + tilt, tipY = y - 11;
  ctx.beginPath(); ctx.moveTo(x - 7, y + 4); ctx.quadraticCurveTo(tipX - side, tipY - 2, tipX, tipY); ctx.lineTo(x + 7, y + 4); ctx.closePath();
  paintFill(ctx, c.kind === 'point' ? c.mark : c.body);
  if (catOutlinePass) return;
  ctx.fillStyle = CAT_EAR_PINK;
  ctx.beginPath(); ctx.moveTo(x - 3.5, y + 2); ctx.lineTo(tipX, tipY + 5); ctx.lineTo(x + 3.5, y + 2); ctx.closePath(); ctx.fill();
}

function drawForeheadStripes(ctx, color) {
  ctx.strokeStyle = color; ctx.lineWidth = 2.2;
  for (const dx of [-4, 0, 4]) { ctx.beginPath(); ctx.moveTo(dx, -14); ctx.lineTo(dx * 0.8, -8.5); ctx.stroke(); }
}

function drawFace(ctx, cat, fx, fy, opts) {
  for (const ex of [fx - 7, fx + 7]) {
    if (opts.asleep || cat.blink > 0) {
      ctx.strokeStyle = CAT_LINE; ctx.lineWidth = 1.7;
      ctx.beginPath(); ctx.arc(ex, fy - 2, 3.2, 0.2, Math.PI - 0.2); ctx.stroke();
      continue;
    }
    if (cat.mood === 'happy' || cat.joy > 0) {
      // Mắt cười híp "^ ^" như tranh mẫu.
      ctx.strokeStyle = CAT_LINE; ctx.lineWidth = 2;
      ctx.beginPath(); ctx.arc(ex, fy, 3.4, Math.PI + 0.35, Math.PI * 2 - 0.35); ctx.stroke();
      continue;
    }
    detailEllipse(ctx, '#1b1514', ex, fy - 1.5, 3.9, 4.6, 0);
    detailEllipse(ctx, cat.coat.eye, ex, fy - 0.3, 2.9, 3, 0);
    detailEllipse(ctx, '#1b1514', ex, fy - 0.8, 1.8, 2.7, 0);
    detailEllipse(ctx, '#ffffff', ex + 1.4, fy - 3.4, 1.5, 1.5, 0);
    detailEllipse(ctx, '#ffffff', ex - 1.2, fy + 0.9, 0.7, 0.7, 0);
  }
  detailEllipse(ctx, CAT_BLUSH, fx - 11, fy + 5, 4, 2.5, 0);
  detailEllipse(ctx, CAT_BLUSH, fx + 11, fy + 5, 4, 2.5, 0);
  if (opts.tongue > 0) detailEllipse(ctx, CAT_TONGUE, fx, fy + 7.5, 2.4, 2.6 * opts.tongue, 0);
  ctx.fillStyle = '#e8868c';
  ctx.beginPath(); ctx.moveTo(fx - 2.2, fy + 3); ctx.lineTo(fx + 2.2, fy + 3); ctx.lineTo(fx, fy + 5); ctx.closePath(); ctx.fill();
  ctx.strokeStyle = CAT_LINE; ctx.lineWidth = 1.1;
  ctx.beginPath(); ctx.arc(fx - 1.8, fy + 5.2, 1.8, 0, Math.PI); ctx.arc(fx + 1.8, fy + 5.2, 1.8, 0, Math.PI); ctx.stroke();
  ctx.strokeStyle = 'rgba(255, 255, 255, .85)'; ctx.lineWidth = 0.9;
  for (const side of [-1, 1]) for (const dy of [-1, 1.5]) {
    ctx.beginPath(); ctx.moveTo(fx + side * 10, fy + 4 + dy * 0.5); ctx.lineTo(fx + side * 21, fy + 3 + dy * 1.7); ctx.stroke();
  }
}
