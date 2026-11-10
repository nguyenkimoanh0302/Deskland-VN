#version 300 es
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uRain, uMist, uWind, uDebug;
uniform vec4 uBird[8];
uniform float uBirdN;
uniform vec4 uFly[15];
uniform float uFlyN;
out vec4 o;

// Bảng màu khoảng trống rêu xanh ẩm ướt
const vec3 MOSS_L = vec3(0.58, 0.73, 0.45);
const vec3 MOSS_M = vec3(0.46, 0.64, 0.38);
const vec3 MOSS_D = vec3(0.33, 0.52, 0.31);
const vec3 INK    = vec3(0.16, 0.30, 0.18);

// Bảng màu tán cây rừng mưa nhiệt đới nhiều tầng
const vec3 FLOOR_WET = vec3(0.11, 0.25, 0.21);
const vec3 CANOPY_0  = vec3(0.14, 0.34, 0.26);
const vec3 CANOPY_1  = vec3(0.19, 0.44, 0.32);
const vec3 CANOPY_2  = vec3(0.27, 0.55, 0.36);
const vec3 CANOPY_3  = vec3(0.38, 0.65, 0.39);
const vec3 CANOPY_TEAL = vec3(0.16, 0.47, 0.43);

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

// Nét cọ mực mảnh trên thảm cỏ rêu
float mossInk(vec2 p) {
  const vec2 G = vec2(13.0, 7.0);
  vec2 c0 = floor(p / G);
  float ink = 0.0;
  for (int j = -1; j <= 1; j++) {
    vec2 c = c0 + vec2(0.0, float(j));
    vec2 cp = (c + 0.5) * G;
    float h = hash2(c + 3.1);
    float dens = 0.72 * ss(0.40, 0.72, N(cp, 850.0).r) + 0.03;
    if (h > dens) continue;
    float ang = (N(cp, 1200.0).g - 0.5) * 0.8 + (hash2(c + 7.7) - 0.5) * 0.35;
    float len = 2.4 + 2.8 * hash2(c + 1.9);
    vec2 ctr = cp + (vec2(hash2(c + 4.3), hash2(c + 5.9)) - 0.5) * vec2(2.5, 3.0);
    vec2 dir = vec2(cos(ang), sin(ang)) * len;
    float d = segD(p, ctr - dir, ctr + dir);
    ink = max(ink, (1.0 - ss(0.2, 0.9, d)) * (0.35 + 0.45 * hash2(c + 9.1)));
  }
  return ink;
}

// Vòng gợn sóng hạt mưa trên mặt đất và vũng nước (vùng lân cận 2x2)
float rainRipples(vec2 p, float t) {
  vec2 G = vec2(52.0, 52.0), q = p / G;
  vec2 c0 = floor(q), f = q - c0;
  int sx = f.x < 0.5 ? -1 : 1, sy = f.y < 0.5 ? -1 : 1;
  float rip = 0.0;
  for (int dy = 0; dy <= 1; dy++) for (int dx = 0; dx <= 1; dx++) {
    vec2 c = c0 + vec2(float(dx * sx), float(dy * sy));
    float h = hash2(c + uSeed * 0.07);
    if (h > uRain * 0.85) continue;
    float ph = fract(t * (0.65 + 0.35 * hash2(c + 2.1)) + hash2(c + 5.3));
    vec2 ctr = (c + 0.22 + 0.56 * vec2(hash2(c + 1.1), hash2(c + 3.7))) * G;
    float d = length(p - ctr);
    float r = ph * 22.0;
    float ring = (1.0 - ss(0.5, 1.8, abs(d - r))) * (1.0 - ph) * ss(0.0, 0.08, ph);
    rip = max(rip, ring);
  }
  return rip;
}

