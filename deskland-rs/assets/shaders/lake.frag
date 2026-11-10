#version 300 es
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uShoal, uDepth, uFlow, uDebug;
uniform highp sampler2D uCirc;
uniform vec2 uCGrid;
uniform float uCellG;
out vec4 o;

// Bảng màu cỏ màu nước
const vec3 G_LIGHT = vec3(0.75, 0.82, 0.53), G_MID = vec3(0.64, 0.76, 0.44), G_DEEP = vec3(0.52, 0.67, 0.36);
const vec3 INK = vec3(0.24, 0.38, 0.18);
const vec3 BANK_SOIL = vec3(0.66, 0.61, 0.40);
// Bảng màu nước hồ: xanh lam nhạt trong trẻo -> xanh ngọc lục bảo sâu
const vec3 LAKE0 = vec3(0.08, 0.76, 0.76), LAKE1 = vec3(0.06, 0.66, 0.66), LAKE2 = vec3(0.07, 0.56, 0.55);
// Bóng mây phản chiếu trên mặt hồ
const vec3 REFL_HI = vec3(0.95, 0.99, 0.99), REFL_BODY = vec3(0.78, 0.92, 0.92), REFL_SH = vec3(0.62, 0.83, 0.84);
const vec2 LIGHT = vec2(-0.6, -0.8);
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
float segD(vec2 p, vec2 a, vec2 b) { vec2 pa = p - a, ba = b - a; return length(pa - ba * clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0)); }

float grassInk(vec2 p) {
  const vec2 G = vec2(12.0, 6.0);
  vec2 c0 = floor(p / G);
  float ink = 0.0;
  for (int j = -1; j <= 1; j++) {
    vec2 c = c0 + vec2(0.0, float(j));
    vec2 cp = (c + 0.5) * G;
    float h = hash2(c + 3.1);
    float dens = 0.75 * ss(0.42, 0.72, N(cp, 900.0).r) * ss(0.35, 0.6, N(cp + 50.0, 380.0).g) + 0.02;
    if (h > dens) continue;
    float ang = (N(cp, 1400.0).g - 0.5) * 0.7 + (hash2(c + 7.7) - 0.5) * 0.3;
    float len = 2.5 + 3.0 * hash2(c + 1.9);
    vec2 ctr = cp + (vec2(hash2(c + 4.3), hash2(c + 5.9)) - 0.5) * vec2(2.0, 3.0);
    vec2 dir = vec2(cos(ang), sin(ang)) * len;
    float d = segD(p, ctr - dir, ctr + dir);
    ink = max(ink, (1.0 - ss(0.2, 0.9, d)) * (0.35 + 0.45 * hash2(c + 9.1)));
  }
  return ink;
}

