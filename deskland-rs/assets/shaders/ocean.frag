#version 300 es
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uDebug, uEdge;
uniform highp sampler2D uCirc;
uniform vec2 uCGrid;
uniform float uCellG;
uniform float uBoatN;
uniform vec4 uBoat[6];       // x, y, huong, pha
uniform float uYachtN;
uniform vec4 uYacht[4];      // x, y, huong, pha
uniform float uBirdN;
uniform vec4 uBird[8];
out vec4 o;

const vec3 SEA_LIGHT = vec3(0.24, 0.56, 0.90), SEA_MID = vec3(0.16, 0.45, 0.83), SEA_DEEP = vec3(0.11, 0.36, 0.74);
const vec3 SHADOW = vec3(0.48, 0.58, 0.78);
const vec3 CLOUD_HI = vec3(1.0, 1.0, 1.0);
const vec3 CLOUD_BODY = vec3(0.89, 0.94, 0.98), CLOUD_SH = vec3(0.76, 0.84, 0.93);
const vec2 LIGHT = vec2(-0.6, -0.8);
const vec2 SHADOW_OFF = vec2(115.0, 156.0);
const int CIRC_K = 36;

float hash2(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}
float ss(float a, float b, float x) { float t = clamp((x - a) / (b - a), 0.0, 1.0); return t * t * (3.0 - 2.0 * t); }
vec4 N(vec2 p, float scale) { return texture(uNoise, p / scale + uSeed * 0.013); }
float fieldS(sampler2D t, vec2 p) { return texture(t, (p / uStep + 0.5) / uGrid).r; }
float smin(float a, float b, float k) { float h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0); return mix(b, a, h) - k * h * (1.0 - h); }
mat2 rot(float a) { float c = cos(a), s = sin(a); return mat2(c, s, -s, c); }

float cloudDist(vec2 q, bool front, inout vec4 fc) {
  float sF = mix(fieldS(uFA, q), fieldS(uFB, q), uMix);
  float d = sF - uEdge + 0.6 * 95.0;
  ivec2 cell = ivec2(floor(q / uCellG)) + 4;
  if (cell.x < 0 || cell.y < 0 || float(cell.x) >= uCGrid.x || float(cell.y) >= uCGrid.y) return d;
  for (int k = 0; k < CIRC_K; k++) {
    vec4 c = texelFetch(uCirc, ivec2(cell.x * CIRC_K + k, cell.y), 0);
    if (c.z <= 0.0) break;
    float di = length(q - c.xy) - c.z;
    d = smin(d, di, 6.0);
    if (front && di < 0.0 && c.y > fc.y) fc = c;
  }
  return d;
}
float cloudDist(vec2 q) { vec4 fc = vec4(0.0, -1e9, 0.0, 0.0); return cloudDist(q, false, fc); }

// Thuyền buồm (nhìn từ trên xuống, x hướng về phía trước): thân thuyền + cánh buồm trắng căng gió
vec2 sailboatSD(vec2 q, float ph) {
  float L = 16.0, Wd = 4.6;
  float t = clamp(q.x / L, -1.0, 1.0);
  float w = Wd * (1.0 - t * t * 0.85);
  float hull = max(abs(q.y) - w, abs(q.x) - L);
  vec2 sq = q - vec2(1.5, 1.2 + sin(ph) * 0.4);
  float sail = (length(sq / vec2(9.5, 3.2)) - 1.0) * 3.2;
  return vec2(hull, sail);
}

// Du thuyền (nhìn từ trên xuống, x hướng về phía trước): thân thuôn + kính chắn gió xám đậm + sóng đuôi
float yachtHullSD(vec2 q) {
  float L = 21.0, Wd = 6.2;
  float t = clamp(q.x / L, -1.0, 1.0);
  float w = t > 0.0 ? Wd * (1.0 - t * t) : Wd * (0.82 + 0.18 * (1.0 + t));
  return max(abs(q.y) - w, abs(q.x) - L);
}

