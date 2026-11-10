#version 300 es
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uPetals, uWind, uDebug;
uniform vec4 uSwallow[8];
uniform float uSwallowN;
uniform vec4 uLantern[8];
uniform float uLanternN;
out vec4 o;

// Bảng màu nền giấy dó nắng xuân ấm áp
const vec3 PAPER_L = vec3(0.99, 0.96, 0.91);
const vec3 PAPER_M = vec3(0.96, 0.92, 0.85);
const vec3 PAPER_D = vec3(0.93, 0.87, 0.79);
const vec3 BARK_INK = vec3(0.29, 0.19, 0.18);

// Bảng màu nước hoa anh đào / đào bích ngày Tết
const vec3 SAKURA_PALE  = vec3(0.99, 0.88, 0.90);
const vec3 SAKURA_PINK  = vec3(0.96, 0.68, 0.75);
const vec3 SAKURA_PEACH = vec3(0.94, 0.52, 0.62);
const vec3 SAKURA_WARM  = vec3(0.97, 0.76, 0.72);
const vec3 GOLD_STAMEN  = vec3(0.98, 0.82, 0.32);

// Bảng màu đèn lồng đỏ và bao lì xì Tết
const vec3 RED_FEST  = vec3(0.88, 0.18, 0.20);
const vec3 RED_DARK  = vec3(0.68, 0.10, 0.14);
const vec3 GOLD_FEST = vec3(0.98, 0.82, 0.28);

float hash2(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}
float ss(float a, float b, float x) { float t = clamp((x - a) / (b - a), 0.0, 1.0); return t * t * (3.0 - 2.0 * t); }
vec4 N(vec2 p, float scale) { return texture(uNoise, p / scale + uSeed * 0.013); }
float fieldS(sampler2D t, vec2 p) { return texture(t, (p / uStep + 0.5) / uGrid).r; }
mat2 rot(float a) { float c = cos(a), s = sin(a); return mat2(c, s, -s, c); }
float segD(vec2 p, vec2 a, vec2 b) { vec2 pa = p - a, ba = b - a; return length(pa - ba * clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0)); }

// Cánh hoa đào rơi rải rác trên sân
float fallenPetals(vec2 p) {
  vec2 G = vec2(20.0, 20.0);
  vec2 c = floor(p / G);
  float h = hash2(c + uSeed * 0.07);
  if (h > 0.36) return 0.0;
  vec2 ctr = (c + 0.22 + 0.56 * vec2(hash2(c + 1.3), hash2(c + 2.9))) * G;
  vec2 dp = rot(hash2(c + 4.7) * 6.283) * (p - ctr);
  float d = length(dp / vec2(2.4, 1.4)) - 1.0;
  return 1.0 - ss(-0.2, 0.5, d * 1.6);
}

// Tán hoa đào / anh đào màu nước và cành đào nét mực (vùng lân cận 2x2 tối ưu cho GPU tích hợp)
vec4 sakuraLayer(vec2 p, float G, float seedOff, float sDist, float minS, float t, float nLobe, out float branchAlpha) {
  branchAlpha = 0.0;
  vec2 q = p / G;
  vec2 c0 = floor(q), f = q - c0;
  int sx = f.x < 0.5 ? -1 : 1, sy = f.y < 0.5 ? -1 : 1;
  float bestD = 1e9;
  vec3 bestCol = SAKURA_PINK;
  vec2 bestCtr = vec2(0.0);
  float bestRad = 1.0, bestH = 0.0;
  for (int dy = 0; dy <= 1; dy++) for (int dx = 0; dx <= 1; dx++) {
    vec2 c = c0 + vec2(float(dx * sx), float(dy * sy));
    float h = hash2(c + seedOff);
    if (h < 0.14) continue;
    vec2 baseCtr = (c + 0.22 + 0.56 * vec2(hash2(c + seedOff + 1.3), hash2(c + seedOff + 2.7))) * G;
    float swPhase = t * (1.25 + 0.45 * h) + baseCtr.x * 0.011 + baseCtr.y * 0.009;
    vec2 ctr = baseCtr + vec2(sin(swPhase), cos(swPhase * 0.82)) * (2.5 * uWind);
    vec2 dp = p - ctr;
    float rad = G * (0.43 + 0.18 * hash2(c + seedOff + 6.1)) * (1.0 + nLobe);
    float d = length(dp) - rad;
    if (d < bestD) {
      bestD = d; bestCtr = ctr; bestRad = rad; bestH = h;
      float tone = hash2(c + seedOff + 8.9);
      vec3 cBase = tone < 0.32 ? SAKURA_PALE
                 : (tone < 0.68 ? SAKURA_PINK
                 : (tone < 0.88 ? SAKURA_WARM : SAKURA_PEACH));
      float lit = ss(rad * 0.75, -rad * 0.75, dp.x * 0.6 + dp.y * 0.8);
      cBase = mix(cBase * 0.90, mix(cBase, SAKURA_PALE, 0.45), lit);
      float rim = ss(-rad * 0.28, 0.0, d) * 0.22;
      cBase = mix(cBase, SAKURA_PEACH, rim);
      float stamen = 1.0 - ss(1.2, 3.2, d + rad);
      bestCol = mix(cBase, GOLD_STAMEN, stamen * 0.65);
    }
  }
  if (bestD < 4.5) {
    vec2 root = bestCtr + vec2((bestH - 0.5) * 8.0, bestRad * 0.62);
    vec2 tip1 = bestCtr + vec2(-bestRad * 0.38, -bestRad * 0.28);
    vec2 tip2 = bestCtr + vec2(bestRad * 0.40, -bestRad * 0.22);
    float bd = min(segD(p, root, bestCtr), min(segD(p, bestCtr, tip1), segD(p, bestCtr, tip2)));
    branchAlpha = (1.0 - ss(0.45, 1.35, bd)) * ss(minS, minS + 16.0, sDist) * 0.62;
  }
  float mask = (1.0 - ss(-1.2, 1.4, bestD)) * ss(minS, minS + 16.0, sDist);
  return vec4(bestCol, mask * 0.88);
}

