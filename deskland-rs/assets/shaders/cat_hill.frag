#version 300 es
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uDebug;
uniform float uWind;
uniform float uTreeShake;
uniform float uCatN;
uniform vec4 uCatPos[15];   // x, y, sc * dir (sign = dir), ph
uniform vec4 uCatMeta[15];  // mode (0=curled, 1=belly, 2=groom, 3=walk), furIdx (0..7), happy (0..2.6), 0
uniform vec4 uToy;          // Cuộn len: x, y, age, ttl
uniform float uBflyN;
uniform vec4 uBfly[6];      // x, y, vx_sign, ph
uniform float uLeafN;
uniform vec4 uLeaf[28];     // x, y, rot, sc (sign: >0 air, <0 ground with fade encoded)
out vec4 o;

const vec3 OUTLINE = vec3(0.282, 0.212, 0.165); // #48362a
const float PI = 3.14159265;

float hash2(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}
float ss(float a, float b, float x) {
  float t = clamp((x - a) / (b - a), 0.0, 1.0);
  return t * t * (3.0 - 2.0 * t);
}
mat2 rot(float a) { float c = cos(a), s = sin(a); return mat2(c, s, -s, c); }
float sdEllipse(vec2 p, vec2 r) {
  return (length(p / r) - 1.0) * min(r.x, r.y);
}
float sdSeg(vec2 p, vec2 a, vec2 b) {
  vec2 pa = p - a, ba = b - a;
  return length(pa - ba * clamp(dot(pa, ba) / max(dot(ba, ba), 1e-4), 0.0, 1.0));
}
float sdBox(vec2 p, vec2 b, float r) {
  vec2 d = abs(p) - b + r;
  return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0) - r;
}
float sdTri(vec2 p, vec2 p0, vec2 p1, vec2 p2) {
  vec2 e0 = p1 - p0, e1 = p2 - p1, e2 = p0 - p2;
  vec2 v0 = p - p0, v1 = p - p1, v2 = p - p2;
  vec2 pq0 = v0 - e0 * clamp(dot(v0, e0) / dot(e0, e0), 0.0, 1.0);
  vec2 pq1 = v1 - e1 * clamp(dot(v1, e1) / dot(e1, e1), 0.0, 1.0);
  vec2 pq2 = v2 - e2 * clamp(dot(v2, e2) / dot(e2, e2), 0.0, 1.0);
  float s = sign(e0.x * e2.y - e0.y * e2.x);
  vec2 d = min(min(vec2(dot(pq0, pq0), s * (v0.x * e0.y - v0.y * e0.x)),
                   vec2(dot(pq1, pq1), s * (v1.x * e1.y - v1.y * e1.x))),
                   vec2(dot(pq2, pq2), s * (v2.x * e2.y - v2.y * e2.x)));
  return -sqrt(d.x) * sign(d.y);
}

void strokeFill(inout vec3 col, float d, vec3 fillC, vec3 strokeC, float sw, float alpha) {
  float aFill = ss(0.7, -0.7, d) * alpha;
  float aOut = ss(sw + 0.7, sw - 0.7, d) * alpha;
  col = mix(col, strokeC, aOut);
  col = mix(col, fillC, aFill);
}

float hillY(float nx, float H) {
  return H * (0.295 - 0.024 * sin(nx * PI * 1.15 - 0.25) + 0.010 * cos(nx * PI * 2.4 + 0.6));
}

// Đám mây trắng phồng đáy phẳng
void drawCloud(inout vec3 col, vec2 p, vec2 ctr, float s) {
  vec2 q = (p - ctr) / s;
  if (abs(q.x) > 80.0 || abs(q.y) > 45.0) return;
  float d = length(q - vec2(-32.0, 3.0)) - 17.0;
  d = min(d, length(q - vec2(-12.0, -7.0)) - 23.0);
  d = min(d, length(q - vec2(14.0, -4.0)) - 20.0);
  d = min(d, length(q - vec2(34.0, 4.0)) - 15.0);
  d = min(d, sdBox(q - vec2(1.0, 6.0), vec2(33.0, 10.0), 4.0));
  d = max(d, q.y - 13.0);
  if (d < 2.0) {
    float ds = length(q - vec2(-30.0, 7.0)) - 14.0;
    ds = min(ds, length(q - vec2(0.0, 6.0)) - 18.0);
    ds = min(ds, length(q - vec2(30.0, 7.0)) - 13.0);
    ds = max(ds, q.y - 13.0);
    ds = max(ds, 3.0 - q.y);
    vec3 cc = mix(vec3(1.0), vec3(0.871, 0.937, 0.984), ss(1.0, -1.0, ds));
    col = mix(col, cc, ss(1.0, -1.0, d * s));
  }
}

