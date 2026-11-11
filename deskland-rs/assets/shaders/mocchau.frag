#version 300 es
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uBloom, uPetals, uWind, uMist, uShake, uDebug;
uniform vec4 uBird[8];
uniform float uBirdN;
uniform vec4 uBurst[6];
out vec4 o;

// Bảng màu cỏ non mùa xuân cao nguyên
const vec3 GRASS_L = vec3(0.76, 0.84, 0.57);
const vec3 GRASS_M = vec3(0.65, 0.77, 0.48);
const vec3 GRASS_D = vec3(0.51, 0.66, 0.40);
const vec3 HILL_BG = vec3(0.47, 0.63, 0.44);
const vec3 HILL_DEEP = vec3(0.36, 0.53, 0.39);
const vec3 INK_GRASS = vec3(0.26, 0.40, 0.22);
const vec3 BARK_INK  = vec3(0.28, 0.21, 0.18);

// Bảng màu tán hoa mận trắng màu nước
const vec3 PLUM_HI   = vec3(0.99, 0.99, 0.97);
const vec3 PLUM_CREAM= vec3(0.96, 0.95, 0.91);
const vec3 PLUM_MIST = vec3(0.84, 0.89, 0.93);
const vec3 PLUM_BLUSH= vec3(0.97, 0.91, 0.92);

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

float grassInk(vec2 p) {
  const vec2 G = vec2(13.0, 6.5);
  vec2 c0 = floor(p / G);
  float ink = 0.0;
  for (int j = -1; j <= 1; j++) {
    vec2 c = c0 + vec2(0.0, float(j));
    vec2 cp = (c + 0.5) * G;
    float h = hash2(c + 3.1);
    float dens = 0.70 * ss(0.42, 0.72, N(cp, 880.0).r) + 0.02;
    if (h > dens) continue;
    float ang = (N(cp, 1300.0).g - 0.5) * 0.7 + (hash2(c + 7.7) - 0.5) * 0.3;
    float len = 2.4 + 2.8 * hash2(c + 1.9);
    vec2 ctr = cp + (vec2(hash2(c + 4.3), hash2(c + 5.9)) - 0.5) * vec2(2.0, 3.0);
    vec2 dir = vec2(cos(ang), sin(ang)) * len;
    float d = segD(p, ctr - dir, ctr + dir);
    ink = max(ink, (1.0 - ss(0.2, 0.9, d)) * (0.35 + 0.42 * hash2(c + 9.1)));
  }
  return ink;
}

// Cánh hoa mận trắng rơi rải rác ven triền cỏ
float groundPetals(vec2 p) {
  vec2 G = vec2(22.0, 22.0);
  vec2 c = floor(p / G);
  float h = hash2(c + uSeed * 0.09);
  if (h > 0.32) return 0.0;
  vec2 ctr = (c + 0.25 + 0.5 * vec2(hash2(c + 1.7), hash2(c + 3.3))) * G;
  vec2 dp = rot(hash2(c + 5.1) * 6.283) * (p - ctr);
  float d = length(dp / vec2(2.1, 1.3)) - 1.0;
  return 1.0 - ss(-0.2, 0.5, d * 1.5);
}