// Chim én báo xuân (đuôi kéo đặc trưng)
vec3 swallowSD(vec2 q, float ph) {
  float flap = sin(ph) * 2.2;
  float body = length(q / vec2(5.2, 1.8)) - 1.0;
  // Đuôi chim én chẻ đôi
  vec2 tq = vec2(q.x + 5.5, abs(q.y) - 1.2);
  float tail = segD(tq, vec2(0.0, -0.5), vec2(-4.5, 1.4)) - 0.65;
  vec2 wq = vec2(q.x - 0.5, abs(q.y) - 1.0);
  vec2 rw = rot(-0.28 + 0.14 * sin(ph)) * wq;
  float wing = length((rw - vec2(-0.8 + flap * 0.15, 4.4)) / vec2(2.1, 5.2)) - 1.0;
  return vec3(min(body * 1.9, tail), wing * 2.1, clamp(wq.y / 9.0, 0.0, 1.0));
}

// Cánh hoa hồng phấn bay theo gió xuân
float floatingPetals(vec2 p, float t, float dens, float scale) {
  if (dens <= 0.01) return 0.0;
  vec2 wp = p - vec2(t * (24.0 + 16.0 * uWind) * scale, t * 20.0 * scale);
  wp.x += sin(p.y * 0.016 + t * 1.3) * 15.0;
  vec2 G = vec2(64.0 * scale, 64.0 * scale);
  vec2 c = floor(wp / G);
  vec2 f = (fract(wp / G) - 0.5) * G;
  float h = hash2(c + scale * 31.7 + uSeed * 0.05);
  if (h > dens * 0.75) return 0.0;
  vec2 ofs = (vec2(hash2(c + 2.3), hash2(c + 5.7)) - 0.5) * G * 0.55;
  float rotA = t * (1.6 + hash2(c + 8.1)) + h * 6.283;
  vec2 q = rot(rotA) * (f - ofs);
  float d = length(q / vec2(3.5 * scale, 2.0 * scale)) - 1.0;
  return 1.0 - ss(-0.15, 0.45, d * 2.0);
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y * uScale - gl_FragCoord.y) / uScale;
  float t = uTime;
  vec2 gp = floor(p * (0.8 * max(uScale, 1.0)));
  float g2 = hash2(gp + 7.3);

  float s0 = mix(fieldS(uFA, p), fieldS(uFB, p), uMix);
  float s = s0 + (N(p, 600.0).r - 0.5) * 10.0;

  // ---------- Nền sân xuân giấy dó ấm áp ----------
  float w1 = N(p, 1700.0).b, w2 = N(p + 281.0, 620.0).g;
  vec3 col = mix(PAPER_M, PAPER_L, ss(0.28, 0.74, w1 * 0.7 + w2 * 0.3));
  col = mix(col, PAPER_D, ss(0.45, 0.85, N(p + 847.0, 1200.0).r) * 0.35);

  if (s <= 0.0) {
    // Cánh hoa đào rơi ven khoảng sân
    float fp = fallenPetals(p) * ss(-42.0, -1.0, s);
    col = mix(col, SAKURA_PINK, fp * 0.82);
  } else {
    // ---------- Rừng hoa đào / anh đào nở rộ ----------
    float dNorm = clamp(s / 240.0, 0.0, 1.0);
    // Lớp màu nước hồng phớt dịu ở xa
    col = mix(col, vec3(0.96, 0.85, 0.84), dNorm * 0.55);

    float nLobe = (N(p, 42.0).g - 0.5) * 0.22;
    float br1 = 0.0, br2 = 0.0;
    vec4 L1 = sakuraLayer(p, 58.0, 15.0, s, 2.0, t, nLobe, br1);
    col = mix(col, BARK_INK, br1 * 0.55);
    if (L1.a > 0.001) {
      col = mix(col, L1.rgb * 0.96, L1.a * 0.84);
    }

    vec4 L2 = sakuraLayer(p + vec2(24.0, 18.0), 40.0, 43.0, s, 0.0, t, -nLobe, br2);
    col = mix(col, BARK_INK, br2 * 0.65);
    if (L2.a > 0.001) {
      col *= 1.0 - 0.06 * L2.a;
      col = mix(col, L2.rgb, L2.a * 0.90);
    }
  }

  // ---------- Đèn lồng đỏ và bao lì xì treo trên cành ----------
  for (int i = 0; i < 8; i++) {
    if (float(i) >= uLanternN) break;
    vec4 ln = uLantern[i];
    vec2 dp = p - ln.xy;
    if (dot(dp, dp) > 900.0) continue;
    float sw = sin(t * 2.1 + ln.z) * (0.12 * uWind);
    vec2 q = rot(sw) * dp;
    // Quầng sáng vàng đỏ ấm áp
    float glow = exp(-length(q) * 0.11) * 0.22;
    col = mix(col, vec3(1.0, 0.78, 0.42), glow);
    if (ln.w < 0.5) {
      // Đèn lồng đỏ tròn trịa
      float cord = segD(q, vec2(0.0, -13.0), vec2(0.0, -7.0)) - 0.6;
      float body = length(q / vec2(7.5, 6.2)) - 1.0;
      float capT = length((q - vec2(0.0, -6.0)) / vec2(3.8, 1.3)) - 1.0;
      float capB = length((q - vec2(0.0,  6.0)) / vec2(3.5, 1.2)) - 1.0;
      float tassel = segD(q, vec2(0.0, 6.5), vec2(0.0, 13.5)) - 0.85;
      col = mix(col, BARK_INK, 1.0 - ss(0.0, 0.8, cord));
      col = mix(col, GOLD_FEST, 1.0 - ss(-0.2, 0.6, tassel));
      if (body < 0.2) {
        vec3 lc = mix(RED_DARK, RED_FEST, ss(6.0, -5.0, q.x * 0.6 + q.y * 0.5));
        // Đường gân chỉ vàng trên đèn lồng
        float rib = 1.0 - ss(0.3, 0.9, abs(abs(q.x) - 3.6));
        lc = mix(lc, GOLD_FEST, rib * 0.35);
        col = mix(col, lc, 1.0 - ss(-0.15, 0.15, body));
      }
      col = mix(col, GOLD_FEST, 1.0 - ss(-0.2, 0.3, min(capT, capB)));
    } else {
      // Bao lì xì Tết
      vec2 b = abs(q) - vec2(5.2, 7.2);
      float boxD = length(max(b, 0.0)) + min(max(b.x, b.y), 0.0) - 1.2;
      float cord = segD(q, vec2(0.0, -13.0), vec2(0.0, -7.5)) - 0.6;
      col = mix(col, GOLD_FEST, 1.0 - ss(0.0, 0.8, cord));
      if (boxD < 0.8) {
        vec3 rc = mix(RED_DARK, RED_FEST, ss(6.0, -6.0, q.x + q.y * 0.5));
        // Ấn tròn nhũ vàng ở giữa bao lì xì
        float seal = 1.0 - ss(1.8, 2.5, length(q - vec2(0.0, -0.5)));
        rc = mix(rc, GOLD_FEST, seal);
        col = mix(col, rc, 1.0 - ss(-0.4, 0.6, boxD));
      }
    }
  }

  // ---------- Chim én báo xuân ----------
  for (int i = 0; i < 8; i++) {
    if (float(i) >= uSwallowN) break;
    vec4 b = uSwallow[i];
    vec2 d0 = p - b.xy, d1 = d0 - vec2(20.0, 32.0);
    if (dot(d1, d1) < 340.0) {
      vec3 g = swallowSD(rot(-b.z) * d1, b.w);
      col = mix(col, col * 0.82, ss(1.8, -1.8, min(g.x, g.y)) * 0.25);
    }
    if (dot(d0, d0) < 340.0) {
      vec2 rq = rot(-b.z) * d0;
      vec3 g = swallowSD(rq, b.w);
      col = mix(col, vec3(0.18, 0.22, 0.30), ss(0.6, -0.6, min(g.x, g.y)));
      // Cổ chim én điểm sắc đỏ nâu
      float throat = 1.0 - ss(1.0, 2.0, length(rq - vec2(3.6, 0.0)));
      col = mix(col, vec3(0.84, 0.32, 0.24), throat * ss(0.4, -0.4, g.x));
    }
  }

  // ---------- Cánh hoa đào bay theo gió ----------
  if (uPetals > 0.01) {
    float p1 = floatingPetals(p, t, uPetals, 1.0);
    float p2 = floatingPetals(p + vec2(37.0, 53.0), t * 0.86, uPetals * 0.82, 0.76);
    col = mix(col, SAKURA_PINK, p1 * 0.88);
    col = mix(col, SAKURA_PALE, p2 * 0.82);
  }

  col *= 1.0 + (g2 - 0.5) * 0.025;
  if (uDebug > 0.5) col = mix(col, vec3(0.9, 0.15, 0.15), 1.0 - ss(0.0, 1.5, abs(s0)));
  o = vec4(col, 1.0);
}