// Cụm tán lá tròn hoạt hình
void drawLeafCluster(inout vec3 col, vec2 p, vec2 ctr, float r, int tone) {
  vec2 q = p - ctr;
  if (dot(q, q) > (r * 1.45) * (r * 1.45)) return;
  vec3 cMain = tone == 0 ? vec3(0.420, 0.722, 0.259) : (tone == 1 ? vec3(0.514, 0.808, 0.306) : vec3(0.620, 0.871, 0.353));
  vec3 cHi   = tone == 0 ? vec3(0.525, 0.812, 0.322) : (tone == 1 ? vec3(0.627, 0.878, 0.369) : vec3(0.722, 0.925, 0.439));
  vec3 cSh   = tone == 0 ? vec3(0.329, 0.604, 0.200) : (tone == 1 ? vec3(0.408, 0.690, 0.235) : vec3(0.494, 0.769, 0.275));

  float d = length(q) - r * 0.78;
  for (int i = 0; i < 6; i++) {
    float a = float(i) * (PI * 2.0 / 6.0) + 0.3;
    vec2 bp = vec2(cos(a) * r * 0.48, sin(a) * r * 0.42);
    d = min(d, length(q - bp) - r * 0.56);
  }
  if (d < 1.5) {
    vec3 lc = cMain;
    float dSh = length(q - vec2(r * 0.08, r * 0.22)) - r * 0.72;
    lc = mix(lc, cSh, ss(1.0, -1.0, max(dSh, d)) * 0.45);
    float dHi = length(q - vec2(-r * 0.18, -r * 0.24)) - r * 0.48;
    lc = mix(lc, cHi, ss(1.0, -1.0, dHi) * 0.55);
    col = mix(col, lc, ss(1.0, -1.0, d));
  }
}

// Cây thân gỗ nhỏ trên đồi
void drawMidTree(inout vec3 col, vec2 p, vec2 base, float s, float sway) {
  vec2 q = p - base;
  if (abs(q.x) > 85.0 * s || q.y < -115.0 * s || q.y > 20.0 * s) return;
  float sh = sdEllipse(q - vec2(4.0 * s, 2.0 * s), vec2(44.0 * s, 11.0 * s));
  col = mix(col, vec3(0.30, 0.52, 0.16), ss(1.0, -1.0, sh) * 0.22);

  // Thân cây
  if (q.y <= 2.0 * s && q.y >= -52.0 * s) {
    float ty = clamp(-q.y / (52.0 * s), 0.0, 1.0);
    float tw = mix(10.0, 5.5, ty) * s;
    float td = max(abs(q.x - sway * 0.25 * ty) - tw, max(q.y - 2.0 * s, -52.0 * s - q.y));
    strokeFill(col, td, vec3(0.541, 0.353, 0.212), OUTLINE, 1.6, 1.0);
  }
  drawLeafCluster(col, p, base + vec2(-22.0 * s + sway * 0.8, -50.0 * s), 25.0 * s, 0);
  drawLeafCluster(col, p, base + vec2( 22.0 * s + sway * 0.8, -48.0 * s), 24.0 * s, 0);
  drawLeafCluster(col, p, base + vec2(  0.0     + sway,       -64.0 * s), 30.0 * s, 1);
  drawLeafCluster(col, p, base + vec2(-12.0 * s + sway * 1.1, -72.0 * s), 22.0 * s, 2);
  drawLeafCluster(col, p, base + vec2( 14.0 * s + sway * 1.1, -70.0 * s), 21.0 * s, 2);
}

// Đá cuội
void drawRock(inout vec3 col, vec2 p, vec2 ctr, float s) {
  vec2 q = p - ctr;
  if (dot(q, q) > (36.0 * s) * (36.0 * s)) return;
  float sh = sdEllipse(q - vec2(2.0 * s, 5.0 * s), vec2(24.0 * s, 8.0 * s));
  col = mix(col, vec3(0.30, 0.52, 0.16), ss(1.0, -1.0, sh) * 0.20);
  float d = sdEllipse(q, vec2(21.0 * s, 12.0 * s));
  vec3 rc = mix(vec3(0.596, 0.635, 0.667), vec3(0.745, 0.780, 0.804), ss(4.0 * s, -4.0 * s, q.y + 3.0 * s));
  strokeFill(col, d, rc, OUTLINE, 1.5, 1.0);
}

// Đệm hồng cho mèo nằm
void drawCushion(inout vec3 col, vec2 p, vec2 ctr, float s) {
  vec2 q = p - ctr;
  if (dot(q, q) > (40.0 * s) * (40.0 * s)) return;
  float sh = sdEllipse(q - vec2(2.0 * s, 4.0 * s), vec2(27.0 * s, 10.0 * s));
  col = mix(col, vec3(0.30, 0.52, 0.16), ss(1.0, -1.0, sh) * 0.22);
  float dOuter = sdEllipse(q, vec2(25.0 * s, 11.0 * s));
  strokeFill(col, dOuter, vec3(0.957, 0.545, 0.631), OUTLINE, 1.6, 1.0);
  float dInner = sdEllipse(q - vec2(0.0, -1.5 * s), vec2(17.0 * s, 6.8 * s));
  col = mix(col, vec3(0.988, 0.722, 0.780), ss(0.8, -0.8, dInner));
}