vec3 gullSD(vec2 q, float span) {
  float body = (length((q - vec2(0.8, 0.0)) / vec2(6.2, 1.9)) - 1.0) * 1.9;
  float tail = max(abs(q.y) - 1.4 - (-q.x - 4.0) * 0.25, max(-q.x - 8.5, q.x + 4.0));
  float y = abs(q.y), reach = 15.0 * span;
  float u = clamp((y - 1.0) / reach, 0.0, 1.0);
  float lead = 1.0 + 1.3 * sin(u * 3.14159) - 4.0 * u * u;
  float chord = mix(4.4, 1.3, u);
  float wing = max(max(q.x - lead, lead - chord - q.x), y - reach - 1.0);
  return vec3(min(body, tail), wing, u);
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y * uScale - gl_FragCoord.y) / uScale;
  float t = uTime;

  // ---------- Mặt biển: màu nước xanh lam đậm nhạt + ánh nước lấp lánh và hạt giấy ----------
  vec2 drift = vec2(t * 2.4, t * 0.9);
  float w1 = N(p + drift * 2.5, 1800.0).b, w2 = N(p - drift * 1.5 + 311.0, 620.0).g;
  vec3 col = mix(SEA_MID, SEA_LIGHT, ss(0.28, 0.74, w1 * 0.7 + w2 * 0.3));
  col = mix(col, SEA_DEEP, ss(0.45, 0.85, N(p + 977.0, 1300.0).r) * 0.42);
  col *= 1.0 + 0.05 * (0.5 - (p.x + p.y) / (uRes.x + uRes.y));

  // Gợn sóng nhỏ và đốm nắng lấp lánh trên mặt biển
  float waveTex = (N(p * vec2(1.0, 2.2) + drift * 5.0, 180.0).r - 0.5) * 0.04;
  col += waveTex;
  float glitter = ss(0.38, 0.75, N(p + drift * 3.0, 520.0).g);
  float spark = 0.0;
  for (int L = 0; L < 2; L++) {
    float fl = float(L), cs = L == 0 ? 25.0 : 39.0;
    mat2 R = rot(L == 0 ? 0.62 : -1.13);
    vec2 q = R * p / cs + fl * 17.3;
    vec2 cell = floor(q), f = fract(q);
    vec2 seedc = cell + fl * 51.7 + uSeed * 0.1;
    if (hash2(seedc) > (L == 0 ? 0.10 : 0.14) * glitter) continue;
    vec2 c = 0.15 + 0.7 * vec2(hash2(seedc + 1.3), hash2(seedc + 2.9));
    float life = fract(t * (0.3 + 0.35 * hash2(seedc + 8.1)) + hash2(seedc + 5.7));
    float tw = ss(0.0, 0.1, life) * (1.0 - ss(0.2, 0.45, life));
    float r = 0.7 + 0.9 * hash2(seedc + 3.3);
    spark = max(spark, (1.0 - ss(r * 0.5, r * 1.5, length((f - c) * cs))) * tw);
  }
  col = mix(col, vec3(0.92, 0.98, 1.0), spark * 0.75);
  col *= 1.0 + (hash2(floor(p * (0.8 * max(uScale, 1.0)))) - 0.5) * 0.032;

  // ---------- Thuyền buồm ----------
  for (int i = 0; i < 6; i++) {
    if (float(i) >= uBoatN) break;
    vec4 b = uBoat[i];
    vec2 d0 = p - b.xy;
    if (dot(d0, d0) > 1600.0) continue;
    vec2 q = rot(-b.z) * d0;
    // Vệt nước phía sau thuyền
    if (q.x < -8.0 && q.x > -38.0) {
      float wk = (1.0 - ss(-8.0, -38.0, q.x)) * (1.0 - ss(1.0, 6.0, abs(abs(q.y) - (-q.x - 8.0) * 0.22)));
      col = mix(col, vec3(0.85, 0.93, 0.99), wk * 0.45);
    }
    vec2 qs = rot(-b.z) * (d0 - vec2(6.0, 9.0));
    vec2 sds = sailboatSD(qs, b.w);
    col = mix(col, col * SHADOW, ss(1.4, -1.0, min(sds.x, sds.y)) * 0.65);
    vec2 sd = sailboatSD(q, b.w);
    if (sd.x < 0.8) {
      vec3 hullC = mix(vec3(0.95, 0.97, 0.99), vec3(0.76, 0.82, 0.89), ss(-3.0, 0.0, sd.x));
      col = mix(col, hullC, ss(0.6, -0.6, sd.x));
    }
    if (sd.y < 0.8) {
      vec3 sailC = mix(vec3(1.0, 1.0, 1.0), vec3(0.86, 0.91, 0.96), ss(-1.5, 1.5, q.y));
      col = mix(col, sailC, ss(0.6, -0.6, sd.y));
    }
  }

  // ---------- Du thuyền (kèm vệt sóng bọt trắng) ----------
  for (int i = 0; i < 4; i++) {
    if (float(i) >= uYachtN) break;
    vec4 y = uYacht[i];
    vec2 d0 = p - y.xy;
    if (dot(d0, d0) > 3600.0) continue;
    vec2 q = rot(-y.z) * d0;
    if (q.x < -14.0 && q.x > -56.0) {
      float distBack = -q.x - 14.0;
      float wakeW = 2.5 + distBack * 0.26;
      float edgeWake = 1.0 - ss(0.8, 3.2, abs(abs(q.y) - wakeW));
      float centerFoam = 1.0 - ss(0.0, wakeW * 0.85, abs(q.y));
      float fade = 1.0 - ss(0.0, 42.0, distBack);
      col = mix(col, vec3(0.94, 0.98, 1.0), max(edgeWake * 0.65, centerFoam * 0.42) * fade);
    }
    vec2 qs = rot(-y.z) * (d0 - vec2(7.0, 10.0));
    col = mix(col, col * SHADOW, ss(1.5, -1.2, yachtHullSD(qs)) * 0.68);
    float yh = yachtHullSD(q);
    if (yh < 0.8) {
      vec3 yc = mix(vec3(0.98, 0.99, 1.0), vec3(0.82, 0.86, 0.91), ss(-4.0, 0.0, yh));
      // Kính chắn gió xám đậm
      float win = (length((q - vec2(4.0, 0.0)) / vec2(4.5, 3.8)) - 1.0) * 3.8;
      if (q.x > 1.0) yc = mix(yc, vec3(0.24, 0.29, 0.36), ss(0.5, -0.5, win));
      // Khu vực ghế ngồi boong sau xám nhạt
      float deck = max(abs(q.y) - 3.6, max(q.x + 2.0, -q.x - 15.0));
      yc = mix(yc, vec3(0.85, 0.88, 0.92), ss(0.5, -0.5, deck));
      col = mix(col, yc, ss(0.6, -0.6, yh));
    }
  }

  // ---------- Hải âu ----------
  for (int i = 0; i < 8; i++) {
    if (float(i) >= uBirdN) break;
    vec4 b = uBird[i];
    vec2 d0 = p - b.xy, d1 = d0 - vec2(34.0, 55.0);
    if (dot(d1, d1) < 400.0) {
      vec3 g = gullSD(rot(-b.z) * d1, b.w);
      col = mix(col, col * SHADOW, ss(1.8, -1.8, min(g.x, g.y)) * 0.35);
    }
    if (dot(d0, d0) < 400.0) {
      vec3 g = gullSD(rot(-b.z) * d0, b.w);
      vec3 wingC = mix(vec3(0.86, 0.90, 0.94), vec3(0.22, 0.25, 0.30), ss(0.72, 0.8, g.z));
      col = mix(col, wingC, ss(0.6, -0.6, g.y));
      col = mix(col, vec3(0.99, 0.99, 1.0), ss(0.6, -0.6, g.x));
    }
  }

  // ---------- Bóng mây xanh đậm đổ trên mặt biển ----------
  float sd = cloudDist(p - SHADOW_OFF);
  col = mix(col, col * SHADOW, ss(3.0, -3.0, sd) * 0.82);

  // ---------- Mây trắng ----------
  vec4 fc = vec4(0.0, -1e9, 0.0, 0.0);
  float d = cloudDist(p, true, fc);
  if (d < 1.5) {
    float hi = d > -21.0 ? ss(-1.0, 1.0, cloudDist(p + LIGHT * 20.0)) : 0.0;
    vec3 cc = fc.y > -1e8 ? mix(CLOUD_BODY, CLOUD_SH, fc.w * 0.6) : mix(CLOUD_BODY, CLOUD_SH, 0.45);
    cc = mix(cc, CLOUD_HI, hi);
    col = mix(col, cc, ss(1.0, -1.0, d));
  }
  if (uDebug > 0.5) col = mix(col, vec3(0.9, 0.15, 0.15), 1.0 - ss(0.0, 1.5, abs(d)));
  o = vec4(col, 1.0);
}