// Tầng tán cây rừng mưa nhiệt đới (vùng lân cận 2x2, không dùng atan)
vec4 canopyLayer(vec2 p, float G, float seedOff, float sDist, float minS, float t, float nLobe) {
  vec2 q = p / G;
  vec2 c0 = floor(q), f = q - c0;
  int sx = f.x < 0.5 ? -1 : 1, sy = f.y < 0.5 ? -1 : 1;
  float bestD = 1e9;
  vec3 bestCol = CANOPY_1;
  for (int dy = 0; dy <= 1; dy++) for (int dx = 0; dx <= 1; dx++) {
    vec2 c = c0 + vec2(float(dx * sx), float(dy * sy));
    float h = hash2(c + seedOff);
    if (h < 0.18) continue;
    vec2 baseCtr = (c + 0.22 + 0.56 * vec2(hash2(c + seedOff + 1.3), hash2(c + seedOff + 2.7))) * G;
    float swPhase = t * (1.3 + 0.5 * h) + baseCtr.x * 0.012 + baseCtr.y * 0.009;
    vec2 ctr = baseCtr + vec2(sin(swPhase), cos(swPhase * 0.85)) * (2.6 * uWind);
    vec2 dp = p - ctr;
    float rad = G * (0.43 + 0.19 * hash2(c + seedOff + 6.1)) * (1.0 + nLobe);
    float d = length(dp) - rad;
    if (d < bestD) {
      bestD = d;
      float tone = hash2(c + seedOff + 8.9);
      vec3 cBase = tone < 0.28 ? CANOPY_1
                 : (tone < 0.58 ? CANOPY_2
                 : (tone < 0.82 ? CANOPY_TEAL : CANOPY_3));
      float lit = ss(rad * 0.8, -rad * 0.7, dp.x * 0.65 + dp.y * 0.75);
      cBase = mix(cBase * 0.84, cBase * 1.12, lit);
      float vein = (1.0 - ss(0.4, 1.3, abs(d + rad * 0.55))) * 0.12;
      bestCol = cBase * (1.0 - vein);
    }
  }
  float mask = (1.0 - ss(-1.2, 1.2, bestD)) * ss(minS, minS + 18.0, sDist);
  return vec4(bestCol, clamp(mask, 0.0, 1.0));
}

// Chim nhiệt đới (bóng dáng chim bói cá / vẹt xanh nhìn từ trên xuống)
vec3 birdSD(vec2 q, float ph) {
  float flap = sin(ph) * 2.2;
  float body = length(q / vec2(5.5, 2.2)) - 1.0;
  vec2 wq = vec2(q.x + 0.5, abs(q.y));
  wq.y -= 1.2;
  float ang = -0.22 + 0.12 * sin(ph);
  vec2 rw = rot(ang) * wq;
  float wing = length((rw - vec2(-0.8 + flap * 0.15, 4.2)) / vec2(2.6, 5.2)) - 1.0;
  return vec3(body * 2.2, wing * 2.6, clamp(wq.y / 9.5, 0.0, 1.0));
}