// Trường khoảng cách mây trôi phản chiếu dưới nước hồ
float lakeCloudDist(vec2 q, bool front, inout vec4 fc) {
  float d = 1e9;
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

// Đá cuội dưới đáy nước nông ven bờ (hai lớp lưới so le)
vec4 underwaterStones(vec2 p) {
  float bestD = 1e9;
  vec3 stoneCol = vec3(0.0);
  for (int L = 0; L < 2; L++) {
    float G = L == 0 ? 28.0 : 19.0;
    vec2 lp = L == 0 ? p : rot(0.55) * p + 73.0;
    vec2 q = lp / G;
    vec2 c0 = floor(q);
    vec2 fq = fract(q);
    int sx = fq.x < 0.5 ? -1 : 1;
    int sy = fq.y < 0.5 ? -1 : 1;
    for (int dy = 0; dy <= 1; dy++) for (int dx = 0; dx <= 1; dx++) {
      vec2 c = c0 + vec2(float(dx * sx), float(dy * sy));
      float h = hash2(c + float(L) * 31.7 + uSeed * 0.05);
      if (h > (L == 0 ? 0.55 : 0.48)) continue;
      vec2 ctr = (c + 0.2 + 0.6 * vec2(hash2(c + 1.3), hash2(c + 2.9))) * G;
      vec2 dp = lp - ctr;
      float ang = hash2(c + 4.7) * 6.283;
      vec2 rdp = rot(ang) * dp;
      vec2 rad = G * vec2(0.26 + 0.18 * hash2(c + 6.1), 0.20 + 0.14 * hash2(c + 8.3));
      float d = (length(rdp / rad) - 1.0) * min(rad.x, rad.y);
      if (d < bestD) {
        bestD = d;
        float tone = hash2(c + 9.7);
        stoneCol = tone < 0.35 ? vec3(0.28, 0.62, 0.55)
                 : (tone < 0.70 ? vec3(0.42, 0.66, 0.48) : vec3(0.52, 0.64, 0.46));
        stoneCol *= 0.92 + 0.16 * ss(rad.y, -rad.y, dp.x * 0.6 + dp.y * 0.8);
      }
    }
  }
  return vec4(stoneCol, 1.0 - ss(-0.8, 1.2, bestD));
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y * uScale - gl_FragCoord.y) / uScale;
  float t = uTime * uFlow;
  vec2 gp = floor(p * (0.8 * max(uScale, 1.0)));
  float g1 = hash2(gp + 0.5), g2 = hash2(gp + 7.3);

  float s0 = mix(fieldS(uFA, p), fieldS(uFB, p), uMix);
  float s = s0 + (N(p, 650.0).r - 0.5) * 9.0;

  vec3 col;
  if (s <= 0.0) {
    // ---------- Bờ cỏ màu nước ----------
    float w1 = N(p, 1900.0).b, w2 = N(p + 311.0, 650.0).g;
    col = mix(G_MID, G_LIGHT, ss(0.3, 0.75, w1 * 0.7 + w2 * 0.3));
    col = mix(col, G_DEEP, ss(0.45, 0.85, N(p + 977.0, 1300.0).r) * 0.4);
    col *= 1.0 + 0.05 * (0.5 - (p.x + p.y) / (uRes.x + uRes.y));
    col = mix(col, INK, grassInk(p) * 0.72 * ss(-2.0, -14.0, s));
    // Dải đất ven bờ chuyển tiếp
    float bank = ss(-9.0, 0.0, s);
    col = mix(col, BANK_SOIL, bank * 0.78);
  } else {
    // ---------- Nước hồ ----------
    float dNorm = clamp(s / max(uDepth, 60.0), 0.0, 1.0);
    col = dNorm < 0.5 ? mix(LAKE0, LAKE1, dNorm * 2.0) : mix(LAKE1, LAKE2, (dNorm - 0.5) * 2.0);
    col += (g1 - 0.5) * 0.025;

    // Đáy nước nông ven bờ và đá cuội xanh ngọc dưới nước
    float shK = 1.0 - ss(uShoal * 0.25, uShoal * 1.15, s);
    if (shK > 0.01) {
      vec3 shallowBed = mix(BANK_SOIL, vec3(0.24, 0.68, 0.62), ss(0.0, 26.0, s));
      col = mix(col, shallowBed, (1.0 - ss(0.0, 22.0, s)) * 0.65);
      vec4 st = underwaterStones(p + vec2(sin(p.y * 0.12 + t * 1.6), cos(p.x * 0.12 + t * 1.3)) * 1.1);
      col = mix(col, st.rgb, st.a * shK * ss(2.0, 12.0, s) * 0.68);
    }

    // Bóng mây phản chiếu rõ nét trên mặt hồ (kèm gợn sóng nhẹ)
    vec2 rp = p + vec2(sin(p.y * 0.22 + t * 2.1) * 1.8, cos(p.x * 0.16 + t * 1.7) * 1.2);
    vec4 fc = vec4(0.0, -1e9, 0.0, 0.0);
    float cd = lakeCloudDist(rp, true, fc);
    if (cd < 2.0) {
      vec4 fc2 = vec4(0.0, -1e9, 0.0, 0.0);
      float hi = cd > -20.0 ? ss(-1.0, 1.0, lakeCloudDist(rp + LIGHT * 18.0, false, fc2)) : 0.0;
      vec3 rc = fc.y > -1e8 ? mix(REFL_BODY, REFL_SH, fc.w * 0.65) : mix(REFL_BODY, REFL_SH, 0.45);
      rc = mix(rc, REFL_HI, hi);
      float shoreFade = ss(3.0, 18.0, s);
      col = mix(col, rc, ss(1.2, -1.2, cd) * shoreFade * 0.82);
    }

    // Gợn sóng nhỏ dọc bờ
    float rip = sin(s * 0.42 - t * 2.4 + N(p, 260.0).r * 5.0);
    float ripLine = ss(0.88, 0.98, rip) * (1.0 - ss(8.0, 68.0, s)) * ss(1.0, 5.0, s);
    col = mix(col, vec3(0.82, 0.96, 0.95), ripLine * 0.38);
  }

  col *= 1.0 + (g2 - 0.5) * 0.032;
  if (uDebug > 0.5) col = mix(col, vec3(0.9, 0.15, 0.15), 1.0 - ss(0.0, 1.5, abs(s0)));
  o = vec4(col, 1.0);
}