// Màu lông mèo theo furIdx (0..7)
void getCatColors(int idx, out vec3 baseC, out vec3 markC, out vec3 bellyC, out int pat) {
  // pat: 0=tabby, 1=solid, 2=calico, 3=siamese
  if (idx == 0) { // orange tabby
    baseC = vec3(0.961, 0.659, 0.333); markC = vec3(0.831, 0.447, 0.165); bellyC = vec3(0.996, 0.925, 0.839); pat = 0;
  } else if (idx == 1) { // cream
    baseC = vec3(0.969, 0.839, 0.643); markC = vec3(0.855, 0.678, 0.455); bellyC = vec3(1.000, 0.969, 0.910); pat = 0;
  } else if (idx == 2) { // calico
    baseC = vec3(0.996, 0.980, 0.957); markC = vec3(0.929, 0.569, 0.227); bellyC = vec3(0.996, 0.980, 0.957); pat = 2;
  } else if (idx == 3) { // grey tabby
    baseC = vec3(0.620, 0.659, 0.706); markC = vec3(0.431, 0.467, 0.514); bellyC = vec3(0.910, 0.925, 0.941); pat = 0;
  } else if (idx == 4) { // black
    baseC = vec3(0.227, 0.227, 0.259); markC = vec3(0.145, 0.145, 0.173); bellyC = vec3(0.314, 0.314, 0.345); pat = 1;
  } else if (idx == 5) { // white
    baseC = vec3(0.992, 0.984, 0.973); markC = vec3(0.933, 0.910, 0.886); bellyC = vec3(1.000, 1.000, 1.000); pat = 1;
  } else if (idx == 6) { // siamese
    baseC = vec3(0.949, 0.886, 0.792); markC = vec3(0.353, 0.251, 0.200); bellyC = vec3(0.973, 0.933, 0.863); pat = 3;
  } else { // brown tabby
    baseC = vec3(0.690, 0.525, 0.369); markC = vec3(0.455, 0.322, 0.208); bellyC = vec3(0.949, 0.886, 0.792); pat = 0;
  }
}

// Chữ z Z Z bay lên khi mèo ngủ
void drawSleepZ(inout vec3 col, vec2 q, float ph) {
  for (int i = 0; i < 3; i++) {
    float a = fract(ph * 0.22 + float(i) * 0.33);
    vec2 zp = vec2(12.0 + float(i) * 6.0 + sin(a * PI * 2.0 + float(i)) * 3.0, -16.0 - a * 24.0);
    float sz = (3.2 + float(i) * 1.1) * (0.6 + 0.4 * sin(a * PI));
    float alpha = sin(a * PI) * 0.85;
    vec2 lq = q - zp;
    if (abs(lq.x) < sz * 1.4 && abs(lq.y) < sz * 1.4) {
      float d = min(sdSeg(lq, vec2(-sz * 0.5, -sz * 0.5), vec2(sz * 0.5, -sz * 0.5)),
                min(sdSeg(lq, vec2(sz * 0.5, -sz * 0.5), vec2(-sz * 0.5, sz * 0.5)),
                    sdSeg(lq, vec2(-sz * 0.5, sz * 0.5), vec2(sz * 0.5, sz * 0.5))));
      strokeFill(col, d - 0.85, vec3(1.0), OUTLINE, 0.9, alpha);
    }
  }
}

// Trái tim hồng bay lên khi vuốt ve mèo
void drawHappyHearts(inout vec3 col, vec2 dq, float sc, float happy) {
  float prog = clamp((2.6 - happy) / 2.6, 0.0, 1.0);
  float alpha = ss(0.0, 0.08, prog) * (1.0 - ss(0.72, 1.0, prog));
  for (int i = 0; i < 2; i++) {
    float lp = clamp(prog * 1.18 - float(i) * 0.22, 0.0, 1.0);
    if (lp <= 0.01) continue;
    float side = i == 0 ? 1.0 : -1.0;
    vec2 hc = vec2(side * (5.0 + sin(lp * 7.0) * 3.5), -24.0 - lp * 24.0);
    float sz = (i == 0 ? 6.8 : 5.0) * ss(0.0, 0.18, lp);
    vec2 hq = (dq / sc) - hc;
    if (abs(hq.x) > sz * 1.2 || abs(hq.y) > sz * 1.2) continue;
    float c1 = length(hq - vec2(-0.34 * sz, -0.25 * sz)) - 0.42 * sz;
    float c2 = length(hq - vec2( 0.34 * sz, -0.25 * sz)) - 0.42 * sz;
    float tr = sdTri(hq, vec2(-0.70 * sz, -0.04 * sz), vec2(0.0, 0.68 * sz), vec2(0.70 * sz, -0.04 * sz));
    float hd = min(min(c1, c2), tr);
    float a = alpha * (1.0 - ss(0.78, 1.0, lp));
    strokeFill(col, hd * sc, vec3(1.0, 0.44, 0.58), OUTLINE, 1.35, a);
    float hi = length(hq - vec2(-0.32 * sz, -0.32 * sz)) - 0.13 * sz;
    col = mix(col, vec3(1.0, 0.88, 0.92), ss(0.4, -0.4, hi * sc) * a);
  }
}