// Tán hoa mận trắng màu nước và cành cây nét mực mảnh (vùng lân cận 2x2)
vec4 plumLayer(vec2 p, float G, float seedOff, float sDist, float minS, float t, float nLobe, out float branchAlpha) {
  branchAlpha = 0.0;
  vec2 q = p / G;
  vec2 c0 = floor(q), f = q - c0;
  int sx = f.x < 0.5 ? -1 : 1, sy = f.y < 0.5 ? -1 : 1;
  float bestD = 1e9;
  vec3 bestCol = PLUM_HI;
  vec2 bestCtr = vec2(0.0);
  float bestRad = 1.0, bestH = 0.0;
  for (int dy = 0; dy <= 1; dy++) for (int dx = 0; dx <= 1; dx++) {
    vec2 c = c0 + vec2(float(dx * sx), float(dy * sy));
    float h = hash2(c + seedOff);
    if (h > uBloom * 0.92) continue;
    vec2 baseCtr = (c + 0.22 + 0.56 * vec2(hash2(c + seedOff + 1.3), hash2(c + seedOff + 2.7))) * G;
    float swPhase = t * (1.2 + 0.4 * h) + baseCtr.x * 0.011 + baseCtr.y * 0.009;
    float rustle = sin(t * 16.0 + baseCtr.x * 0.06 + baseCtr.y * 0.05) * 3.6 * uShake;
    vec2 ctr = baseCtr + vec2(sin(swPhase), cos(swPhase * 0.8)) * (2.4 * uWind) + vec2(rustle, -rustle * 0.65);
    vec2 dp = p - ctr;
    float rad = G * (0.42 + 0.18 * hash2(c + seedOff + 6.1)) * (1.0 + nLobe);
    float d = length(dp) - rad;
    if (d < bestD) {
      bestD = d; bestCtr = ctr; bestRad = rad; bestH = h;
      float tone = hash2(c + seedOff + 8.9);
      vec3 cBase = tone < 0.52 ? PLUM_HI : (tone < 0.82 ? PLUM_CREAM : PLUM_BLUSH);
      float lit = ss(rad * 0.75, -rad * 0.75, dp.x * 0.6 + dp.y * 0.8);
      cBase = mix(PLUM_MIST, cBase, lit);
      float rim = ss(-rad * 0.35, 0.0, d) * 0.12;
      bestCol = mix(cBase, PLUM_MIST * 0.94, rim);
    }
  }
  if (bestD < 4.0) {
    vec2 root = bestCtr + vec2((bestH - 0.5) * 6.0, bestRad * 0.55);
    vec2 tip1 = bestCtr + vec2(-bestRad * 0.35, -bestRad * 0.25);
    vec2 tip2 = bestCtr + vec2(bestRad * 0.38, -bestRad * 0.18);
    float bd = min(segD(p, root, bestCtr), min(segD(p, bestCtr, tip1), segD(p, bestCtr, tip2)));
    branchAlpha = (1.0 - ss(0.4, 1.3, bd)) * ss(minS, minS + 16.0, sDist) * 0.55;
  }
  float mask = (1.0 - ss(-1.2, 1.4, bestD)) * ss(minS, minS + 16.0, sDist);
  return vec4(bestCol, mask * 0.92);
}

// Cánh hoa mận trắng bay theo gió
float floatingPetals(vec2 p, float t, float dens, float scale) {
  if (dens <= 0.01) return 0.0;
  vec2 wp = p - vec2(t * (26.0 + 18.0 * uWind) * scale, t * 18.0 * scale);
  wp.y += sin(p.x * 0.018 + t * 1.4) * 14.0;
  vec2 G = vec2(68.0 * scale, 68.0 * scale);
  vec2 c = floor(wp / G);
  vec2 f = (fract(wp / G) - 0.5) * G;
  float h = hash2(c + scale * 29.1 + uSeed * 0.05);
  if (h > dens * 0.72) return 0.0;
  vec2 ofs = (vec2(hash2(c + 2.3), hash2(c + 5.7)) - 0.5) * G * 0.55;
  float rotA = t * (1.8 + hash2(c + 8.1)) + h * 6.283;
  vec2 q = rot(rotA) * (f - ofs);
  float d = length(q / vec2(3.2 * scale, 1.9 * scale)) - 1.0;
  return 1.0 - ss(-0.15, 0.45, d * 2.0);
}