// Lớp hạt mưa rơi xiên
float rainStreaks(vec2 p, float t, float dens, float speed, float scale) {
  if (dens <= 0.01) return 0.0;
  vec2 rp = rot(0.18) * p;
  rp.y -= t * speed;
  vec2 G = vec2(26.0 * scale, 140.0 * scale);
  vec2 c = floor(rp / G);
  vec2 f = fract(rp / G);
  float h = hash2(c + scale * 19.3 + uSeed * 0.03);
  if (h > dens * 0.65) return 0.0;
  float xPos = 0.2 + 0.6 * hash2(c + 4.1);
  float yStart = 0.1 + 0.3 * hash2(c + 7.3);
  float yLen = 0.35 + 0.35 * hash2(c + 9.7);
  float dx = abs(f.x - xPos) * G.x;
  float dy = f.y - yStart;
  float inY = ss(0.0, 0.08, dy) * (1.0 - ss(yLen - 0.08, yLen, dy));
  return (1.0 - ss(0.35, 1.15, dx)) * inY * (0.45 + 0.45 * (dy / max(yLen, 0.1)));
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y * uScale - gl_FragCoord.y) / uScale;
  float t = uTime;
  vec2 gp = floor(p * (0.8 * max(uScale, 1.0)));
  float g1 = hash2(gp + 0.5), g2 = hash2(gp + 7.3);

  float s0 = mix(fieldS(uFA, p), fieldS(uFB, p), uMix);
  float s = s0 + (N(p, 580.0).r - 0.5) * 11.0;

  vec3 col;
  if (s <= 0.0) {
    // ---------- Khoảng trống rêu ẩm giữa rừng (vùng biểu tượng) ----------
    float w1 = N(p, 1600.0).b, w2 = N(p + 271.0, 580.0).g;
    col = mix(MOSS_M, MOSS_L, ss(0.28, 0.74, w1 * 0.68 + w2 * 0.32));
    col = mix(col, MOSS_D, ss(0.48, 0.86, N(p + 813.0, 1100.0).r) * 0.38);
    col = mix(col, INK, mossInk(p) * 0.65 * ss(-2.0, -14.0, s));
    // Chuyển tiếp bờ rêu đậm ẩm ướt sát bìa rừng
    float edgeWet = ss(-14.0, 0.0, s);
    col = mix(col, FLOOR_WET, edgeWet * 0.48);
    // Vòng gợn nước nhỏ khi mưa rơi trên thảm rêu ẩm
    if (uRain > 0.01) {
      float rip = rainRipples(p, t);
      col = mix(col, vec3(0.78, 0.90, 0.76), rip * 0.38);
    }
  } else {
    // ---------- Nền rừng đọng nước và tán cây nhiều tầng ----------
    float dNorm = clamp(s / 240.0, 0.0, 1.0);
    col = mix(CANOPY_0, FLOOR_WET, dNorm * 0.7);

    // Gợn sóng hạt mưa ở vùng nước đọng dưới tán rừng
    if (uRain > 0.01) {
      float rip = rainRipples(p + 19.0, t * 1.08);
      col = mix(col, vec3(0.45, 0.72, 0.64), rip * 0.52);
    }

    // Hai tầng tán cây rừng mưa nhiệt đới đan xen
    float nLobe = (N(p, 44.0).g - 0.5) * 0.22;
    vec4 L1 = canopyLayer(p, 58.0, 11.0, s, 2.0, t, nLobe);
    if (L1.a > 0.001) {
      col = mix(col * 0.85, L1.rgb * 0.88, L1.a);
    }
    vec4 L2 = canopyLayer(p + vec2(25.0, 19.0), 40.0, 37.0, s, 0.0, t, -nLobe);
    if (L2.a > 0.001) {
      col *= 1.0 - 0.15 * L2.a;
      col = mix(col, L2.rgb * 1.03, L2.a);
    }
  }

  // ---------- Chim nhiệt đới ----------
  for (int i = 0; i < 8; i++) {
    if (float(i) >= uBirdN) break;
    vec4 b = uBird[i];
    vec2 d0 = p - b.xy, d1 = d0 - vec2(24.0, 36.0);
    if (dot(d1, d1) < 360.0) {
      vec3 g = birdSD(rot(-b.z) * d1, b.w);
      col = mix(col, col * 0.68, ss(1.8, -1.8, min(g.x, g.y)) * 0.32);
    }
    if (dot(d0, d0) < 360.0) {
      vec3 g = birdSD(rot(-b.z) * d0, b.w);
      vec3 wingC = mix(vec3(0.18, 0.78, 0.52), vec3(0.12, 0.45, 0.68), ss(0.35, 0.85, g.z));
      col = mix(col, wingC, ss(0.6, -0.6, g.y));
      col = mix(col, vec3(0.96, 0.82, 0.24), ss(0.6, -0.6, g.x));
      col = mix(col, vec3(0.95, 0.36, 0.22), ss(0.5, -0.5, length(rot(-b.z) * d0 - vec2(6.2, 0.0)) - 1.1));
    }
  }

  // ---------- Sương mù trôi qua rừng mưa ----------
  if (uMist > 0.01) {
    vec2 mp1 = p + vec2(-t * 14.0, sin(p.x * 0.004 + t * 0.2) * 18.0);
    vec2 mp2 = p + vec2(-t * 22.0 + 400.0, cos(p.x * 0.003 - t * 0.15) * 24.0);
    float m1 = N(mp1, 920.0).r * N(mp2, 480.0).g;
    float mistA = ss(0.22, 0.62, m1) * uMist * (0.25 + 0.35 * ss(-20.0, 90.0, s));
    col = mix(col, vec3(0.84, 0.93, 0.89), mistA);
  }

  // ---------- Đom đóm trong rừng ----------
  for (int i = 0; i < 15; i++) {
    if (float(i) >= uFlyN) break;
    vec4 fl = uFly[i];
    vec2 dp = p - fl.xy;
    float d2 = dot(dp, dp);
    if (d2 < 900.0) {
      float d = sqrt(d2);
      float pulse = 0.55 + 0.45 * sin(t * 3.2 + fl.z);
      float glow = exp(-d * 0.14) * pulse * fl.w;
      float core = (1.0 - ss(1.2, 2.8, d)) * pulse * fl.w;
      col += vec3(0.68, 0.96, 0.32) * glow * 0.55;
      col = mix(col, vec3(0.96, 1.0, 0.72), core);
    }
  }

  // ---------- Hạt mưa rơi xiên dày đặc ----------
  if (uRain > 0.01) {
    float r1 = rainStreaks(p, t, uRain, 680.0, 1.0);
    float r2 = rainStreaks(p + vec2(73.0, 41.0), t, uRain * 0.75, 520.0, 0.78);
    float rAmt = max(r1, r2 * 0.75);
    col = mix(col, vec3(0.86, 0.95, 0.92), rAmt * 0.42);
  }

  col *= 1.0 + (g2 - 0.5) * 0.03;
  if (uDebug > 0.5) col = mix(col, vec3(0.9, 0.15, 0.15), 1.0 - ss(0.0, 1.5, abs(s0)));
  o = vec4(col, 1.0);
}