// Cuộn len đỏ-cam đồ chơi trên đồi cỏ
void drawYarnBall(inout vec3 col, vec2 p, vec4 toy, float s) {
  if (toy.w <= 0.0) return;
  float age = toy.z;
  float fade = ss(0.0, 0.5, toy.w);
  float bounce = abs(sin(age * 8.5)) * exp(-age * 2.8) * 18.0 * s;
  vec2 dp = p - toy.xy;
  if (dot(dp, dp) > (42.0 * s) * (42.0 * s)) return;
  vec2 q = dp / s;

  // Bóng đổ trên cỏ và sợi len vương ra ngoài
  float sh = sdEllipse(q - vec2(0.0, 7.5), vec2(11.5 - bounce * 0.15 / max(s, 0.1), 4.2));
  col = mix(col, vec3(0.28, 0.48, 0.15), ss(1.0, -1.0, sh * s) * 0.26 * fade);

  vec2 tailA = vec2(6.0, 6.5);
  vec2 tailB = vec2(16.0, 8.5 + sin(age * 4.0) * 1.8);
  vec2 tailC = vec2(23.0, 5.5 + cos(age * 4.0) * 2.0);
  float dStrand = min(sdSeg(q, tailA, tailB), sdSeg(q, tailB, tailC)) - 1.15;
  strokeFill(col, dStrand * s, vec3(0.96, 0.36, 0.34), OUTLINE, 1.0, fade);

  // Quả cầu len tròn
  vec2 bq = rot(age * 1.4 * exp(-age * 1.6)) * (q - vec2(0.0, -bounce / max(s, 0.01)));
  float r = 9.5;
  float dBall = length(bq) - r;
  if (dBall < 2.5) {
    vec3 yc = mix(vec3(0.86, 0.24, 0.25), vec3(0.98, 0.44, 0.38), ss(r, -r, bq.x * 0.5 + bq.y * 0.8));
    // Các vòng sợi len quấn đan chéo
    float w1 = abs(length(bq - vec2(-3.5, -2.0)) - 8.5) - 0.55;
    float w2 = abs(length(bq - vec2( 4.0,  2.5)) - 8.0) - 0.55;
    float w3 = abs(length(bq - vec2( 1.5, -4.5)) - 7.8) - 0.55;
    float wrap = min(w1, min(w2, w3));
    if (dBall < 0.0) {
      yc = mix(yc, OUTLINE, ss(0.6, -0.4, wrap) * 0.45);
      yc = mix(yc, vec3(1.0, 0.72, 0.66), ss(1.5, -0.5, length(bq - vec2(-3.2, -3.4)) - 2.2) * 0.55);
    }
    strokeFill(col, dBall * s, yc, OUTLINE, 1.6, fade);
  }
}

