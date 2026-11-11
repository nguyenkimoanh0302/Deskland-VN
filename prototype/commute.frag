#version 300 es
// Deskland · Chuyến xe chiều - phong cảnh nhìn ngang kiểu hoạt hình: trời đổi màu theo giờ trong ngày,
// phố bên sông, cầu vòm, con đường có xe buýt, lau sậy tiền cảnh, đồi cỏ ôm biểu tượng.
// Dùng chung cho prototype/commute.html (WebGL2) và deskland-rs (include_str!). p tính theo px CSS.
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uDebug;
uniform float uTod;        // giờ trong ngày 0..1 (0 = nửa đêm, 0.5 = trưa)
uniform float uWind, uReeds, uLights;
uniform vec4 uBus;         // x giữa xe, hướng (+1 sang phải / -1), góc quay bánh, độ hiện
uniform vec4 uBusB;        // cửa mở 0..1, tuổi tiếng còi (s, <0 không có), phanh 0..1, 0
uniform vec4 uStop;        // x trạm dừng, độ hiện, 0, 0
uniform vec4 uGust;        // x, y, tuổi (s), cường độ - gió rạp lau khi bấm vào cỏ
uniform float uPartN;
uniform vec4 uPart[24];    // x, y, pha, loại (0 chim, 1 hạt bồ công anh, 2 đom đóm)
uniform vec4 uStar;        // x, y, tuổi (s, <0 không có), góc - sao băng
out vec4 o;

const float TASKBAR = 48.0;
const float PI = 3.14159265;

float hash1(float n) { return fract(sin(n * 127.1 + 311.7) * 43758.5453); }
float hash2(vec2 p) { vec3 p3 = fract(vec3(p.xyx) * 0.1031); p3 += dot(p3, p3.yzx + 33.33); return fract((p3.x + p3.y) * p3.z); }
float ss(float a, float b, float x) { float t = clamp((x - a) / (b - a), 0.0, 1.0); return t * t * (3.0 - 2.0 * t); }
vec4 N(vec2 p, float scale) { return texture(uNoise, p / scale + uSeed * 0.013); }
float fieldS(sampler2D t, vec2 p) { return texture(t, (p / uStep + 0.5) / uGrid).r; }
float segD(vec2 p, vec2 a, vec2 b) { vec2 pa = p - a, ba = b - a; return length(pa - ba * clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0)); }
float boxD(vec2 p, vec2 c, vec2 h, float r) { vec2 d = abs(p - c) - h + r; return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0) - r; }

// ---- Bố cục (theo chiều cao vùng làm việc, trên thanh tác vụ) ----
float HU() { return uRes.y - TASKBAR; }
float yRiver() { return HU() * 0.56; }     // mép bờ xa / mặt sông
float yRiverB() { return HU() * 0.645; }   // mép bờ gần
float yRoad() { return HU() * 0.715; }     // mép trên mặt đường
float yWheel() { return HU() * 0.790; }    // vạch bánh xe

// ---- Ánh sáng theo giờ: trọng số đêm / vàng (bình minh, hoàng hôn) / ngày ----
vec3 wts() {
  float t = uTod;
  float night = clamp(1.0 - ss(0.21, 0.29, t) + ss(0.79, 0.87, t), 0.0, 1.0);
  float gold = ss(0.65, 0.72, t) * (1.0 - ss(0.77, 0.84, t)) + 0.7 * ss(0.20, 0.26, t) * (1.0 - ss(0.28, 0.35, t));
  gold = min(gold, 1.0 - night * 0.5);
  float day = max(1.0 - night - gold, 0.0);
  float s = night + gold + day;
  return vec3(night, gold, day) / s;
}
vec3 pal(vec3 w, vec3 n, vec3 g, vec3 d) { return n * w.x + g * w.y + d * w.z; }

// Mặt trời / mặt trăng: cung từ trái sang phải phía trên đường chân trời
vec3 orb(float t) {
  float a = (t - 0.25) / 0.5 * PI;                  // 0 lúc 6h, PI lúc 18h
  float x = uRes.x * (0.08 + 0.84 * clamp(a / PI, 0.0, 1.0));
  float y = yRiver() - sin(clamp(a, 0.0, PI)) * yRiver() * 0.80 + 30.0;
  return vec3(x, y, step(0.0, a) * step(a, PI));
}

