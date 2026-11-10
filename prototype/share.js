// Deskland · Chia sẻ một chạm (dùng chung cho tất cả trang chủ đề, tải sau script chủ đề)
// Vẽ lại khung hình hiện tại theo độ phân giải thực tế của màn hình, sau đó vẽ biểu tượng mẫu tại vị trí từng biểu tượng
// và vẽ tên tệp thành khối bo góc màu trắng để bảo vệ quyền riêng tư (không chụp màn hình, không đọc tên tệp hay hình ảnh biểu tượng thật).
(function () {
  function rnd(x, y, k) {
    let h = Math.imul(Math.round(x) | 0, 374761393) ^ Math.imul(Math.round(y) | 0, 668265263) ^ Math.imul(k + 7, 2246822519);
    h = Math.imul(h ^ (h >>> 13), 1274126177);
    return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
  }
  function roundRect(ctx, x, y, w, h, r) { ctx.beginPath(); addRoundRect(ctx, x, y, w, h, r); }
  function addRoundRect(ctx, x, y, w, h, r) {
    ctx.moveTo(x + r, y); ctx.arcTo(x + w, y, x + w, y + h, r); ctx.arcTo(x + w, y + h, x, y + h, r);
    ctx.arcTo(x, y + h, x, y, r); ctx.arcTo(x, y, x + w, y, r); ctx.closePath();
  }
  const BADGES = ['#2b6cd4', '#1d7d45', '#c8372c', '#d86f2a', '#7a3fc4'];
  const APPS = [['#29a3e8', '#1474c4'], ['#9b5de5', '#6a35c2'], ['#20c08f', '#0f8f68'], ['#ff7a59', '#e0452c']];
  function sampleGlyph(ctx, x, y, g, u) {
    const kind = u(1), sh = Math.max(1, g * 0.03);
    const shape = (fill, f) => { ctx.fillStyle = 'rgba(0,0,0,0.18)'; ctx.save(); ctx.translate(0, sh); f(); ctx.fill(); ctx.restore(); ctx.fillStyle = fill; f(); ctx.fill(); };
    if (kind < 0.36) {
      shape('#e3a32f', () => { ctx.beginPath(); addRoundRect(ctx, x + g * 0.06, y + g * 0.2, g * 0.88, g * 0.64, g * 0.07);
        addRoundRect(ctx, x + g * 0.06, y + g * 0.14, g * 0.38, g * 0.2, g * 0.06); });
      ctx.fillStyle = '#fcd25c'; roundRect(ctx, x + g * 0.06, y + g * 0.32, g * 0.88, g * 0.52, g * 0.07); ctx.fill();
      ctx.fillStyle = 'rgba(255,255,255,0.35)'; ctx.fillRect(x + g * 0.12, y + g * 0.36, g * 0.76, g * 0.03);
    } else if (kind < 0.76) {
      const px = x + g * 0.2, py = y + g * 0.06, pw = g * 0.6, ph = g * 0.84, c = g * 0.16;
      shape('#f7f9fc', () => { ctx.beginPath(); ctx.moveTo(px, py + g * 0.04); ctx.arcTo(px, py, px + g * 0.04, py, g * 0.04);
        ctx.lineTo(px + pw - c, py); ctx.lineTo(px + pw, py + c); ctx.lineTo(px + pw, py + ph - g * 0.04);
        ctx.arcTo(px + pw, py + ph, px + pw - g * 0.04, py + ph, g * 0.04); ctx.lineTo(px + g * 0.04, py + ph);
        ctx.arcTo(px, py + ph, px, py + ph - g * 0.04, g * 0.04); ctx.closePath(); });
      ctx.fillStyle = '#d9dfe7'; ctx.beginPath(); ctx.moveTo(px + pw - c, py); ctx.lineTo(px + pw - c, py + c); ctx.lineTo(px + pw, py + c); ctx.fill();
      ctx.fillStyle = '#cfd6df';
      for (let i = 0; i < 4; i++) ctx.fillRect(px + pw * 0.38, py + ph * (0.3 + i * 0.13), pw * 0.48, g * 0.035);
      const badge = BADGES[Math.floor(u(2) * BADGES.length)];
      shape(badge, () => { ctx.beginPath(); roundRect(ctx, x + g * 0.08, y + g * 0.44, g * 0.42, g * 0.38, g * 0.06); });
      ctx.fillStyle = 'rgba(255,255,255,0.9)'; roundRect(ctx, x + g * 0.17, y + g * 0.56, g * 0.24, g * 0.14, g * 0.03); ctx.fill();
    } else if (kind < 0.9) {
      const fx = x + g * 0.1, fy = y + g * 0.16, fw = g * 0.8, fh = g * 0.66, m = g * 0.06;
      shape('#f7f9fc', () => { ctx.beginPath(); roundRect(ctx, fx, fy, fw, fh, g * 0.05); });
      ctx.save(); ctx.beginPath(); ctx.rect(fx + m, fy + m, fw - 2 * m, fh - 2 * m); ctx.clip();
      ctx.fillStyle = '#8fd0f2'; ctx.fillRect(fx + m, fy + m, fw - 2 * m, fh - 2 * m);
      ctx.fillStyle = '#ffd34d'; ctx.beginPath(); ctx.arc(fx + fw * 0.72, fy + fh * 0.32, g * 0.07, 0, 7); ctx.fill();
      ctx.fillStyle = '#5cbf6c'; ctx.beginPath(); ctx.moveTo(fx, fy + fh); ctx.lineTo(fx + fw * 0.36, fy + fh * 0.44); ctx.lineTo(fx + fw * 0.66, fy + fh); ctx.fill();
      ctx.fillStyle = '#3e9f58'; ctx.beginPath(); ctx.moveTo(fx + fw * 0.4, fy + fh); ctx.lineTo(fx + fw * 0.7, fy + fh * 0.58); ctx.lineTo(fx + fw, fy + fh * 0.9); ctx.lineTo(fx + fw, fy + fh); ctx.fill();
      ctx.restore();
    } else {
      const [a, b] = APPS[Math.floor(u(3) * APPS.length)];
      const grad = ctx.createLinearGradient(0, y + g * 0.12, 0, y + g * 0.88);
      grad.addColorStop(0, a); grad.addColorStop(1, b);
      shape(grad, () => { ctx.beginPath(); roundRect(ctx, x + g * 0.12, y + g * 0.12, g * 0.76, g * 0.76, g * 0.18); });
    }
  }
  function sampleLabel(ctx, cx, y, w, lh) {
    const h = lh * 0.56, r = h / 2, o = Math.max(1, lh * 0.07);
    ctx.fillStyle = 'rgba(0,0,0,0.35)'; roundRect(ctx, cx - w / 2 + o, y + (lh - h) / 2 + o, w, h, r); ctx.fill();
    ctx.fillStyle = 'rgba(255,255,255,0.95)'; roundRect(ctx, cx - w / 2, y + (lh - h) / 2, w, h, r); ctx.fill();
  }
  function drawIcon(ctx, ic, s, px, py, lift) {
    if (!ic._seed) ic._seed = [ic.x, ic.y];
    const u = k => rnd(ic._seed[0], ic._seed[1], k);
    lift = lift || 0;
    ctx.save();
    if (lift > 0) {
      const sc = 1 + 0.06 * lift, cx = (px + ic.w / 2) * s, cy = (py + ic.h / 2) * s;
      ctx.translate(cx, cy); ctx.scale(sc, sc); ctx.translate(-cx, -cy);
      ctx.globalAlpha = 1 - 0.25 * lift;
    }
    const x = px * s, y = py * s, w = ic.w * s, h = ic.h * s;
    const g = Math.min(48 * s, w - 8 * s, h * 0.62);
    sampleGlyph(ctx, x + (w - g) / 2, y + 3 * s, g, u);
    const lh = 15 * s, ly = y + 3 * s + g + 4 * s, lines = u(9) < 0.45 ? 2 : 1;
    if (ly + lh <= y + h + 6 * s)
      for (let i = 0; i < lines; i++) sampleLabel(ctx, x + w / 2, ly + i * lh, w * (i ? 0.3 + 0.35 * u(11) : 0.5 + 0.42 * u(10)), lh);
    ctx.restore();
  }
  function labelsReady() {
    return typeof renderLabels === 'function' && ((typeof city !== 'undefined' && city) || (typeof world !== 'undefined' && world));
  }
  function drawSampleIcons(ctx, s) { for (const ic of icons) if (!ic._lockWidget) drawIcon(ctx, ic, s, ic.x, ic.y, 0); }
  function snapshot(scale, withIcons) {
    if (withIcons === undefined) withIcons = true;
    scale = scale || window.devicePixelRatio || 1;
    const old = { w: cv.width, h: cv.height, s: cv._scale };
    const hasLabels = labelsReady();
    const hasCache = typeof cacheOK !== 'undefined';
    const box = cv.getBoundingClientRect();
    cv.width = Math.round((box.width || W) * scale); cv.height = Math.round((box.height || H) * scale); cv._scale = scale;
    if (hasLabels) renderLabels();
    if (hasCache) cacheOK = false;
    draw(performance.now());
    const out = document.createElement('canvas');
    out.width = cv.width; out.height = cv.height;
    const ctx = out.getContext('2d');
    ctx.drawImage(cv, 0, 0);
    cv.width = old.w; cv.height = old.h; cv._scale = old.s;
    if (hasLabels) renderLabels();
    if (hasCache) cacheOK = false;
    draw(performance.now());
    if (withIcons) drawSampleIcons(ctx, scale);
    return out;
  }
  function thumbOf(canvas, maxW) {
    const t = document.createElement('canvas');
    t.width = maxW; t.height = Math.round(canvas.height * maxW / canvas.width);
    const c = t.getContext('2d'); c.imageSmoothingQuality = 'high';
    c.drawImage(canvas, 0, 0, t.width, t.height);
    return t.toDataURL('image/jpeg', 0.86);
  }
  window.desklandSnapshot = function (scale, withIcons) { const c = snapshot(scale, withIcons); return { canvas: c, thumb: thumbOf(c, 520) }; };
  window.desklandSnapshot.parts = { snapshot, drawSampleIcons, sampleGlyph, sampleLabel, thumbOf };

  /* ---- Chia sẻ hoạt ảnh: quay 12 giây hoạt ảnh chủ đề theo bố cục màn hình hiện tại ---- */
  function planMoves(list, n, only) {
    const H2 = H - TASKBAR, cells = [];
    for (let c = 0; GRID_X0 + (c + 1) * CELL_W <= W; c++)
      for (let r = 0; GRID_Y0 + (r + 1) * CELL_H <= H2; r++) cells.push([GRID_X0 + c * CELL_W, GRID_Y0 + r * CELL_H]);
    const pos = new Map(list.map(ic => [ic, [ic.x, ic.y]]));
    const ctr = ic => { const p = pos.get(ic); return [p[0] + ic.w / 2, p[1] + ic.h / 2]; };
    const moves = [];
    for (let k = 0; k < n; k++) {
      let best = null;
      for (const ic of list) {
        if (only && ic !== only) continue;
        const [cx, cy] = ctr(ic);
        let nb = 0;
        for (const o of list) if (o !== ic) { const [ox, oy] = ctr(o); if (Math.hypot(ox - cx, oy - cy) < 170) nb++; }
        for (const [x, y] of cells) {
          const dx = x + ic.w / 2, dy = y + ic.h / 2, d = Math.hypot(dx - cx, dy - cy);
          if (d < 260 || d > 900 || Math.min(dx, dy, W - dx, H2 - dy) < 90) continue;
          let md = 1e9;
          for (const o of list) if (o !== ic) { const [ox, oy] = ctr(o); md = Math.min(md, Math.hypot(ox - dx, oy - dy)); }
          const score = Math.min(md, 420) + Math.min(nb, 6) * 25 + rnd(x, y, 3 + (window.__recSeed || 0)) * 30;
          if (!best || score > best.score) best = { ic, from: pos.get(ic).slice(), to: [x, y], score };
        }
      }
      if (!best) break;
      moves.push(best);
      pos.set(best.ic, best.to);
    }
    return moves;
  }
  function planGroup(list, pos, fixed) {
    if (list.length < 4) return null;
    const H2 = H - TASKBAR, ctr = ic => { const p = pos.get(ic); return [p[0] + ic.w / 2, p[1] + ic.h / 2]; };
    let cand = [];
    if (fixed) cand = [fixed];
    else {
      let seed = null, bn = -1;
      for (const ic of list) {
        const [cx, cy] = ctr(ic); let nb = 0;
        for (const o of list) if (o !== ic) { const [ox, oy] = ctr(o); if (Math.hypot(ox - cx, oy - cy) < 220) nb++; }
        if (nb > bn) { bn = nb; seed = ic; }
      }
      const [sx, sy] = ctr(seed);
      const byDist = list.slice().sort((a, b) => { const [ax, ay] = ctr(a), [bx, by] = ctr(b); return Math.hypot(ax - sx, ay - sy) - Math.hypot(bx - sx, by - sy); });
      for (let K = window.__recGroupCount || Math.max(5, Math.min(16, Math.round(list.length * 0.45))); K >= 4; K -= 2) cand.push(byDist.slice(0, K));
    }
    for (const batch of cand) {
      let x0 = 1e9, y0 = 1e9, x1 = -1e9, y1 = -1e9;
      for (const ic of batch) { const p = pos.get(ic); x0 = Math.min(x0, p[0]); y0 = Math.min(y0, p[1]); x1 = Math.max(x1, p[0] + ic.w); y1 = Math.max(y1, p[1] + ic.h); }
      const group = list.filter(ic => { const p = pos.get(ic); return p[0] >= x0 - 1 && p[1] >= y0 - 1 && p[0] + ic.w <= x1 + 1 && p[1] + ic.h <= y1 + 1; });
      const rest = list.filter(ic => !group.includes(ic)).map(ctr);
      const gw = x1 - x0, gh = y1 - y0;
      let best = null;
      for (let ty = 24; ty + gh <= H2 - 24; ty += 24) for (let tx = 24; tx + gw <= W - 24; tx += 24) {
        const dx = tx - x0, dy = ty - y0, d = Math.hypot(dx, dy);
        if (d < 300) continue;
        let md = 1e9;
        for (const [ox, oy] of rest) md = Math.min(md, Math.hypot(Math.max(tx - ox, 0, ox - tx - gw), Math.max(ty - oy, 0, oy - ty - gh)));
        const score = Math.min(md, 300) - d * 0.05 + rnd(tx, ty, 5 + (window.__recSeed || 0)) * 70;
        if (md >= 24 && (!best || score > best.score)) best = { dx, dy, score };
      }
      if (best)
        return { kind: 'group', group, from: group.map(ic => pos.get(ic).slice()), delta: [best.dx, best.dy], seed: batch[0], box: [x0, y0, x1, y1] };
    }
    return null;
  }
  const ease = t => t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
  const clamp01 = t => Math.min(1, Math.max(0, t));
  function drawSelected(ctx, ic, x, y, s, a) {
    if (a <= 0) return;
    ctx.save(); ctx.globalAlpha = a;
    roundRect(ctx, x * s, y * s, ic.w * s, ic.h * s, 4 * s);
    ctx.fillStyle = 'rgba(170, 210, 255, 0.38)'; ctx.fill();
    ctx.lineWidth = Math.max(1, s); ctx.strokeStyle = 'rgba(200, 230, 255, 0.9)'; ctx.stroke();
    ctx.restore();
  }
  function drawCursor(ctx, x, y, s, alpha) {
    if (alpha <= 0) return;
    const k = 1.6 * s, pts = [[0, 0], [0, 16], [4, 12.5], [6.8, 18.6], [9.3, 17.5], [6.6, 11.6], [11.5, 11.6]];
    ctx.save(); ctx.globalAlpha = alpha; ctx.translate(x, y); ctx.scale(k, k);
    ctx.beginPath(); pts.forEach(([px, py], i) => i ? ctx.lineTo(px, py) : ctx.moveTo(px, py)); ctx.closePath();
    ctx.fillStyle = 'rgba(0,0,0,0.25)'; ctx.save(); ctx.translate(0.8, 1.2); ctx.fill(); ctx.restore();
    ctx.fillStyle = '#fff'; ctx.fill(); ctx.lineWidth = 1.1; ctx.strokeStyle = '#111'; ctx.lineJoin = 'round'; ctx.stroke();
    ctx.restore();
  }
  function pickMime() {
    for (const t of ['video/mp4;codecs=avc1.640028', 'video/mp4;codecs=avc1', 'video/mp4', 'video/webm;codecs=vp9', 'video/webm'])
      if (window.MediaRecorder && MediaRecorder.isTypeSupported(t)) return t;
    return null;
  }
  let busy = false;
  function limitedRangeFilter() {
    if (document.getElementById('deskland-limited-range')) return 'url(#deskland-limited-range)';
    const ns = 'http://www.w3.org/2000/svg', svg = document.createElementNS(ns, 'svg');
    svg.setAttribute('width', '0'); svg.setAttribute('height', '0');
    svg.style.cssText = 'position:absolute;width:0;height:0;overflow:hidden';
    const fl = document.createElementNS(ns, 'filter');
    fl.id = 'deskland-limited-range';
    fl.setAttribute('color-interpolation-filters', 'sRGB');
    const ct = document.createElementNS(ns, 'feComponentTransfer');
    for (const c of ['R', 'G', 'B']) {
      const fn = document.createElementNS(ns, 'feFunc' + c);
      fn.setAttribute('type', 'linear'); fn.setAttribute('slope', String(219 / 255)); fn.setAttribute('intercept', String(16 / 255));
      ct.appendChild(fn);
    }
    fl.appendChild(ct); svg.appendChild(fl); document.body.appendChild(svg);
    return 'url(#deskland-limited-range)';
  }
  async function markLimitedRange(blob) {
    const head = new Uint8Array(await blob.slice(0, 8192).arrayBuffer());
    for (let i = 0; i + 15 < head.length; i++) {
      if (head[i] === 0x63 && head[i + 1] === 0x6f && head[i + 2] === 0x6c && head[i + 3] === 0x72 &&       // 'colr'
          head[i + 4] === 0x6e && head[i + 5] === 0x63 && head[i + 6] === 0x6c && head[i + 7] === 0x78) {  // 'nclx'
        head[i + 14] &= 0x7f;
        return new Blob([head, blob.slice(8192)], { type: blob.type });
      }
    }
    return blob;
  }
  function record(onProgress) {
    const mime = pickMime();
    if (!mime) return Promise.reject(new Error('Máy tính này không hỗ trợ quay video'));
    if (busy) return Promise.reject(new Error('Đang quay video trước đó'));
    busy = true;
    const wasPaused = typeof paused !== 'undefined' && paused;
    if (wasPaused) paused = false;
    return new Promise(ok => {
      const t0 = performance.now(), f0 = frameN;
      const tick = () => {
        const dt = performance.now() - t0, ran = frameN - f0;
        if ((ran >= 10 && settled()) || dt > 6000 || (dt > 1000 && ran < 2)) ok(); else setTimeout(tick, 100);
      };
      if (!wasPaused && settled()) ok(); else tick();
    }).then(() => recordNow(onProgress, mime, wasPaused));
  }
  function recordNow(onProgress, mime, wasPaused) {
    const rs = Math.min(window.devicePixelRatio || 1, Math.sqrt((window.__recMaxPixels || 2.0e6) / (W * H)));
    const outW = Math.round(W * rs / 2) * 2, outH = Math.round(H * rs / 2) * 2;
    const old = { w: cv.width, h: cv.height, s: cv._scale };
    const hasLabels = typeof renderLabels === 'function';
    const hasCache = typeof cacheOK !== 'undefined';
    const demo = !WALL && !!window.__recDemoStyle;
    const saved = demo ? icons.map(ic => [ic, ic.x, ic.y]) : [];
    const temp = demo && !icons.length ? { x: W * 0.28, y: (H - TASKBAR) * 0.3, w: 76, h: 92 } : null;
    if (temp) { icons.push(temp); commit(); }
    let moves = [];
    if (demo && !temp) {
      const cx = W / 2, cy = (H - TASKBAR) / 2;
      const hero = icons.reduce((a, b) => Math.hypot(a.x + a.w / 2 - cx, a.y + a.h / 2 - cy) <= Math.hypot(b.x + b.w / 2 - cx, b.y + b.h / 2 - cy) ? a : b);
      moves = planMoves(icons, 1, hero);
    } else if (demo) moves = planMoves(icons, temp ? 1 : 2);
    const HOLD0 = 0.8, DRAG = 1.5, ADAPT = 2.6, HOLD1 = 0.6;
    const GSEL = 1.1, GDROP = 2.8, GADAPT = 2.9;
    const after = new Map(icons.map(ic => [ic, [ic.x, ic.y]]));
    for (const m of moves) after.set(m.ic, m.to);
    let grp = demo && !temp ? planGroup(icons, after) : null;
    if (demo && !temp) {
      const bx0 = GRID_X0 + 2 * CELL_W - 2, bx1 = GRID_X0 + 4 * CELL_W - 2, by1 = GRID_Y0 + 3 * CELL_H - 2;
      const block = icons.filter(ic => ic.x >= bx0 && ic.x < bx1 && ic.y < by1);
      if (block.length) grp = planGroup(icons, after, block);
    }
    if (grp) moves.push(grp);
    let tt = HOLD0;
    for (const m of moves) { m.t0 = tt; m.done = false; tt += m.kind === 'group' ? GDROP + GADAPT : DRAG + ADAPT; }
    const total = demo ? tt + HOLD1 : 12;

    cv.width = outW; cv.height = outH; cv._scale = rs;
    if (hasLabels && labelsReady()) renderLabels();
    if (hasCache) cacheOK = false;
    if (typeof paused !== 'undefined') paused = false;
    const comp = document.createElement('canvas');
    comp.width = outW; comp.height = outH;
    const ctx = comp.getContext('2d');
    const rec = document.createElement('canvas');
    rec.width = outW; rec.height = outH;
    const rctx = rec.getContext('2d');
    const mp4 = mime.indexOf('mp4') >= 0 && !window.__recFullRange;
    if (mp4) rctx.filter = limitedRangeFilter();
    const stream = rec.captureStream(0), track = stream.getVideoTracks()[0];
    const mr = new MediaRecorder(stream, { mimeType: mime, videoBitsPerSecond: window.__recBitrate || 8e6 });
    const chunks = [];
    mr.ondataavailable = e => { if (e.data && e.data.size) chunks.push(e.data); };

    return new Promise((resolve, reject) => {
      const origDraw = draw;
      let t0 = null, lastProg = -1, finished = false, poster = null;
      const finish = err => {
        if (finished) return;
        finished = true;
        draw = origDraw;
        for (const [ic, x, y] of saved) { ic.x = x; ic.y = y; }
        if (temp) icons.splice(icons.indexOf(temp), 1);
        cv.width = old.w; cv.height = old.h; cv._scale = old.s;
        if (hasLabels && labelsReady()) renderLabels();
        if (hasCache) cacheOK = false;
        if (demo) commit();
        if (typeof paused !== 'undefined') paused = hostPause === null ? wasPaused : hostPause;
        busy = false;
        if (err) { try { mr.stop(); } catch (e) { } reject(err); return; }
        mr.onstop = () => {
          const blob = new Blob(chunks, { type: mime.split(';')[0] });
          (mp4 ? markLimitedRange(blob) : Promise.resolve(blob)).then(b => resolve({ blob: b, mime: mime.split(';')[0], poster }));
        };
        mr.stop();
      };
      draw = function (now) {
        origDraw(now);
        if (finished) return;
        if (t0 === null) { t0 = now; mr.start(); }
        const t = (now - t0) / 1000;
        ctx.drawImage(cv, 0, 0, outW, outH);
        const disp = new Map();
        let cursor = null, selRect = null;
        for (const m of moves) {
          const dt = t - m.t0;
          if (m.kind === 'group') {
            if (dt >= GDROP && !m.done) {
              m.done = true;
              m.group.forEach((ic, i) => { ic.x = m.from[i][0] + m.delta[0]; ic.y = m.from[i][1] + m.delta[1]; });
              commit();
            }
            if (dt < 0 || dt > GDROP + 0.8) continue;
            const [bx0, by0, bx1, by1] = m.box, A = [bx0 - 16, by0 - 16], B = [bx1 + 16, by1 + 16];
            const si = m.group.indexOf(m.seed), grab = [m.from[si][0] + m.seed.w * 0.55, m.from[si][1] + m.seed.h * 0.32];
            let cx, cy, off = [0, 0], lift = 0;
            if (dt < GSEL) {
              const p = ease(clamp01((dt - 0.3) / (GSEL - 0.3)));
              cx = A[0] + (B[0] - A[0]) * p; cy = A[1] + (B[1] - A[1]) * p;
              if (dt > 0.3) selRect = [A[0], A[1], cx, cy];
            } else if (dt < 1.5) {
              const p = ease((dt - GSEL) / (1.5 - GSEL));
              cx = B[0] + (grab[0] - B[0]) * p; cy = B[1] + (grab[1] - B[1]) * p;
            } else {
              const p = ease(clamp01((dt - 1.6) / (GDROP - 1.6)));
              off = [m.delta[0] * p, m.delta[1] * p - Math.sin(Math.PI * p) * 30];
              lift = dt < GDROP ? Math.min(1, (dt - 1.5) / 0.12) : 0;
              cx = grab[0] + off[0]; cy = grab[1] + off[1];
            }
            m.group.forEach((ic, i) => {
              const x = dt < GDROP ? m.from[i][0] + off[0] : ic.x, y = dt < GDROP ? m.from[i][1] + off[1] : ic.y;
              let sel;
              if (dt < GSEL) {
                sel = selRect && x < Math.max(selRect[0], selRect[2]) && x + ic.w > Math.min(selRect[0], selRect[2]) &&
                  y < Math.max(selRect[1], selRect[3]) && y + ic.h > Math.min(selRect[1], selRect[3]) ? 1 : 0;
              } else sel = dt < GDROP + 0.2 ? 1 : Math.max(0, 1 - (dt - GDROP - 0.2) / 0.4);
              disp.set(ic, { x, y, lift, sel });
            });
            const alpha = Math.min(1, dt / 0.25) * (dt > GDROP + 0.3 ? Math.max(0, 1 - (dt - GDROP - 0.3) / 0.3) : 1);
            cursor = [cx * rs, cy * rs, alpha];
            continue;
          }
          if (dt >= DRAG && !m.done) { m.done = true; m.ic.x = m.to[0]; m.ic.y = m.to[1]; commit(); }
          if (dt < 0 || dt > DRAG + 0.6) continue;
          const p = ease(clamp01((dt - 0.4) / (DRAG - 0.4)));
          const x = m.from[0] + (m.to[0] - m.from[0]) * p, y = m.from[1] + (m.to[1] - m.from[1]) * p - Math.sin(Math.PI * p) * 24;
          const lift = dt < 0.35 ? 0 : dt < DRAG ? Math.min(1, (dt - 0.35) / 0.12) : 0;
          if (dt < DRAG) disp.set(m.ic, { x, y, lift, sel: 0 });
          const alpha = Math.min(1, dt / 0.25) * (dt > DRAG + 0.3 ? Math.max(0, 1 - (dt - DRAG - 0.3) / 0.3) : 1);
          cursor = [(x + m.ic.w * 0.55) * rs, (y + m.ic.h * 0.32) * rs, alpha];
        }
        for (const ic of icons) if (!disp.has(ic)) drawIcon(ctx, ic, rs, ic.x, ic.y, 0);
        for (const [ic, d] of disp) { drawSelected(ctx, ic, d.x, d.y, rs, d.sel * (1 - 0.4 * d.lift)); drawIcon(ctx, ic, rs, d.x, d.y, d.lift); }
        if (selRect) {
          const [ax, ay, bx, by] = selRect.map(v => v * rs);
          ctx.fillStyle = 'rgba(0, 120, 215, 0.2)'; ctx.fillRect(Math.min(ax, bx), Math.min(ay, by), Math.abs(bx - ax), Math.abs(by - ay));
          ctx.lineWidth = Math.max(1, rs); ctx.strokeStyle = 'rgba(0, 120, 215, 0.9)';
          ctx.strokeRect(Math.min(ax, bx) + 0.5, Math.min(ay, by) + 0.5, Math.abs(bx - ax), Math.abs(by - ay));
        }
        if (cursor) drawCursor(ctx, cursor[0], cursor[1], rs, cursor[2]);
        rctx.drawImage(comp, 0, 0);
        track.requestFrame();
        const prog = Math.min(1, t / total);
        if (onProgress && prog - lastProg >= 0.05) { lastProg = prog; onProgress(prog); }
        if (t >= total) { poster = thumbOf(comp, 520); finish(null); }
      };
      setTimeout(() => finish(new Error('Hình nền không chuyển động (có thể màn hình đang bị che khuất hoàn toàn)')), (total + 8) * 1000);
    });
  }
  window.desklandRecord = record;
  window.desklandRecord.parts = { planMoves, planGroup };

  const host = WALL && window.chrome && window.chrome.webview;
  let hostPause = null;
  function settled() {
    return typeof trees === 'undefined' || !trees.length || trees.every(t => t.grow === t.want);
  }
  function send(cleanOnly) {
    try {
      let savedIcons = null;
      if (cleanOnly && typeof icons !== 'undefined' && Array.isArray(icons) && typeof commit === 'function') {
        savedIcons = icons.slice();
        const W0 = window.innerWidth || W, H0 = window.innerHeight || H;
        const x0 = Math.round(W0 * 0.37), x1 = Math.round(W0 * 0.63);
        const y0 = Math.round(H0 * 0.11), y1 = Math.round(H0 * 0.30);
        const cw = 76, ch = 92;
        const cols = Math.max(1, Math.ceil((x1 - x0) / cw));
        const rows = Math.max(1, Math.ceil((y1 - y0) / ch));
        const sx = cols > 1 ? (x1 - x0 - cw) / (cols - 1) : 0;
        const sy = rows > 1 ? (y1 - y0 - ch) / (rows - 1) : 0;
        for (let r = 0; r < rows; r++)
          for (let c = 0; c < cols; c++)
            icons.push({ x: Math.round(x0 + c * sx), y: Math.round(y0 + r * sy), w: cw, h: ch, _lockWidget: true });
        try { commit(true); } catch (e) {}
      }
      const res = window.desklandSnapshot(undefined, !cleanOnly);
      if (savedIcons) {
        icons.length = 0;
        for (const ic of savedIcons) icons.push(ic);
        try { commit(true); } catch (e) {}
      }
      if (cleanOnly) host.postMessage('locksnap|' + res.canvas.toDataURL('image/png'));
      else host.postMessage('snapshot|' + res.thumb + '|' + res.canvas.toDataURL('image/png'));
    } catch (e) {
      host.postMessage((cleanOnly ? 'locksnap|error|' : 'snapshot|error|') + (e && e.message || e));
    }
  }
  if (host) host.addEventListener('message', ev => {
    let m; try { m = JSON.parse(ev.data); } catch (e) { return; }
    if (!m) return;
    if (m.t === 'pause') { hostPause = !!m.v; return; }
    if (m.t === 'record') {
      record(p => host.postMessage('recprogress|' + p.toFixed(2))).then(r => {
        const fr = new FileReader();
        fr.onload = () => host.postMessage('record|' + r.mime + '|' + r.poster + '|' + fr.result);
        fr.onerror = () => host.postMessage('record|error|Không thể đọc dữ liệu video đã quay');
        fr.readAsDataURL(r.blob);
      }, e => host.postMessage('record|error|' + (e && e.message || e)));
      return;
    }
    if (m.t !== 'snapshot' && m.t !== 'locksnap') return;
    const cleanOnly = m.t === 'locksnap';
    if (busy) { host.postMessage((cleanOnly ? 'locksnap|error|' : 'snapshot|error|') + 'Đang quay hoạt ảnh, vui lòng đợi quay xong rồi thử lại'); return; }
    if (typeof paused === 'undefined' || !paused) { send(cleanOnly); return; }
    paused = false;
    const t0 = performance.now(), f0 = frameN;
    const wait = () => {
      const dt = performance.now() - t0, ran = frameN - f0;
      if ((ran >= 10 && settled()) || dt > 6000 || (dt > 1000 && ran < 2)) {
        send(cleanOnly);
        paused = hostPause === null ? true : hostPause;
      } else setTimeout(wait, 100);
    };
    setTimeout(wait, 100);
  });

  /* ---- Chế độ Màn hình khóa động (Live Lock Screen & Widget Overlay) ---- */
  const lockQS = new URLSearchParams(location.search);
  if (lockQS.has('lockscreen')) {
    const isPrimaryLock = lockQS.get('primary') !== '0';
    const showWidget = lockQS.get('widget') !== '0' && isPrimaryLock;
    const requirePin = lockQS.get('pin') !== '0' && isPrimaryLock;
    const winUser = (lockQS.get('user') || 'WINDOWS').trim();
    let lockLang = (lockQS.get('lang') || 'vi').toLowerCase().startsWith('en') ? 'en' : 'vi';
    let themeLabel = (lockQS.get('themeName') || document.title.replace(/^Deskland\s*[·\-]\s*/i, '') || 'Deskland').trim();

    const style = document.createElement('style');
    style.textContent = `
      @keyframes dlLockFadeIn { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
      @keyframes dlLockPulse { 0%, 100% { opacity: 0.92; } 50% { opacity: 0.58; } }
      @keyframes dlLockShake {
        0%, 100% { transform: translateX(0); }
        20%, 60% { transform: translateX(-8px); }
        40%, 80% { transform: translateX(8px); }
      }
      #dl-lock-overlay {
        position: fixed; inset: 0; z-index: 9999; pointer-events: auto; cursor: ${requirePin ? 'default' : 'none'};
        display: flex; flex-direction: column; align-items: center; justify-content: space-between;
        padding: 7.5vh 24px 34px; user-select: none;
        font-family: "Segoe UI Variable Display", "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
        transition: opacity 0.22s ease;
      }
      #dl-lock-overlay.dl-unlocking { opacity: 0; }
      .dl-lock-center {
        display: flex; flex-direction: column; align-items: center; gap: 22px; width: 100%;
      }
      .dl-lock-card {
        display: flex; flex-direction: column; align-items: center; text-align: center;
        padding: 22px 44px 22px; border-radius: 30px;
        background: rgba(16, 24, 34, 0.26);
        backdrop-filter: blur(18px) saturate(145%);
        -webkit-backdrop-filter: blur(18px) saturate(145%);
        border: 1px solid rgba(255, 255, 255, 0.30);
        box-shadow: 0 22px 54px rgba(0, 0, 0, 0.24), inset 0 1px 0 rgba(255, 255, 255, 0.38);
        color: #ffffff;
        animation: dlLockFadeIn 0.45s cubic-bezier(0.22, 1, 0.36, 1) both;
      }
      .dl-lock-badge {
        display: inline-flex; align-items: center; gap: 7px;
        padding: 5px 14px; border-radius: 999px;
        background: rgba(255, 255, 255, 0.16);
        border: 1px solid rgba(255, 255, 255, 0.24);
        font-size: 12.5px; font-weight: 600; letter-spacing: 0.01em;
        color: rgba(255, 255, 255, 0.96);
        text-shadow: 0 1px 4px rgba(0, 0, 0, 0.22);
      }
      .dl-lock-badge svg { width: 14px; height: 14px; flex: none; }
      .dl-lock-time {
        margin-top: 8px;
        font-size: clamp(64px, 7.6vw, 98px);
        font-weight: 700; line-height: 1.02; letter-spacing: -0.04em;
        font-variant-numeric: tabular-nums;
        text-shadow: 0 6px 28px rgba(0, 0, 0, 0.30);
      }
      .dl-lock-date {
        margin-top: 8px;
        font-size: clamp(14px, 1.55vw, 19px);
        font-weight: 500; letter-spacing: 0.01em;
        color: rgba(255, 255, 255, 0.94);
        text-shadow: 0 2px 12px rgba(0, 0, 0, 0.28);
      }
      /* Khu vực đăng nhập User Windows ngay trên nền động */
      .dl-signin-card {
        display: flex; flex-direction: column; align-items: center; text-align: center;
        padding: 22px 34px 18px; border-radius: 28px;
        background: rgba(16, 24, 34, 0.30);
        backdrop-filter: blur(20px) saturate(150%);
        -webkit-backdrop-filter: blur(20px) saturate(150%);
        border: 1px solid rgba(255, 255, 255, 0.32);
        box-shadow: 0 22px 54px rgba(0, 0, 0, 0.28), inset 0 1px 0 rgba(255, 255, 255, 0.38);
        color: #ffffff;
        animation: dlLockFadeIn 0.55s cubic-bezier(0.22, 1, 0.36, 1) both;
      }
      .dl-user-avatar {
        width: 72px; height: 72px; border-radius: 50%;
        display: grid; place-items: center;
        background: linear-gradient(145deg, rgba(255, 255, 255, 0.28), rgba(255, 255, 255, 0.10));
        border: 1.5px solid rgba(255, 255, 255, 0.48);
        box-shadow: 0 10px 28px rgba(0, 0, 0, 0.24);
      }
      .dl-user-avatar svg { width: 38px; height: 38px; color: rgba(255, 255, 255, 0.95); }
      .dl-user-name {
        margin-top: 10px;
        font-size: 20px; font-weight: 650; letter-spacing: -0.01em;
        color: #ffffff; text-shadow: 0 2px 10px rgba(0, 0, 0, 0.35);
      }
      .dl-pin-form {
        margin-top: 14px; display: flex; align-items: center;
        width: 276px; height: 40px; border-radius: 10px;
        border: 1px solid rgba(255, 255, 255, 0.38);
        border-bottom: 2px solid rgba(255, 255, 255, 0.72);
        background: rgba(12, 18, 26, 0.46);
        overflow: hidden; transition: border-color 0.15s, background 0.15s;
      }
      .dl-pin-form:focus-within {
        background: rgba(12, 18, 26, 0.62);
        border-color: rgba(255, 255, 255, 0.65);
        border-bottom-color: #60cdff;
      }
      .dl-pin-form.dl-shake { animation: dlLockShake 0.36s ease; }
      .dl-pin-input {
        flex: 1; min-width: 0; height: 100%; padding: 0 12px; border: 0;
        background: transparent; color: #fff;
        font: 500 14px/1.2 "Segoe UI Variable Text", "Segoe UI", sans-serif;
        outline: none;
      }
      .dl-pin-input::placeholder { color: rgba(255, 255, 255, 0.68); font-weight: 400; font-size: 13px; }
      .dl-pin-icon-btn {
        width: 34px; height: 100%; border: 0; background: transparent;
        color: rgba(255, 255, 255, 0.85); display: grid; place-items: center;
        cursor: pointer; transition: background 0.15s, color 0.15s;
      }
      .dl-pin-icon-btn:hover { background: rgba(255, 255, 255, 0.16); color: #fff; }
      .dl-pin-icon-btn svg { width: 16px; height: 16px; }
      .dl-pin-err {
        margin-top: 8px; min-height: 17px; max-width: 280px; font-size: 12px; font-weight: 500;
        color: #ffd2d2; text-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
      }
      .dl-lock-hint {
        display: inline-flex; align-items: center; gap: 8px;
        padding: 9px 20px; border-radius: 999px;
        background: rgba(16, 24, 34, 0.32);
        backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
        border: 1px solid rgba(255, 255, 255, 0.24);
        color: rgba(255, 255, 255, 0.92);
        font-size: 13px; font-weight: 500;
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.20);
        animation: dlLockPulse 3.6s ease-in-out infinite;
      }
      .dl-lock-hint svg { width: 14px; height: 14px; flex: none; opacity: 0.9; }
    `;
    document.head.appendChild(style);

    const overlay = document.createElement('div');
    overlay.id = 'dl-lock-overlay';
    const centerWrap = document.createElement('div');
    centerWrap.className = 'dl-lock-center';
    overlay.appendChild(centerWrap);

    let cardEl = null, signInEl = null, badgeEl = null, timeEl = null, dateEl = null, hintTextEl = null;
    let pinFormEl = null, pinInputEl = null, pinEyeEl = null, pinErrEl = null;

    if (showWidget) {
      cardEl = document.createElement('div');
      cardEl.className = 'dl-lock-card';
      cardEl.innerHTML = `
        <div class="dl-lock-badge" id="dl-lock-badge"></div>
        <div class="dl-lock-time" id="dl-lock-time">00:00</div>
        <div class="dl-lock-date" id="dl-lock-date"></div>
      `;
      centerWrap.appendChild(cardEl);
      badgeEl = cardEl.querySelector('#dl-lock-badge');
      timeEl = cardEl.querySelector('#dl-lock-time');
      dateEl = cardEl.querySelector('#dl-lock-date');
    }

    if (requirePin) {
      const escUser = winUser.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
      signInEl = document.createElement('div');
      signInEl.className = 'dl-signin-card';
      signInEl.innerHTML = `
        <div class="dl-user-avatar" aria-hidden="true">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="8" r="4.2"/>
            <path d="M4.5 20.2c1.6-3.8 4.4-5.7 7.5-5.7s5.9 1.9 7.5 5.7"/>
          </svg>
        </div>
        <div class="dl-user-name">${escUser}</div>
        <form class="dl-pin-form" id="dl-pin-form" autocomplete="off">
          <input type="password" class="dl-pin-input" id="dl-pin-input" placeholder="${lockLang === 'en' ? 'Password or PIN' : 'Mật khẩu hoặc mã PIN'}" maxlength="64">
          <button type="button" class="dl-pin-icon-btn" id="dl-pin-eye" title="${lockLang === 'en' ? 'Show password' : 'Hiện mật khẩu'}" tabindex="-1">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"><path d="M1.2 8s2.5-4.4 6.8-4.4S14.8 8 14.8 8s-2.5 4.4-6.8 4.4S1.2 8 1.2 8z"/><circle cx="8" cy="8" r="2.1"/></svg>
          </button>
          <button type="submit" class="dl-pin-icon-btn" id="dl-pin-btn" title="${lockLang === 'en' ? 'Sign in' : 'Đăng nhập'}">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M3 8h10M9 4l4 4-4 4"/></svg>
          </button>
        </form>
        <div class="dl-pin-err" id="dl-pin-err"></div>
      `;
      centerWrap.appendChild(signInEl);
      pinFormEl = signInEl.querySelector('#dl-pin-form');
      pinInputEl = signInEl.querySelector('#dl-pin-input');
      pinEyeEl = signInEl.querySelector('#dl-pin-eye');
      pinErrEl = signInEl.querySelector('#dl-pin-err');
    }

    if (isPrimaryLock) {
      const hintEl = document.createElement('div');
      hintEl.className = 'dl-lock-hint';
      hintEl.innerHTML = `<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3.2" y="7" width="9.6" height="6.8" rx="1.6"/><path d="M5.2 7V5a2.8 2.8 0 0 1 5.6 0v2"/></svg><span id="dl-lock-hint-text"></span>`;
      overlay.appendChild(hintEl);
      hintTextEl = hintEl.querySelector('#dl-lock-hint-text');
    } else {
      overlay.appendChild(document.createElement('div'));
    }
    document.body.appendChild(overlay);

    function greetingForHour(h) {
      if (lockLang === 'en') {
        if (h >= 5 && h < 12) return 'Good morning';
        if (h >= 12 && h < 18) return 'Good afternoon';
        if (h >= 18 && h < 22) return 'Good evening';
        return 'Peaceful night';
      }
      if (h >= 5 && h < 11) return 'Chào buổi sáng';
      if (h >= 11 && h < 13) return 'Chào buổi trưa';
      if (h >= 13 && h < 18) return 'Chào buổi chiều';
      if (h >= 18 && h < 22) return 'Chào buổi tối';
      return 'Đêm an lành';
    }

    function updateClock() {
      if (hintTextEl) {
        if (requirePin) {
          hintTextEl.textContent = lockLang === 'en'
            ? 'Enter your Windows password or Deskland PIN to sign in'
            : 'Nhập mật khẩu Windows hoặc mã PIN Deskland để mở khóa';
        } else {
          hintTextEl.textContent = lockLang === 'en'
            ? 'Move mouse or press any key to unlock'
            : 'Di chuột hoặc nhấn phím bất kỳ để mở khóa';
        }
      }
      if (!showWidget || !timeEl) return;
      const now = new Date();
      const hh = String(now.getHours()).padStart(2, '0');
      const mm = String(now.getMinutes()).padStart(2, '0');
      timeEl.textContent = hh + ':' + mm;
      const isNight = now.getHours() < 6 || now.getHours() >= 18;
      const iconSvg = isNight
        ? '<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M13.2 9.8A5.5 5.5 0 0 1 6.2 2.8a5.5 5.5 0 1 0 7 7z"/></svg>'
        : '<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="8" r="3"/><path d="M8 1.8v1.5M8 12.7v1.5M1.8 8h1.5M12.7 8h1.5M3.6 3.6l1.1 1.1M11.3 11.3l1.1 1.1M12.4 3.6l-1.1 1.1M4.7 11.3l-1.1 1.1"/></svg>';
      badgeEl.innerHTML = iconSvg + '<span>' + greetingForHour(now.getHours()) + (themeLabel ? ' · ' + themeLabel : '') + '</span>';
      try {
        const locale = lockLang === 'en' ? 'en-US' : 'vi-VN';
        let ds = new Intl.DateTimeFormat(locale, { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' }).format(now);
        if (ds && ds.length > 1) ds = ds.charAt(0).toUpperCase() + ds.slice(1);
        dateEl.textContent = ds;
      } catch (e) {
        dateEl.textContent = now.toLocaleDateString();
      }
    }
    updateClock();
    setInterval(updateClock, 1000);

    // Đưa khung chữ nhật của Widget Đồng hồ và Khu vực nhập mật khẩu vào trường khoảng cách (icons) để cảnh vật uốn lượn bao quanh
    function addRectToIslands(el) {
      if (!el) return;
      const rect = el.getBoundingClientRect();
      if (rect.width <= 20 || rect.height <= 20) return;
      const padX = 14, padY = 10;
      const x0 = Math.max(8, rect.left - padX), y0 = Math.max(8, rect.top - padY);
      const x1 = Math.min(window.innerWidth - 8, rect.right + padX), y1 = Math.min(window.innerHeight - 8, rect.bottom + padY);
      const cw = 76, ch = 92;
      const cols = Math.max(1, Math.ceil((x1 - x0) / cw));
      const rows = Math.max(1, Math.ceil((y1 - y0) / ch));
      const stepX = cols > 1 ? (x1 - x0 - cw) / (cols - 1) : 0;
      const stepY = rows > 1 ? (y1 - y0 - ch) / (rows - 1) : 0;
      for (let r = 0; r < rows; r++) {
        for (let c = 0; c < cols; c++) {
          icons.push({ x: Math.round(x0 + c * stepX), y: Math.round(y0 + r * stepY), w: cw, h: ch, _lockWidget: true });
        }
      }
    }

    function syncLockWidgetIslands() {
      if (typeof icons === 'undefined' || !Array.isArray(icons)) return;
      for (let i = icons.length - 1; i >= 0; i--) {
        if (icons[i] && icons[i]._lockWidget) icons.splice(i, 1);
      }
      const hasUserIcons = icons.length > 0;
      if (showWidget && cardEl) addRectToIslands(cardEl);
      if (requirePin && signInEl) addRectToIslands(signInEl);
      if (!hasUserIcons) {
        const W0 = window.innerWidth, H0 = window.innerHeight;
        const scenic = [
          [0.12, 0.58], [0.16, 0.64], [0.82, 0.56], [0.86, 0.63], [0.20, 0.24], [0.78, 0.26]
        ];
        for (const [fx, fy] of scenic) {
          icons.push({ x: Math.round(W0 * fx - 38), y: Math.round(H0 * fy - 46), w: 76, h: 92, _lockWidget: true });
        }
      }
    }

    if (typeof commit === 'function') {
      const origCommit = commit;
      commit = function (fromEmpty) {
        syncLockWidgetIslands();
        return origCommit(fromEmpty);
      };
      setTimeout(() => { try { commit(true); } catch (e) {} }, 60);
    }

    if (requirePin && pinFormEl && pinInputEl) {
      setTimeout(() => { try { pinInputEl.focus(); } catch (e) {} }, 220);
      if (pinEyeEl) {
        pinEyeEl.addEventListener('click', e => {
          e.preventDefault();
          pinInputEl.type = pinInputEl.type === 'password' ? 'text' : 'password';
          pinInputEl.focus();
        });
      }
      window.addEventListener('mousedown', e => {
        if (e.target !== pinInputEl && !e.target.closest('.dl-pin-icon-btn')) {
          setTimeout(() => { try { pinInputEl.focus(); } catch (err) {} }, 10);
        }
      });
      window.addEventListener('keydown', () => {
        if (document.activeElement !== pinInputEl) {
          try { pinInputEl.focus(); } catch (err) {}
        }
      });
      pinFormEl.addEventListener('submit', e => {
        e.preventDefault();
        const val = pinInputEl.value || '';
        if (host) host.postMessage('unlockPin|' + val);
      });
      if (host) {
        host.addEventListener('message', ev => {
          let m; try { m = JSON.parse(ev.data); } catch (e) { return; }
          if (m && m.t === 'pinError') {
            pinFormEl.classList.remove('dl-shake');
            void pinFormEl.offsetWidth;
            pinFormEl.classList.add('dl-shake');
            pinInputEl.value = '';
            pinInputEl.focus();
            if (pinErrEl) {
              pinErrEl.textContent = lockLang === 'en'
                ? 'Incorrect password or PIN. Please try again.'
                : 'Mật khẩu hoặc mã PIN không đúng, vui lòng thử lại.';
            }
          }
        });
      }
    } else {
      // Bắt tương tác người dùng để mở khóa (sau khoảng trễ 650ms tránh nhận nhầm phím/chuột lúc vừa kích hoạt)
      const lockOpenedAt = performance.now();
      let unlocked = false;
      let baseMouse = null;
      function triggerUnlock() {
        if (unlocked || performance.now() - lockOpenedAt < 650) return;
        unlocked = true;
        overlay.classList.add('dl-unlocking');
        if (host) {
          setTimeout(() => { try { host.postMessage('unlock'); } catch (e) {} }, 120);
        }
      }
      window.addEventListener('keydown', triggerUnlock, true);
      window.addEventListener('mousedown', triggerUnlock, true);
      window.addEventListener('wheel', triggerUnlock, { capture: true, passive: true });
      window.addEventListener('touchstart', triggerUnlock, { capture: true, passive: true });
      window.addEventListener('mousemove', e => {
        if (performance.now() - lockOpenedAt < 650) return;
        if (!baseMouse) { baseMouse = [e.screenX, e.screenY]; return; }
        if (Math.hypot(e.screenX - baseMouse[0], e.screenY - baseMouse[1]) > 48) triggerUnlock();
      }, true);
    }
  }
})();