// Chim nhỏ trong rừng
vec3 birdSD(vec2 q, float ph) {
  float flap = sin(ph) * 2.0;
  float body = length(q / vec2(5.0, 2.0)) - 1.0;
  vec2 wq = vec2(q.x + 0.4, abs(q.y) - 1.0);
  vec2 rw = rot(-0.2 + 0.12 * sin(ph)) * wq;
  float wing = length((rw - vec2(-0.6 + flap * 0.14, 3.8)) / vec2(2.3, 4.6)) - 1.0;
  return vec3(body * 2.0, wing * 2.3, clamp(wq.y / 8.5, 0.0, 1.0));
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y * uScale - gl_FragCoord.y) / uScale;
  float t = uTime;
  vec2 gp = floor(p * (0.8 * max(uScale, 1.0)));
  float g2 = hash2(gp + 7.3);

  float s0 = mix(fieldS(uFA, p), fieldS(uFB, p), uMix);
  float s = s0 + (N(p, 620.0).r - 0.5) * 10.0;

  vec3 col;
  if (s <= 0.0) {
    // ---------- Thảm cỏ xuân cao nguyên (vùng biểu tượng) ----------
    float w1 = N(p, 1750.0).b, w2 = N(p + 293.0, 620.0).g;
    col = mix(GRASS_M, GRASS_L, ss(0.28, 0.74, w1 * 0.7 + w2 * 0.3));
    col = mix(col, GRASS_D, ss(0.45, 0.85, N(p + 911.0, 1200.0).r) * 0.36);
    col = mix(col, INK_GRASS, grassInk(p) * 0.68 * ss(-2.0, -14.0, s));
    // Cánh hoa mận trắng rơi ven triền cỏ
    float gpMask = groundPetals(p) * ss(-36.0, -2.0, s);
    col = mix(col, PLUM_HI, gpMask * 0.85);
  } else {
    // ---------- Đồi chè nhấp nhô và rừng hoa mận trắng ----------
    float dNorm = clamp(s / 260.0, 0.0, 1.0);
    col = mix(GRASS_D, HILL_DEEP, dNorm * 0.75);
    // Đường đồng mức màu nước mềm mại trên sườn đồi
    float terrace = sin(s * 0.14 + N(p, 420.0).r * 4.5);
    col = mix(col, HILL_BG, ss(-0.3, 0.6, terrace) * 0.35);

    // Hai tầng tán hoa mận trắng đan xen và cành cây nét mực mảnh
    float nLobe = (N(p, 42.0).g - 0.5) * 0.22;
    float br1 = 0.0, br2 = 0.0;
    vec4 L1 = plumLayer(p, 58.0, 13.0, s, 2.0, t, nLobe, br1);
    col = mix(col, BARK_INK, br1 * 0.55);
    if (L1.a > 0.001) {
      col *= 1.0 - 0.10 * L1.a;
      col = mix(col, L1.rgb * 0.95, L1.a);
    }

    vec4 L2 = plumLayer(p + vec2(24.0, 17.0), 40.0, 41.0, s, 0.0, t, -nLobe, br2);
    col = mix(col, BARK_INK, br2 * 0.62);
    if (L2.a > 0.001) {
      col *= 1.0 - 0.12 * L2.a;
      col = mix(col, L2.rgb, L2.a);
    }
  }

  // ---------- Chim bay trên cao nguyên ----------
  for (int i = 0; i < 8; i++) {
    if (float(i) >= uBirdN) break;
    vec4 b = uBird[i];
    vec2 d0 = p - b.xy, d1 = d0 - vec2(22.0, 34.0);
    if (dot(d1, d1) < 320.0) {
      vec3 g = birdSD(rot(-b.z) * d1, b.w);
      col = mix(col, col * 0.74, ss(1.8, -1.8, min(g.x, g.y)) * 0.28);
    }
    if (dot(d0, d0) < 320.0) {
      vec3 g = birdSD(rot(-b.z) * d0, b.w);
      col = mix(col, vec3(0.26, 0.32, 0.36), ss(0.6, -0.6, min(g.x, g.y)));
      col = mix(col, vec3(0.95, 0.96, 0.94), ss(0.6, -0.6, g.x) * 0.6);
    }
  }

  // ---------- Sương sớm dịu nhẹ trên cao nguyên ----------
  if (uMist > 0.01) {
    vec2 mp1 = p + vec2(-t * 11.0, sin(p.x * 0.0035 + t * 0.18) * 16.0);
    vec2 mp2 = p + vec2(-t * 18.0 + 350.0, cos(p.x * 0.003 - t * 0.14) * 20.0);
    float m = N(mp1, 950.0).r * N(mp2, 520.0).g;
    float mistA = ss(0.20, 0.62, m) * uMist * 0.52;
    col = mix(col, vec3(0.95, 0.97, 0.96), mistA);
  }

  // ---------- Cánh hoa mận trắng bay theo gió ----------
  if (uPetals > 0.01) {
    float p1 = floatingPetals(p, t, uPetals, 1.0);
    float p2 = floatingPetals(p + vec2(43.0, 29.0), t * 0.85, uPetals * 0.8, 0.76);
    float pMask = max(p1, p2 * 0.85);
    col = mix(col, vec3(1.0, 0.99, 0.98), pMask * 0.92);
  }

  // ---------- Chùm hoa mận 5 cánh & cánh hoa bung xoáy khi tương tác click ----------
  for (int bi = 0; bi < 6; bi++) {
    vec4 b = uBurst[bi];
    if (b.w <= 0.0) continue;
    float age = b.z;
    // Vòng gió xuân lan toả nhẹ khi vừa chạm cành
    if (age < 1.1) {
      float wr = 8.0 + age * 68.0;
      float wRing = (1.0 - ss(0.8, 3.0, abs(length(p - b.xy) - wr))) * (1.0 - age / 1.1);
      col = mix(col, vec3(1.0, 0.98, 0.92), wRing * 0.48);
    }
    vec2 drift = vec2((28.0 + 18.0 * uWind) * age, 13.0 * age);
    vec2 bCenter = b.xy + drift;
    if (dot(p - bCenter, p - bCenter) > 225.0 * 225.0) continue;
    float alpha = ss(0.0, 0.10, age) * (1.0 - ss(3.2, 4.5, age));
    float ease = 1.0 - exp(-age * 2.2);
    for (int k = 0; k < 16; k++) {
      float fk = float(k);
      float h1 = hash2(vec2(fk * 4.3, b.w * 37.1));
      float h2 = hash2(vec2(fk * 8.7 + 2.1, b.w * 23.9));
      float ang = fk * 0.3927 + (h1 - 0.5) * 0.35 + age * (0.55 + 0.35 * h2);
      float rad = (14.0 + 58.0 * h2) * ease + age * (8.0 + 10.0 * h1);
      vec2 pos = b.xy + drift + vec2(cos(ang), sin(ang) * 0.82) * rad;
      pos.y += sin(age * 2.6 + h1 * 6.28) * 6.0;
      vec2 dq = p - pos;
      if (dot(dq, dq) > 144.0) continue;
      if (k < 6) {
        // Bông hoa mận 5 cánh trắng tinh khôi kèm bóng đổ & nhụy vàng hồng
        vec2 shQ = dq - vec2(2.6, 3.4);
        col = mix(col, HILL_DEEP * 0.72, (1.0 - ss(3.5, 7.5, length(shQ))) * 0.42 * alpha);
        vec2 rq = rot(age * (1.8 + h1) + h2 * 6.28) * dq;
        float r = length(rq);
        float a5 = atan(rq.y, rq.x);
        float fRad = (5.4 + 2.2 * cos(a5 * 5.0)) * (0.88 + 0.22 * h1);
        float rim = (1.0 - ss(fRad - 0.2, fRad + 1.1, r)) * alpha;
        col = mix(col, vec3(0.58, 0.68, 0.62), rim * 0.55);
        float flw = 1.0 - ss(fRad - 0.65, fRad + 0.55, r);
        if (flw > 0.001) {
          vec3 fCol = mix(vec3(1.0, 1.0, 0.99), PLUM_BLUSH, ss(3.8, 0.6, r) * 0.65);
          fCol = mix(fCol, vec3(0.96, 0.74, 0.22), 1.0 - ss(0.8, 2.2, r));
          col = mix(col, fCol, flw * alpha * 0.98);
        }
      } else {
        // Cánh hoa mận trắng chao liệng có bóng nhẹ
        col = mix(col, HILL_DEEP * 0.75, (1.0 - ss(2.0, 5.5, length(dq - vec2(2.0, 2.8)))) * 0.34 * alpha);
        vec2 rq = rot(age * (2.6 + h2 * 1.8) + h1 * 6.28) * dq;
        float squeeze = 0.55 + 0.45 * abs(cos(age * 2.4 + fk));
        float pd = length(rq / vec2(5.2, 3.2 * squeeze)) - 1.0;
        float pm = 1.0 - ss(-0.15, 0.35, pd * 2.0);
        vec3 pCol = h1 > 0.55 ? PLUM_BLUSH : vec3(1.0, 1.0, 0.99);
        col = mix(col, pCol, pm * alpha * 0.96);
      }
    }
  }

  col *= 1.0 + (g2 - 0.5) * 0.028;
  if (uDebug > 0.5) col = mix(col, vec3(0.9, 0.15, 0.15), 1.0 - ss(0.0, 1.5, abs(s0)));
  o = vec4(col, 1.0);
}