// Lá lau / cỏ: trả về (độ phủ, độ cao tương đối tt, có bông lau)
vec3 reed(vec2 p, float base, float cw, float hmin, float hmax, float dens, float k) {
  vec3 best = vec3(0.0);
  float c0 = floor(p.x / cw);
  for (int j = -2; j <= 2; j++) {
    float c = c0 + float(j);
    float hh = hash1(c * 1.31 + k);
    if (hh > dens) continue;
    float h = mix(hmin, hmax, hash1(c * 7.7 + k + 3.0)) * (0.75 + 0.5 * N(vec2(c * cw, k * 50.0), 900.0).r);
    float tt = (base - p.y) / h;
    if (tt < 0.0 || tt > 1.12) continue;
    float bx = (c + 0.5) * cw + (hash1(c * 3.3 + k) - 0.5) * cw * 0.8;
    // Gió: dao động chung + cơn gió rạp khi bấm (sóng lan từ điểm bấm)
    float sway = uWind * (sin(uTime * 1.5 + bx * 0.013 + k) * 0.6 + sin(uTime * 2.7 + bx * 0.031) * 0.25) * 14.0;
    if (uGust.z >= 0.0) {
      float dd = abs(bx - uGust.x);
      float front = uGust.z * 420.0;
      sway += uGust.w * 26.0 * exp(-uGust.z * 1.4) * exp(-pow((dd - front) / 90.0, 2.0)) * sign(bx - uGust.x + 0.01);
    }
    float cx = bx + sway * pow(clamp(tt, 0.0, 1.0), 1.6) + (hash1(c + k) - 0.5) * 18.0 * tt * tt;
    float wdt = mix(3.0, 0.6, clamp(tt, 0.0, 1.0));
    float d = abs(p.x - cx) - wdt;
    float plume = 0.0;
    if (hash1(c * 5.1 + k) < 0.55) {
      // Bông lau ở ngọn: hình giọt dài, nghiêng theo gió
      vec2 tip = vec2(bx + sway, base - h);
      vec2 q = p - tip - vec2(sway * 0.15, 10.0);
      plume = 1.0 - ss(0.6, 1.0, length(q / vec2(6.0 + 2.0 * hash1(c), 17.0)));
    }
    float cov = max((1.0 - ss(-0.4, 0.8, d)) * step(tt, 1.0), plume);
    if (cov > best.x) best = vec3(cov, clamp(tt, 0.0, 1.0), plume);
  }
  return best;
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y * uScale - gl_FragCoord.y) / uScale;
  float t = uTime;
  vec3 w = wts();
  float night = w.x, gold = w.y;
  float lit = ss(0.35, 0.95, night) * uLights;         // đèn phố bật
  vec3 amb = pal(w, vec3(0.34, 0.36, 0.58), vec3(1.0, 0.80, 0.68), vec3(1.0));
  vec3 gamb = mix(amb, vec3(1.0), 0.35) * mix(1.0, 0.8, night);   // ánh sáng mặt đất (ít nhuộm hơn)
  vec3 haze = pal(w, vec3(0.16, 0.16, 0.32), vec3(0.98, 0.70, 0.55), vec3(0.72, 0.84, 0.93));

  // ---------- Bầu trời ----------
  float yr = yRiver();
  float sk = clamp(p.y / yr, 0.0, 1.0);
  vec3 skyTop = pal(w, vec3(0.04, 0.05, 0.15), vec3(0.36, 0.36, 0.66), vec3(0.30, 0.60, 0.90));
  vec3 skyBot = pal(w, vec3(0.20, 0.17, 0.36), vec3(1.00, 0.64, 0.42), vec3(0.74, 0.88, 0.96));
  vec3 col = mix(skyTop, skyBot, pow(sk, 1.3));
  // Sao đêm
  if (night > 0.05) {
    vec2 sc = floor(p / 3.0);
    float st = step(0.9975, hash2(sc + 7.0)) * (0.6 + 0.4 * sin(t * 2.0 + hash2(sc) * 30.0));
    col += vec3(1.0, 0.97, 0.9) * st * night * (1.0 - sk);
  }
  // Mặt trời + quầng sáng / mặt trăng khuyết
  vec3 sun = orb(uTod), moon = orb(fract(uTod + 0.5));
  if (sun.z > 0.5) {
    float d = length(p - sun.xy);
    vec3 sc = mix(vec3(1.0, 0.97, 0.80), vec3(1.0, 0.72, 0.40), gold);
    col += sc * 0.35 * exp(-d / 140.0) * (1.0 - night);
    col = mix(col, sc, 1.0 - ss(26.0, 28.0, d));
  }
  if (moon.z > 0.5) {
    float d = length(p - moon.xy), d2 = length(p - moon.xy - vec2(7.0, -5.0));
    col += vec3(0.6, 0.65, 0.9) * 0.18 * exp(-d / 90.0) * night;
    col = mix(col, vec3(0.97, 0.95, 0.85), (1.0 - ss(15.0, 16.5, d)) * ss(14.0, 15.5, d2) * night);
  }
  // Mây hoạt hình: hai tông sáng / tối, trôi theo gió
  if (p.y < yr * 0.78) {
    vec2 cp = vec2(p.x + t * 6.0 * (0.4 + uWind), p.y * 2.3);
    float c1 = N(cp, 1100.0).r * 0.65 + N(cp + 333.0, 420.0).g * 0.35;
    float band = ss(0.05, 0.25, p.y / yr) * (1.0 - ss(0.45, 0.78, p.y / yr));
    float cl = ss(0.585, 0.62, c1) * band;
    float under = ss(0.585, 0.62, N(cp + vec2(0.0, 26.0), 1100.0).r * 0.65 + N(cp + vec2(333.0, 359.0), 420.0).g * 0.35) * band;
    vec3 cLit = pal(w, vec3(0.30, 0.30, 0.48), vec3(1.0, 0.86, 0.76), vec3(1.0, 1.0, 1.0));
    vec3 cSh = pal(w, vec3(0.17, 0.16, 0.30), vec3(0.88, 0.56, 0.55), vec3(0.80, 0.88, 0.97));
    col = mix(col, mix(cSh, cLit, 1.0 - under * 0.85), cl * 0.95);
  }

  // ---------- Núi xa (2 lớp, chìm trong khí quyển) ----------
  float m1 = yr - 50.0 - 45.0 * N(vec2(p.x * 0.5, 10.0), 900.0).r - 30.0 * sin(p.x * 0.0021 + uSeed);
  float m2 = yr - 22.0 - 34.0 * N(vec2(p.x * 0.9, 90.0), 700.0).g - 16.0 * sin(p.x * 0.004 + uSeed * 2.0);
  col = mix(col, mix(haze, vec3(0.42, 0.55, 0.72) * amb, 0.45), ss(-0.8, 0.8, p.y - m1) * step(p.y, yr));
  col = mix(col, mix(haze, vec3(0.30, 0.44, 0.52) * amb, 0.62), ss(-0.8, 0.8, p.y - m2) * step(p.y, yr));

  // ---------- Phố bên kia sông ----------
  {
    float cwid = 30.0;
    float ci = floor(p.x / cwid);
    float cluster = ss(0.30, 0.62, N(vec2(ci * cwid, 40.0), 1600.0).b);
    float bh = (25.0 + 150.0 * hash1(ci + uSeed) * hash1(ci * 2.1 + 5.0)) * (0.25 + cluster);
    float bw = cwid * (0.62 + 0.38 * hash1(ci * 4.7));
    float bx0 = ci * cwid + (cwid - bw) * 0.5;
    if (p.y < yr && p.y > yr - bh && p.x > bx0 && p.x < bx0 + bw) {
      vec3 bc = mix(haze, vec3(0.45, 0.52, 0.62) * amb, 0.55 + 0.1 * hash1(ci * 9.0));
      bc *= 1.0 - 0.10 * ss(bx0 + bw * 0.6, bx0 + bw, p.x);          // mặt bên tối hơn
      // Cửa sổ
      vec2 wq = vec2(p.x - bx0, yr - p.y);
      vec2 wc = floor(wq / vec2(5.0, 7.0));
      vec2 wf = fract(wq / vec2(5.0, 7.0));
      float win = step(0.25, wf.x) * step(wf.x, 0.75) * step(0.3, wf.y) * step(wf.y, 0.8) * step(4.0, wq.x) * step(wq.x, bw - 4.0) * step(wq.y, bh - 6.0);
      float on = step(0.55, hash2(wc + ci * 13.0 + floor(t * 0.05 + hash1(ci) * 9.0)));
      bc = mix(bc, mix(bc * 1.15, vec3(1.0, 0.82, 0.45), lit * on), win * (0.35 + 0.65 * lit));
      col = bc;
    }
  }

  // ---------- Cầu vòm (bên phải) ----------
  float bxL = uRes.x * 0.58;
  if (p.x > bxL && p.y < yr + 2.0 && p.y > yr - 64.0) {
    float deck = yr - 16.0;
    vec3 brc = mix(haze, vec3(0.35, 0.33, 0.38) * amb, 0.7);
    float span = 96.0;
    float lx = mod(p.x - bxL, span) - span * 0.5;
    float pier = step(abs(mod(p.x - bxL + span * 0.5, span) - span * 0.5), 3.5) * step(deck, p.y);
    float archY = deck - 32.0 * (1.0 - pow(lx / (span * 0.5), 2.0));
    float arch = (1.0 - ss(0.0, 2.2, abs(p.y - archY))) * step(p.y, deck);
    float hang = step(abs(fract((p.x - bxL) / 12.0) - 0.5), 0.06) * step(archY, p.y) * step(p.y, deck);
    float dk = step(abs(p.y - deck), 2.4);
    float m = max(max(pier, arch), max(dk, hang * 0.6));
    col = mix(col, brc, m);
    // Đèn trên vòm cầu (đêm)
    float lamps = (1.0 - ss(0.0, 2.6, length(vec2(fract((p.x - bxL) / 16.0) - 0.5, 0.0) * 16.0 + vec2(0.0, p.y - archY + 2.0))));
    col += vec3(1.0, 0.62, 0.30) * lamps * lit * 1.2;
    col += vec3(1.0, 0.55, 0.25) * 0.10 * lit * exp(-abs(p.y - archY) / 10.0) * step(p.y, deck);
  }

  // ---------- Sông: phản chiếu trời, lấp lánh, vệt đèn ----------
  float yrb = yRiverB();
  if (p.y >= yr && p.y < yrb) {
    float rk = (p.y - yr) / (yrb - yr);
    vec3 wc = mix(skyBot, skyTop, rk * 0.6) * mix(vec3(0.75, 0.82, 0.92), vec3(0.6, 0.6, 0.8), night);
    float sh = N(vec2(p.x * 0.4 + t * 9.0, p.y * 6.0), 300.0).r;
    wc *= 0.92 + 0.12 * sh;
    // Vệt nắng / trăng trên mặt nước
    vec3 o2 = sun.z > 0.5 ? sun : moon;
    float glit = exp(-pow((p.x - o2.x) / (40.0 + rk * 50.0), 2.0)) * step(0.6, fract(sh * 7.0 + p.y * 0.3 - t * 0.6));
    wc += mix(vec3(1.0, 0.85, 0.6), vec3(0.8, 0.85, 1.0), night) * glit * 0.45 * (1.0 - rk * 0.6);
    // Vệt đèn phố + đèn cầu phản chiếu ban đêm
    float ci = floor(p.x / 30.0);
    float refl = step(0.5, hash1(ci * 3.7 + floor(p.x / 6.0))) * step(fract(p.y * 0.35 + sh * 3.0 - t * 0.3), 0.55);
    float bridgeR = step(bxL, p.x) * step(fract((p.x - bxL) / 16.0), 0.25);
    wc += vec3(1.0, 0.70, 0.35) * max(refl * 0.35, bridgeR * 0.6) * lit * (1.0 - rk) * 0.8;
    col = mix(wc, haze, 0.15 * (1.0 - night));
    col = mix(col, col * 0.75, 1.0 - ss(0.0, 3.0, p.y - yr));    // bóng mép bờ xa
  }

  // ---------- Bờ gần: hàng cây ----------
  float yrd = yRoad();
  if (p.y >= yrb - 60.0 && p.y < yrd + 2.0) {
    float tl = yrb - 8.0 - 46.0 * ss(0.25, 0.75, N(vec2(p.x * 0.8, 200.0), 500.0).r) - 14.0 * N(vec2(p.x * 3.0, 260.0), 180.0).g;
    float tree = 1.0 - ss(-0.8, 0.8, tl - p.y);
    float bank = step(yrb, p.y);
    vec3 tc = vec3(0.20, 0.42, 0.24) * amb * (0.8 + 0.3 * N(p * vec2(1.0, 1.6), 120.0).g);
    tc = mix(tc, tc * 1.35 + vec3(0.10, 0.05, 0.0) * gold, ss(12.0, 0.0, p.y - tl) * (1.0 - night));
    col = mix(col, tc, max(tree * step(p.y, yrb + 4.0), 0.0));
    col = mix(col, vec3(0.30, 0.52, 0.24) * gamb * (0.9 + 0.2 * N(p, 200.0).r), bank);
  }

  // ---------- Đường, lan can, đèn đường, trạm dừng ----------
  float yw = yWheel();
  float roadB = yw + 10.0;
  if (p.y >= yrd && p.y < roadB + 8.0) {
    vec3 rc = vec3(0.26, 0.27, 0.31) * gamb;
    rc *= 0.94 + 0.08 * N(p * 2.0, 80.0).r;
    float lane = step(abs(p.y - (yrd + roadB) * 0.5 - 2.0), 1.3) * step(0.5, fract(p.x / 44.0));
    rc = mix(rc, vec3(0.95, 0.93, 0.85) * gamb, lane * 0.85);
    rc = mix(rc, vec3(0.92, 0.92, 0.90) * gamb, step(abs(p.y - yrd - 2.0), 1.2) + step(abs(p.y - roadB + 2.0), 1.2));  // vạch mép
    rc = mix(rc, vec3(0.66, 0.65, 0.62) * gamb, step(roadB, p.y));              // lề đường
    col = rc;
    col = mix(col, col * 0.8, 1.0 - ss(0.0, 5.0, p.y - yrd));
  }
  // Bãi cỏ phía dưới đường
  if (p.y >= roadB + 6.0) {
    float gk = (p.y - roadB) / max(uRes.y - roadB, 1.0);
    vec3 gc = pal(w, vec3(0.12, 0.17, 0.20), vec3(0.46, 0.56, 0.26), vec3(0.38, 0.58, 0.24));
    col = gc * (1.0 - 0.35 * gk) * (0.9 + 0.2 * N(p * vec2(1.0, 3.0), 160.0).r);
  }
  // Lan can phía sông
  float rail = step(abs(p.y - (yrd - 12.0)), 1.3) + step(abs(p.y - (yrd - 4.0)), 1.0);
  rail += step(abs(fract(p.x / 26.0) - 0.5) * 26.0, 1.3) * step(yrd - 13.0, p.y) * step(p.y, yrd);
  col = mix(col, vec3(0.80, 0.82, 0.84) * amb, min(rail, 1.0) * step(p.y, yrd + 0.5));
  // Đèn đường: cột cong, quầng sáng ban đêm
  {
    float sp = 380.0;
    float lx = mod(p.x + 120.0, sp) - sp * 0.5;
    float top = yrd - 150.0;
    float pole = step(abs(lx), 2.2) * step(top, p.y) * step(p.y, yrd);
    float arm = (1.0 - ss(1.2, 2.2, abs(length(vec2(lx - 14.0, p.y - top)) - 14.0))) * step(p.y, top) * step(lx, 14.0);
    float head = 1.0 - ss(0.0, 1.5, boxD(vec2(lx, p.y), vec2(28.0, top - 13.0), vec2(9.0, 3.0), 2.0));
    col = mix(col, vec3(0.22, 0.24, 0.27) * amb + 0.1, max(max(pole, arm), head));
    vec2 lp = vec2(lx - 28.0, p.y - (top - 9.0));
    col += vec3(1.0, 0.85, 0.55) * lit * (exp(-length(lp) / 18.0) * 0.8 + 0.35 * exp(-length(lp * vec2(1.0, 0.25)) / 40.0) * step(0.0, lp.y));
  }
  // Trạm dừng xe buýt
  if (uStop.y > 0.01) {
    float sx = uStop.x;
    float pole = step(abs(p.x - sx), 1.8) * step(yrd - 98.0, p.y) * step(p.y, yrd + 2.0);
    vec2 sp = p - vec2(sx, yrd - 98.0);
    float plate = 1.0 - ss(0.0, 1.2, boxD(sp, vec2(0.0), vec2(20.0, 14.0), 3.0));
    float icon = 1.0 - ss(0.0, 1.0, boxD(sp, vec2(0.0, -1.0), vec2(11.0, 6.5), 2.0));
    icon = max(icon, 1.0 - ss(0.0, 1.0, min(length(sp - vec2(-6.0, 7.0)), length(sp - vec2(6.0, 7.0))) - 2.4));
    float winS = 1.0 - ss(0.0, 0.8, boxD(sp, vec2(0.0, -3.0), vec2(8.5, 2.2), 0.5));
    col = mix(col, vec3(0.30) * gamb, pole * uStop.y);
    vec3 sc = mix(vec3(0.16, 0.42, 0.78), vec3(0.97), icon);
    sc = mix(sc, vec3(0.16, 0.42, 0.78), winS);
    col = mix(col, sc * gamb, plate * uStop.y);
  }

  // ---------- Xe buýt ----------
  if (uBus.w > 0.01 && abs(p.x - uBus.x) < 140.0 && p.y > yw - 100.0 && p.y < yw + 30.0) {
    float dir = uBus.y;
    vec2 q = vec2((p.x - uBus.x) * dir, p.y - yw);   // q.x > 0 là phía đầu xe
    float L = 108.0, Ht = 70.0;
    float body = boxD(q, vec2(0.0, -Ht * 0.5 - 6.0), vec2(L, Ht * 0.5), 10.0);
    // Bóng dưới gầm
    col *= 1.0 - 0.35 * uBus.w * (1.0 - ss(0.0, 8.0, boxD(q, vec2(0.0, 8.0), vec2(L * 0.95, 4.0), 4.0)));
    if (body < 1.5) {
      vec3 upper = vec3(0.98, 0.79, 0.22), lower = vec3(0.95, 0.94, 0.88);
      vec3 bc = q.y < -Ht * 0.45 ? upper : lower;
      bc = mix(bc, vec3(0.20, 0.45, 0.62), step(abs(q.y + 18.0), 2.2));            // sọc xanh
      // Dãy cửa kính (+ người ngồi trong xe)
      float wy = q.y + Ht * 0.68;
      float winBand = step(abs(wy), 13.0) * step(q.x, L - 26.0) * step(-L + 12.0, q.x);
      float mull = step(abs(fract((q.x + L) / 24.0) - 0.5) * 24.0, 1.8);
      vec3 glass = mix(vec3(0.24, 0.34, 0.42), vec3(1.0, 0.86, 0.55), lit);
      glass += vec3(0.25, 0.30, 0.32) * (1.0 - lit) * step(fract((q.x - q.y * 0.6) / 70.0), 0.18);   // vệt phản chiếu
      float seat = floor((q.x + L) / 24.0);
      float head = (1.0 - ss(4.0, 5.2, length(vec2(fract((q.x + L) / 24.0) * 24.0 - 12.0, wy - 2.0)))) * step(0.45, hash1(seat + 3.0));
      float shoulder = (1.0 - ss(0.0, 1.0, boxD(vec2(fract((q.x + L) / 24.0) * 24.0 - 12.0, wy - 13.0), vec2(0.0), vec2(8.0, 6.0), 4.0))) * step(0.45, hash1(seat + 3.0));
      glass = mix(glass, glass * 0.45, max(head, shoulder) * 0.8);
      bc = mix(bc, glass, winBand * (1.0 - mull));
      // Kính lái phía trước
      float wind = (1.0 - ss(0.0, 1.0, boxD(q, vec2(L - 12.0, -Ht * 0.62), vec2(10.0, 19.0), 5.0)));
      bc = mix(bc, glass * 1.05, wind);
      // Cửa đôi (mở khi dừng)
      float dx = q.x - (L - 44.0);
      float door = step(abs(dx), 13.0) * step(-Ht + 2.0, q.y) * step(q.y, -9.0);
      float open = uBusB.x * 9.0;
      float pane = door * step(open, abs(dx));
      bc = mix(bc, mix(vec3(0.42, 0.38, 0.33), vec3(0.95, 0.80, 0.52), lit) * (0.75 + 0.25 * ss(-Ht, -9.0, q.y)), door * (1.0 - pane));   // lòng xe khi cửa mở
      bc = mix(bc, glass, pane * step(abs(q.y + Ht * 0.55), 18.0));
      bc = mix(bc, vec3(0.5), pane * step(abs(abs(dx) - open - 0.5), 0.8));
      bc = mix(bc, vec3(0.25), door * step(abs(dx), 0.8) * (1.0 - step(0.1, uBusB.x)));
      // Đèn pha + đèn hậu
      float hl = 1.0 - ss(0.0, 1.0, length(q - vec2(L - 1.0, -14.0)) - 4.0);
      float tl = 1.0 - ss(0.0, 1.0, boxD(q, vec2(-L + 1.0, -18.0), vec2(2.0, 6.0), 1.0));
      bc = mix(bc, vec3(1.0, 0.95, 0.75), hl);
      bc = mix(bc, vec3(1.0, 0.15 + 0.1 * uBusB.z, 0.1) * (0.7 + 0.6 * uBusB.z), tl);
      bc *= mix(amb, vec3(1.0), 0.15);
      bc *= 1.0 - 0.18 * ss(-Ht * 0.2, 0.0, q.y);                                     // gầm xe tối
      bc += vec3(0.18, 0.14, 0.05) * gold * ss(-Ht * 0.3, -Ht, q.y);                 // nắng chiều rọi nóc
      col = mix(col, bc, (1.0 - ss(-0.8, 0.8, body)) * uBus.w);
    }
    // Bánh xe quay
    for (int k = 0; k < 2; k++) {
      vec2 wc = vec2(k == 0 ? L - 30.0 : -L + 28.0, -2.0);
      vec2 wq = q - wc;
      float r = length(wq);
      if (r < 15.0) {
        float a = atan(wq.y, wq.x) * dir + uBus.z;
        vec3 wcol = r < 7.0 ? mix(vec3(0.72), vec3(0.45), step(0.5, fract(a / PI * 2.0))) : vec3(0.10, 0.10, 0.12);
        col = mix(col, wcol * mix(amb, vec3(1.0), 0.3), (1.0 - ss(12.5, 13.5, r)) * uBus.w);
      }
    }
    // Pha đèn chiếu xuống đường ban đêm
    vec2 bq = q - vec2(L, -14.0);
    if (bq.x > 0.0) col += vec3(1.0, 0.92, 0.7) * lit * 0.35 * exp(-bq.x / 120.0) * (1.0 - ss(0.0, 8.0 + bq.x * 0.25, abs(bq.y - bq.x * 0.08))) * uBus.w;
    // Tiếng còi: các vạch toả ra phía trước
    if (uBusB.y >= 0.0 && uBusB.y < 0.8) {
      vec2 hq = q - vec2(L + 8.0, -Ht * 0.55);
      float rr = 8.0 + uBusB.y * 50.0;
      float arc = (1.0 - ss(0.0, 1.6, abs(length(hq) - rr))) * step(abs(atan(hq.y, hq.x)), 0.6) * step(0.0, hq.x);
      col = mix(col, vec3(1.0), arc * (1.0 - uBusB.y / 0.8));
    }
  }

  // ---------- Đồi cỏ ôm biểu tượng ----------
  float s = mix(fieldS(uFA, p), fieldS(uFB, p), uMix);
  if (s < 40.0) {
    float sUp = mix(fieldS(uFA, p - vec2(0.0, 8.0)), fieldS(uFB, p - vec2(0.0, 8.0)), uMix);
    vec3 g0 = vec3(0.56, 0.74, 0.34), g1 = vec3(0.36, 0.58, 0.26);
    vec3 gc = mix(g0, g1, ss(0.0, 120.0, -s));
    gc *= 0.9 + 0.18 * N(p, 260.0).g;
    gc = mix(gc, gc * vec3(0.78, 0.86, 0.70), (1.0 - ss(0.0, 1.0, abs(fract(p.x / 7.0 + N(p, 90.0).r * 2.0) - 0.5) * 2.0)) * 0.25);
    gc += vec3(0.20, 0.12, 0.02) * gold * ss(0.0, 6.0, sUp - s) * (1.0 - ss(0.0, 26.0, -s));   // mép đón nắng
    gc *= amb;
    float edge = 1.0 - ss(-1.0, 1.0, s);
    // Viền cỏ: lá mảnh nhô lên ở mép trên (nơi đồi nằm phía dưới)
    float cell = floor(p.x / 5.0);
    float bh = 6.0 + 16.0 * hash1(cell + uSeed);
    float tip = 1.0 - abs(fract(p.x / 5.0 + sin(t * 1.6 + cell * 0.7) * 0.15 * uWind) - 0.5) * 2.0;
    float fringe = step(s, bh * tip) * step(0.0, s) * ss(0.0, 4.0, sUp - s);
    // Mây ôm biểu tượng ở vùng trời: mép phồng thành cụm, sáng phía trên, tối phía dưới
    vec2 wq = p / 30.0, wi = floor(wq), wf = fract(wq);
    float wd = 9.0;
    for (int yy = -1; yy <= 1; yy++) for (int xx = -1; xx <= 1; xx++) {
      vec2 g = vec2(float(xx), float(yy));
      vec2 r = g + vec2(hash2(wi + g + uSeed), hash2(wi + g + uSeed + 7.0)) * 0.8 + 0.1 - wf;
      wd = min(wd, length(r));
    }
    float puff = s - 14.0 + 22.0 * wd - 6.0 * (N(p * 2.0, 120.0).r - 0.5);   // mép thành các cụm bông tròn
    vec3 cLit = pal(w, vec3(0.42, 0.42, 0.62), vec3(1.0, 0.88, 0.78), vec3(1.0, 1.0, 1.0));
    vec3 cSh = pal(w, vec3(0.22, 0.22, 0.40), vec3(0.90, 0.60, 0.58), vec3(0.80, 0.88, 0.97));
    vec3 cc = mix(cSh, cLit, ss(-6.0, 8.0, sUp - s + 4.0) * 0.7 + 0.3 * ss(-60.0, 0.0, s));
    cc = mix(cc, cLit, (1.0 - ss(0.0, 10.0, -puff)) * ss(0.0, 6.0, sUp - s) * 0.6);
    float cloudA = 1.0 - ss(-1.2, 1.2, puff);
    float sky2ground = ss(yr - 90.0, yr + 30.0, p.y);
    vec4 cloudL = vec4(cc, cloudA);
    vec4 grassL = vec4(gc, max(edge, fringe));
    vec4 L = mix(cloudL, grassL, sky2ground);
    col = mix(col, L.rgb, L.a);
    col = mix(col, gc * 0.8, (1.0 - ss(0.0, 2.5, abs(s))) * 0.35 * sky2ground);
  }

  // ---------- Lau sậy tiền cảnh (trước xe buýt) ----------
  float baseY = uRes.y - TASKBAR + 6.0;
  if (p.y > yrd - 40.0) {
    float span = baseY - yrd;
    vec3 r2 = reed(p, baseY + 10.0, 7.0, span * 0.45, span * 0.85, 0.80 * uReeds + 0.05, 11.0);  // lớp sau, thấp
    vec3 r1 = reed(p, baseY + 20.0, 10.0, span * 0.6, span * 1.12, 0.75 * uReeds + 0.05, 37.0); // lớp trước, cao
    vec3 stemD = pal(w, vec3(0.08, 0.10, 0.16), vec3(0.42, 0.34, 0.14), vec3(0.22, 0.42, 0.16));
    vec3 stemL = pal(w, vec3(0.16, 0.18, 0.26), vec3(0.92, 0.66, 0.30), vec3(0.50, 0.70, 0.28));
    vec3 plumeC = pal(w, vec3(0.55, 0.58, 0.72), vec3(1.0, 0.86, 0.60), vec3(0.96, 0.92, 0.80));
    vec3 c2 = r2.z > 0.5 ? plumeC * 0.85 : mix(stemD, stemL, r2.y) * 0.85;
    col = mix(col, c2, r2.x * 0.9);
    vec3 c1 = r1.z > 0.5 ? plumeC : mix(stemD, stemL, r1.y);
    col = mix(col, c1, r1.x);
    // Bụi cỏ sát đất
    float tuft = ss(-1.0, 1.0, p.y - (baseY - 14.0 - 12.0 * N(vec2(p.x * 2.0, 0.0), 160.0).r - 4.0 * sin(p.x * 0.09 + t * uWind)));
    col = mix(col, stemD * 0.75, tuft);
  }

  // ---------- Hạt: chim, bồ công anh, đom đóm ----------
  for (int i = 0; i < 24; i++) {
    if (float(i) >= uPartN) break;
    vec4 q4 = uPart[i];
    vec2 d = p - q4.xy;
    if (dot(d, d) > 900.0) continue;
    if (q4.w < 0.5) {
      // Chim: nét chữ V vỗ cánh
      float flap = sin(q4.z) * 5.0;
      float wb = min(segD(p, q4.xy, q4.xy + vec2(-9.0, -flap - 2.0)), segD(p, q4.xy, q4.xy + vec2(9.0, -flap - 2.0)));
      col = mix(col, pal(w, vec3(0.1), vec3(0.25, 0.15, 0.15), vec3(0.20, 0.22, 0.28)), 1.0 - ss(0.6, 1.4, wb));
    } else if (q4.w < 1.5) {
      // Hạt bồ công anh: chấm + tơ trắng
      float r = length(d);
      float fluff = (1.0 - ss(3.5, 5.5, r)) * (0.5 + 0.5 * step(0.5, fract(atan(d.y, d.x) / PI * 4.0 + q4.z)));
      col = mix(col, vec3(1.0, 0.98, 0.94) * mix(vec3(1.0), amb, 0.3), max(fluff * 0.7, 1.0 - ss(0.8, 1.6, r)));
    } else {
      // Đom đóm: chấm vàng xanh lập loè
      float r = length(d);
      float blink = 0.5 + 0.5 * sin(q4.z * 3.0);
      col += vec3(0.85, 1.0, 0.45) * (exp(-r / 4.0) * 0.9 + exp(-r / 14.0) * 0.3) * blink;
    }
  }
  // Sao băng
  if (uStar.z >= 0.0 && uStar.z < 1.2) {
    vec2 dirS = vec2(cos(uStar.w), sin(uStar.w));
    vec2 head = uStar.xy + dirS * uStar.z * 520.0;
    float tail = segD(p, head, head - dirS * 110.0);
    float along = clamp(dot(p - head, -dirS) / 110.0, 0.0, 1.0);
    col += vec3(1.0, 0.95, 0.85) * (1.0 - ss(0.0, 1.8, tail)) * (1.0 - along) * (1.0 - uStar.z / 1.2);
  }

  // Ánh hoàng hôn phủ nhẹ + tối góc
  col = mix(col, col * vec3(1.06, 0.96, 0.88), gold * 0.25);
  vec2 c = p / uRes - 0.5;
  col *= 1.0 - 0.22 * ss(0.45, 0.95, length(c * vec2(1.0, 1.25)));
  if (uDebug > 0.5) col = mix(col, vec3(0.9, 0.15, 0.15), 1.0 - ss(0.0, 1.5, abs(s)));
  o = vec4(col, 1.0);
}