// Vẽ chú mèo hoạt hình tròn trịa
void drawCat(inout vec3 col, vec2 p, vec4 pos, vec4 meta) {
  float sc = abs(pos.z);
  float dir = pos.z >= 0.0 ? 1.0 : -1.0;
  float ph = pos.w;
  int mode = int(meta.x + 0.5);
  int furIdx = int(meta.y + 0.5);
  float happy = meta.z;

  vec2 dp = p - pos.xy;
  if (dot(dp, dp) > (66.0 * sc) * (66.0 * sc)) return;
  vec2 q = vec2(dp.x * dir, dp.y) / sc;

  vec3 baseC, markC, bellyC;
  int pat;
  getCatColors(furIdx, baseC, markC, bellyC, pat);

  // Bóng đổ dưới chân mèo
  float shD = sdEllipse(q - vec2(0.0, 11.0), vec2(24.0, 7.5));
  col = mix(col, vec3(0.28, 0.48, 0.15), ss(1.0, -1.0, shD * sc) * 0.25);

  if (mode == 0) {
    // 0: Nằm cuộn tròn ngủ (sleep_cushion / sleep_rock)
    float br = 1.0 + sin(ph * 1.6) * 0.035;
    vec2 bq = vec2(q.x / br, q.y * br);
    float dBody = sdEllipse(bq, vec2(19.5, 13.0));
    vec3 fillB = baseC;
    if (pat == 0) {
      for (int s = -1; s <= 1; s++) {
        float stripe = sdEllipse(bq - vec2(float(s) * 6.0, -7.0), vec2(2.2, 6.0));
        fillB = mix(fillB, markC, ss(0.8, -0.8, stripe));
      }
    } else if (pat == 2) {
      fillB = mix(fillB, markC, ss(0.8, -0.8, length(bq - vec2(-5.0, -3.0)) - 7.5));
      fillB = mix(fillB, vec3(0.22, 0.20, 0.19), ss(0.8, -0.8, length(bq - vec2(6.0, -1.0)) - 6.5));
    }
    strokeFill(col, dBody * sc, fillB, OUTLINE, 1.8, 1.0);

    // Đầu mèo gác lên chân
    vec2 hq = bq - vec2(10.0, 2.0);
    float dEar1 = sdTri(hq, vec2(-6.0, -6.0), vec2(-2.0, -14.5), vec2(2.0, -6.5));
    float dEar2 = sdTri(hq, vec2(2.0, -6.5), vec2(7.5, -14.0), vec2(9.5, -4.5));
    float dHead = min(sdEllipse(hq, vec2(10.5, 9.0)), min(dEar1, dEar2));
    vec3 headC = pat == 3 ? mix(baseC, markC, ss(5.0, -2.0, length(hq - vec2(2.0, 1.0)) - 4.0)) : baseC;
    strokeFill(col, dHead * sc, headC, OUTLINE, 1.7, 1.0);
    // Tai hồng bên trong
    col = mix(col, vec3(0.957, 0.675, 0.718), ss(0.5, -0.5, (dEar1 + 1.8) * sc));
    col = mix(col, vec3(0.957, 0.675, 0.718), ss(0.5, -0.5, (dEar2 + 1.8) * sc));
    // Mắt nhắm cong
    float eye = abs(length(hq - vec2(2.5, 0.5)) - 3.0) - 0.7;
    eye = max(eye, -hq.y + 0.5);
    col = mix(col, OUTLINE, ss(0.6, -0.6, eye * sc));
    // Đuôi cuộn quanh người
    float tailD = sdSeg(bq, vec2(-15.0, 6.0), vec2(4.0, 11.0)) - 3.6;
    strokeFill(col, tailD * sc, pat == 3 ? markC : baseC, OUTLINE, 1.5, 1.0);
    if (happy <= 0.01) drawSleepZ(col, q, ph);
  } else if (mode == 1) {
    // 1: Nằm ngửa phơi bụng (sleep_belly)
    float br = 1.0 + sin(ph * 1.5) * 0.04;
    vec2 bq = vec2(q.x, q.y / br);
    float dBody = sdEllipse(rot(-0.12) * bq, vec2(20.5, 12.5));
    vec3 fillB = mix(baseC, bellyC, ss(1.0, -1.0, sdEllipse(bq - vec2(0.0, 1.5), vec2(14.5, 8.0))));
    strokeFill(col, dBody * sc, fillB, OUTLINE, 1.8, 1.0);
    // Đầu ngửa sang bên
    vec2 hq = bq - vec2(-14.0, -2.0);
    float dEar1 = sdTri(hq, vec2(-8.0, -4.0), vec2(-9.5, -13.5), vec2(-2.0, -7.5));
    float dEar2 = sdTri(hq, vec2(0.0, -7.5), vec2(4.5, -14.0), vec2(7.5, -5.0));
    float dHead = min(sdEllipse(hq, vec2(10.0, 8.8)), min(dEar1, dEar2));
    strokeFill(col, dHead * sc, baseC, OUTLINE, 1.7, 1.0);
    // Bốn chân chổng lên trời
    float p1 = sdEllipse(bq - vec2(-6.0, -9.5), vec2(3.8, 5.2));
    float p2 = sdEllipse(bq - vec2( 7.0, -9.0), vec2(3.8, 5.2));
    strokeFill(col, min(p1, p2) * sc, bellyC, OUTLINE, 1.5, 1.0);
    if (happy <= 0.01) drawSleepZ(col, q, ph);
  } else if (mode == 2) {
    // 2: Ngồi liếm tay (groom)
    float bob = sin(ph * 4.5) * 1.2;
    float dTail = sdSeg(q, vec2(-10.0, 9.0), vec2(-22.0, 4.0 + sin(ph * 2.0) * 3.0)) - 3.6;
    strokeFill(col, dTail * sc, pat == 3 ? markC : baseC, OUTLINE, 1.6, 1.0);

    float dBody = sdEllipse(q - vec2(-2.0, 1.0), vec2(13.5, 14.5));
    vec3 fillB = mix(baseC, bellyC, ss(1.0, -1.0, sdEllipse(q - vec2(3.0, 3.0), vec2(7.5, 9.5))));
    strokeFill(col, dBody * sc, fillB, OUTLINE, 1.8, 1.0);

    // Đầu cúi nhẹ nhịp nhàng
    vec2 hq = q - vec2(4.0, -12.0 + bob);
    float dEar1 = sdTri(hq, vec2(-8.5, -5.0), vec2(-7.0, -16.0), vec2(-1.0, -8.5));
    float dEar2 = sdTri(hq, vec2( 1.5, -8.5), vec2( 7.5, -16.0), vec2( 9.0, -5.0));
    float dHead = min(sdEllipse(hq, vec2(11.0, 9.8)), min(dEar1, dEar2));
    vec3 headC = pat == 3 ? mix(baseC, markC, ss(4.5, -1.5, length(hq - vec2(2.0, 1.5)) - 3.8)) : baseC;
    strokeFill(col, dHead * sc, headC, OUTLINE, 1.7, 1.0);
    col = mix(col, vec3(0.957, 0.675, 0.718), ss(0.5, -0.5, (min(dEar1, dEar2) + 1.8) * sc));

    // Chân trước giơ lên mặt
    vec2 pq = hq - vec2(7.5, 4.0 - bob * 0.6);
    float dPaw = sdEllipse(rot(-0.4) * pq, vec2(5.8, 3.6));
    strokeFill(col, dPaw * sc, bellyC, OUTLINE, 1.5, 1.0);
    // Lưỡi hồng nhỏ
    float dTongue = length(hq - vec2(4.8, 3.5)) - 1.8;
    col = mix(col, vec3(0.96, 0.48, 0.56), ss(0.5, -0.5, dTongue * sc));
  } else {
    // 3: Đi dạo trên đồi (walk)
    float st = sin(ph);
    float cs = cos(ph);
    float bob = abs(st) * 1.4;

    // Đuôi cong vểnh lên ngoắc nhẹ
    vec2 t0 = vec2(-14.0, -2.0 - bob);
    vec2 t1 = vec2(-23.0 + sin(ph * 0.8) * 3.0, -16.0 - bob);
    float dTail = sdSeg(q, t0, t1) - 3.5;
    strokeFill(col, dTail * sc, pat == 3 ? markC : baseC, OUTLINE, 1.6, 1.0);

    // Hai chân phía xa
    float legB1 = sdSeg(q, vec2(-8.0, 4.0), vec2(-8.0 - st * 4.5, 12.5)) - 3.2;
    float legB2 = sdSeg(q, vec2( 7.0, 4.0), vec2( 7.0 + st * 4.5, 12.5)) - 3.2;
    strokeFill(col, min(legB1, legB2) * sc, baseC * 0.88, OUTLINE, 1.5, 1.0);

    // Thân mèo mũm mĩm
    vec2 bq = q - vec2(0.0, -1.5 - bob);
    float dBody = sdEllipse(bq, vec2(16.8, 11.2));
    vec3 fillB = mix(baseC, bellyC, ss(1.0, -1.0, sdEllipse(bq - vec2(0.0, 5.5), vec2(12.0, 5.5))));
    if (pat == 0) {
      for (int s = -1; s <= 1; s++) {
        float stripe = sdEllipse(bq - vec2(float(s) * 5.2, -5.5), vec2(2.0, 5.5));
        fillB = mix(fillB, markC, ss(0.8, -0.8, stripe));
      }
    } else if (pat == 2) {
      fillB = mix(fillB, markC, ss(0.8, -0.8, length(bq - vec2(-4.5, -2.5)) - 6.5));
      fillB = mix(fillB, vec3(0.22, 0.20, 0.19), ss(0.8, -0.8, length(bq - vec2(4.5, -1.5)) - 5.5));
    }
    strokeFill(col, dBody * sc, fillB, OUTLINE, 1.8, 1.0);

    // Hai chân phía gần
    float legF1 = sdSeg(q, vec2(-8.0, 4.0), vec2(-8.0 + st * 4.5, 12.5 - max(0.0, cs) * 1.5)) - 3.3;
    float legF2 = sdSeg(q, vec2( 7.0, 4.0), vec2( 7.0 - st * 4.5, 12.5 - max(0.0, -cs) * 1.5)) - 3.3;
    strokeFill(col, min(legF1, legF2) * sc, pat == 3 ? markC : baseC, OUTLINE, 1.5, 1.0);

    // Đầu mèo tròn
    vec2 hq = q - vec2(12.5, -8.5 - bob * 0.8);
    float dEar1 = sdTri(hq, vec2(-8.0, -5.0), vec2(-6.5, -15.5), vec2(-0.5, -8.0));
    float dEar2 = sdTri(hq, vec2( 1.5, -8.0), vec2( 7.5, -15.5), vec2( 9.0, -4.5));
    float dHead = min(sdEllipse(hq, vec2(10.8, 9.5)), min(dEar1, dEar2));
    vec3 headC = pat == 3 ? mix(baseC, markC, ss(4.5, -1.5, length(hq - vec2(2.5, 1.0)) - 3.8)) : baseC;
    strokeFill(col, dHead * sc, headC, OUTLINE, 1.7, 1.0);
    col = mix(col, vec3(0.957, 0.675, 0.718), ss(0.5, -0.5, (min(dEar1, dEar2) + 1.8) * sc));

    // Mắt, mũi hồng và má hồng
    float eye1 = length(hq - vec2(0.5, -0.5)) - 1.5;
    float eye2 = length(hq - vec2(6.2, -0.5)) - 1.5;
    col = mix(col, OUTLINE, ss(0.5, -0.5, min(eye1, eye2) * sc));
    float nose = length(hq - vec2(3.8, 2.0)) - 1.2;
    col = mix(col, vec3(0.95, 0.56, 0.62), ss(0.5, -0.5, nose * sc));
  }
  if (happy > 0.01) {
    drawHappyHearts(col, dp, sc, happy);
  }
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y * uScale - gl_FragCoord.y) / uScale;
  float W = uRes.x, H = uRes.y;
  float nx = p.x / max(W, 1.0), ny = p.y / max(H, 1.0);
  float t = uTime;
  float s = min(W, H) / 900.0;

  // ---------- 1. Bầu trời xanh chuyển sắc ----------
  float skyT = clamp(ny / 0.42, 0.0, 1.0);
  vec3 col = skyT < 0.55
    ? mix(vec3(0.431, 0.729, 0.965), vec3(0.624, 0.831, 0.984), skyT / 0.55)
    : mix(vec3(0.624, 0.831, 0.984), vec3(0.824, 0.933, 0.996), (skyT - 0.55) / 0.45);

  // ---------- 2. Mây trắng trôi ----------
  const vec4 CLOUDS[4] = vec4[4](
    vec4(-0.02, 0.14, 1.35, 0.0040),
    vec4( 0.24, 0.09, 1.05, 0.0032),
    vec4( 0.52, 0.14, 0.95, 0.0036),
    vec4( 0.78, 0.10, 1.25, 0.0028)
  );
  if (ny < 0.36) {
    for (int i = 0; i < 4; i++) {
      vec4 c = CLOUDS[i];
      float cx = (fract(c.x + t * c.w * max(0.25, uWind) + 0.12) * 1.24 - 0.12) * W;
      float cy = c.y * H;
      drawCloud(col, p, vec2(cx, cy), c.z * s);
    }
  }

  // ---------- 3. Dãy núi xa và thung lũng ----------
  float mFarY = H * (0.255 - 0.042 * sin(nx * PI * 2.2 + 0.2) - 0.022 * cos(nx * PI * 4.5 - 0.8));
  if (p.y > mFarY - 2.0) {
    col = mix(col, vec3(0.541, 0.714, 0.690), ss(1.0, -1.0, mFarY - p.y));
  }
  float mMidY = H * (0.285 - 0.032 * sin(nx * PI * 1.8 - 0.6) + 0.018 * sin(nx * PI * 3.7 + 1.1));
  if (p.y > mMidY - 2.0) {
    col = mix(col, vec3(0.416, 0.659, 0.424), ss(1.0, -1.0, mMidY - p.y));
  }

  // ---------- 4. Sườn đồi chính ngập nắng ----------
  float hY = hillY(nx, H);
  // Bụi cây dọc đường chân trời (vẽ trước sườn đồi chính một phần và nổi lên trên)
  if (abs(p.y - hY) < 42.0 * s) {
    for (int i = 0; i < 9; i++) {
      float bx = (0.04 + float(i) * 0.115) * W;
      float by = hillY(bx / W, H) + 2.0 * s;
      drawLeafCluster(col, p, vec2(bx, by - 8.0 * s), (16.0 + float(i % 3) * 3.0) * s, i % 2);
    }
  }

  if (p.y >= hY - 1.5) {
    float ht = clamp((p.y - H * 0.26) / (H * 0.74), 0.0, 1.0);
    vec3 hillC = ht < 0.45
      ? mix(vec3(0.667, 0.882, 0.365), vec3(0.596, 0.831, 0.298), ht / 0.45)
      : mix(vec3(0.596, 0.831, 0.298), vec3(0.494, 0.737, 0.235), (ht - 0.45) / 0.55);

    // Mảng nắng ấm trên đồi
    float sunPatch1 = sdEllipse(p - vec2(W * 0.36, H * 0.48), vec2(W * 0.28, H * 0.13));
    float sunPatch2 = sdEllipse(p - vec2(W * 0.68, H * 0.68), vec2(W * 0.24, H * 0.11));
    hillC = mix(hillC, vec3(0.765, 0.949, 0.431), ss(20.0, -20.0, min(sunPatch1, sunPatch2)) * 0.28);

    // Cỏ nhỏ & hoa dại trên đồi
    vec2 fg = vec2(54.0 * s, 46.0 * s);
    vec2 fc = floor(p / fg);
    float fh = hash2(fc + 17.3);
    vec2 fctr = (fc + 0.22 + 0.56 * vec2(hash2(fc + 1.9), hash2(fc + 4.3))) * fg;
    if (fctr.y > hillY(fctr.x / W, H) + 14.0 * s) {
      float swayF = sin(t * 2.2 * uWind + fctr.x * 0.02 + fctr.y * 0.01) * 2.0 * s * uWind;
      vec2 fq = p - (fctr + vec2(swayF, 0.0));
      // Bụi cỏ chữ V nhỏ
      if (fh < 0.68) {
        float gd = min(sdSeg(fq, vec2(0.0, 3.0 * s), vec2(-3.5 * s, -4.0 * s)),
                       sdSeg(fq, vec2(0.0, 3.0 * s), vec2( 3.5 * s, -4.5 * s)));
        hillC = mix(hillC, vec3(0.369, 0.624, 0.180), ss(1.1 * s, 0.2 * s, gd) * 0.75);
      }
      // Hoa dại 5 cánh nhỏ xinh
      if (fh > 0.42 && fh < 0.86) {
        vec2 flq = fq - vec2(5.0 * s, -2.0 * s);
        float flR = (3.4 + 1.4 * hash2(fc + 8.1)) * s;
        if (dot(flq, flq) < (flR * 2.2) * (flR * 2.2)) {
          float pd = 1e9;
          for (int k = 0; k < 5; k++) {
            float ka = float(k) * (PI * 2.0 / 5.0);
            pd = min(pd, length(flq - vec2(cos(ka), sin(ka)) * flR * 0.62) - flR * 0.48);
          }
          float tone = hash2(fc + 11.7);
          vec3 pCol = tone < 0.45 ? vec3(1.0, 0.992, 0.969)
                    : (tone < 0.72 ? vec3(1.0, 0.886, 0.337)
                    : (tone < 0.90 ? vec3(1.0, 0.620, 0.733) : vec3(0.561, 0.816, 1.0)));
          hillC = mix(hillC, pCol, ss(0.7, -0.7, pd));
          hillC = mix(hillC, vec3(1.0, 0.745, 0.196), ss(0.6, -0.6, length(flq) - flR * 0.36));
        }
      }
    }

    col = mix(col, hillC, ss(1.0, -1.0, hY - p.y));
  }

  // ---------- 5. Đá cuội, đệm hồng và cây trên đồi ----------
  float swayTree = sin(t * 1.6 * uWind) * 3.5 * s * uWind + sin(t * 20.0) * 4.5 * s * uTreeShake;
  drawRock(col, p, vec2(W * 0.13, H * 0.45), 0.95 * s);
  drawRock(col, p, vec2(W * 0.88, H * 0.47), 0.90 * s);
  drawRock(col, p, vec2(W * 0.43, H * 0.63), 0.85 * s);
  drawCushion(col, p, vec2(W * 0.26, H * 0.57), 1.05 * s);

  // Các bụi cây nhỏ trên đồi
  drawLeafCluster(col, p, vec2(W * 0.36 + swayTree * 0.4, H * 0.46), 19.0 * s, 1);
  drawLeafCluster(col, p, vec2(W * 0.64 + swayTree * 0.4, H * 0.45), 21.0 * s, 0);
  drawLeafCluster(col, p, vec2(W * 0.48 + swayTree * 0.5, H * 0.56), 18.0 * s, 2);
  drawLeafCluster(col, p, vec2(W * 0.74 + swayTree * 0.5, H * 0.54), 22.0 * s, 1);

  // Cây tròn trên sườn đồi (giống hệt ảnh màn hình của user)
  drawMidTree(col, p, vec2(W * 0.18, H * 0.69), 0.95 * s, swayTree);
  drawMidTree(col, p, vec2(W * 0.32, H * 0.63), 1.05 * s, -swayTree * 0.9);
  drawMidTree(col, p, vec2(W * 0.72, H * 0.66), 1.00 * s, swayTree * 0.85);
  drawMidTree(col, p, vec2(W * 0.86, H * 0.64), 0.98 * s, -swayTree);

  // ---------- 6. Đàn mèo và cuộn len đồ chơi trên đồi ----------
  bool toyDrawn = uToy.w <= 0.0;
  for (int i = 0; i < 15; i++) {
    if (float(i) >= uCatN) break;
    if (!toyDrawn && uCatPos[i].y >= uToy.y) {
      drawYarnBall(col, p, uToy, s);
      toyDrawn = true;
    }
    drawCat(col, p, uCatPos[i], uCatMeta[i]);
  }
  if (!toyDrawn) {
    drawYarnBall(col, p, uToy, s);
  }

  // ---------- 7. Tán cây lớn viền phải & viền trái khung hình ----------
  if (nx > 0.90 && ny < 0.52) {
    for (int i = 0; i < 5; i++) {
      float cy = (0.08 + float(i) * 0.085) * H;
      drawLeafCluster(col, p, vec2(W * 0.985 + swayTree * 0.6, cy), 42.0 * s, i % 3);
    }
  }
  if (nx < 0.09 && ny < 0.40) {
    for (int i = 0; i < 4; i++) {
      float cy = (0.10 + float(i) * 0.075) * H;
      drawLeafCluster(col, p, vec2(W * 0.015 - swayTree * 0.5, cy), 36.0 * s, (i + 1) % 3);
    }
  }

  // ---------- 8. Bướm vàng bay lượn ----------
  for (int i = 0; i < 6; i++) {
    if (float(i) >= uBflyN) break;
    vec4 bf = uBfly[i];
    vec2 bq = (p - bf.xy) / s;
    if (dot(bq, bq) > 260.0) continue;
    bq.x *= bf.z >= 0.0 ? 1.0 : -1.0;
    float flap = 0.35 + 0.65 * abs(sin(bf.w));
    float w1 = sdEllipse(vec2(bq.x - 2.5, bq.y / flap + 2.0), vec2(5.2, 4.2));
    float w2 = sdEllipse(vec2(bq.x + 2.0, bq.y / flap - 1.5), vec2(4.0, 3.4));
    strokeFill(col, min(w1, w2) * s, vec3(1.0, 0.878, 0.333), OUTLINE, 1.1, 1.0);
  }

  // ---------- 9. Lá xanh bay trong gió ----------
  for (int i = 0; i < 28; i++) {
    if (float(i) >= uLeafN) break;
    vec4 lf = uLeaf[i];
    float fade = abs(lf.w);
    if (fade <= 0.01) continue;
    vec2 lq = rot(-lf.z) * (p - lf.xy) / s;
    if (dot(lq, lq) > 120.0) continue;
    float ld = sdEllipse(lq, vec2(5.2, 2.7));
    strokeFill(col, ld * s, vec3(0.545, 0.847, 0.314), OUTLINE, 1.0, min(1.0, fade));
  }

  if (uDebug > 0.5) {
    col = mix(col, vec3(0.9, 0.15, 0.15), 1.0 - ss(0.0, 1.5, abs(p.y - hY)));
  }
  o = vec4(col, 1.0);
}
