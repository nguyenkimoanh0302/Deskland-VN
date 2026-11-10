#version 300 es
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uDebug, uEdge;
uniform float uPhase;        // 1.0 = ban ngay, 2.0 = hoang hon, 3.0 = ban dem
uniform float uMoon, uStars;
uniform vec4 uPlane;         // x, y, huong(1 hoac -1), chieu dai vet khoi(>0 la co may bay)
uniform highp sampler2D uCirc;
uniform vec2 uCGrid;
uniform float uCellG;
uniform float uBirdN;
uniform vec4 uBird[8];
out vec4 o;

const int CIRC_K = 36;
const vec2 LIGHT = vec2(-0.6, -0.8);

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

// Máy bay nhỏ (x hướng về phía trước)
float planeSD(vec2 q) {
  float fus = (length(q / vec2(11.0, 2.0)) - 1.0) * 2.0;
  vec2 wq = vec2(q.x + abs(q.y) * 0.45 - 1.0, abs(q.y));
  float wing = max(abs(wq.x) - 2.3, wq.y - 9.5);
  vec2 tq = vec2(q.x + abs(q.y) * 0.35 + 8.5, abs(q.y));
  float tail = max(abs(tq.x) - 1.5, tq.y - 4.0);
  return min(fus, min(wing, tail));
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y * uScale - gl_FragCoord.y) / uScale;
  float vy = clamp(p.y / max(uRes.y, 1.0), 0.0, 1.0);

  // Bảng màu bầu trời và mây theo ba khung giờ
  float isDusk = clamp(1.0 - abs(uPhase - 2.0), 0.0, 1.0);
  float isNight = clamp(uPhase - 2.0, 0.0, 1.0);
  float isDay = clamp(2.0 - uPhase, 0.0, 1.0);

  vec3 daySky = mix(vec3(0.20, 0.52, 0.80), vec3(0.52, 0.76, 0.92), pow(vy, 0.85));
  vec3 duskSky = mix(vec3(0.24, 0.27, 0.50), mix(vec3(0.86, 0.48, 0.42), vec3(0.97, 0.76, 0.48), ss(0.35, 1.0, vy)), ss(0.0, 0.75, vy));
  vec3 nightSky = mix(vec3(0.05, 0.09, 0.20), vec3(0.12, 0.21, 0.38), pow(vy, 0.9));
  vec3 col = daySky * isDay + duskSky * isDusk + nightSky * isNight;

  // Bầu trời sao ban đêm và tia sáng chữ thập
  if (isNight > 0.01 && uStars > 0.01) {
    for (int L = 0; L < 2; L++) {
      float cs = L == 0 ? 52.0 : 84.0;
      vec2 q = p / cs + float(L) * 19.7;
      vec2 cell = floor(q), f = fract(q);
      vec2 sc = cell + float(L) * 43.1 + uSeed * 0.1;
      float h = hash2(sc);
      if (h > (L == 0 ? 0.24 : 0.16) * (0.4 + 0.8 * uStars)) continue;
      vec2 ctr = 0.18 + 0.64 * vec2(hash2(sc + 1.7), hash2(sc + 3.1));
      vec2 dp = (f - ctr) * cs;
      float tw = 0.55 + 0.45 * sin(uTime * (1.2 + 1.8 * hash2(sc + 5.3)) + h * 25.0);
      float sz = (0.7 + 1.3 * uStars) * (L == 1 ? 1.35 : 0.9);
      float dotStar = (1.0 - ss(0.3 * sz, 1.5 * sz, length(dp))) * tw;
      // Tia sáng chữ thập (ngôi sao lớn)
      float cross = 0.0;
      if (L == 1 && uStars > 0.35) {
        float cx = (1.0 - ss(0.0, 0.9, abs(dp.y))) * (1.0 - ss(0.0, 5.5 * uStars, abs(dp.x)));
        float cy = (1.0 - ss(0.0, 0.9, abs(dp.x))) * (1.0 - ss(0.0, 5.5 * uStars, abs(dp.y)));
        cross = max(cx, cy) * tw * 0.75;
      }
      col = mix(col, vec3(0.98, 0.99, 1.0), max(dotStar, cross) * isNight);
    }
  }

  // Trăng lưỡi liềm và quầng trăng ban đêm
  if (isNight > 0.01 && uMoon > 0.5) {
    vec2 mp = vec2(uRes.x * 0.82, uRes.y * 0.18);
    float md1 = length(p - mp) - 34.0;
    float md2 = length(p - (mp + vec2(-13.0, -8.0))) - 30.0;
    float crescent = max(md1, -md2);
    float glow = exp(-max(md1, 0.0) / 55.0) * 0.25;
    col += vec3(0.55, 0.68, 0.95) * glow * isNight;
    col = mix(col, vec3(1.0, 0.97, 0.84), ss(1.0, -1.0, crescent) * isNight);
  }

  // Máy bay và vệt khói trắng
  if (uPlane.w > 0.5) {
    vec2 pp = uPlane.xy;
    float dir = uPlane.z;
    vec2 qp = vec2((p.x - pp.x) * dir, p.y - pp.y);
    if (qp.x < -10.0 && qp.x > -uPlane.w && abs(qp.y) < 12.0) {
      float tTrail = (-qp.x - 10.0) / uPlane.w;
      float wTrail = 1.4 + tTrail * 3.8 + (N(p, 90.0).r - 0.5) * 1.8;
      float aTrail = (1.0 - ss(0.4, wTrail, abs(qp.y))) * (1.0 - tTrail) * 0.55;
      vec3 trailC = isDusk > 0.5 ? vec3(1.0, 0.86, 0.76) : (isNight > 0.5 ? vec3(0.65, 0.75, 0.92) : vec3(0.98, 0.99, 1.0));
      col = mix(col, trailC, aTrail);
    }
    if (dot(qp, qp) < 250.0) {
      float pd = planeSD(qp);
      vec3 pc = isNight > 0.5 ? vec3(0.82, 0.88, 0.96) : vec3(0.99, 0.99, 1.0);
      col = mix(col, pc, ss(0.6, -0.6, pd));
    }
  }

  // ---------- Cụm mây (tự động đổi sắc độ theo khung giờ) ----------
  vec3 cHi = vec3(0.99, 1.0, 1.0) * isDay + vec3(1.0, 0.91, 0.78) * isDusk + vec3(0.76, 0.84, 0.96) * isNight;
  vec3 cBody = vec3(0.95, 0.98, 1.0) * isDay + vec3(0.96, 0.76, 0.68) * isDusk + vec3(0.32, 0.42, 0.62) * isNight;
  vec3 cSh = vec3(0.76, 0.87, 0.96) * isDay + vec3(0.72, 0.50, 0.58) * isDusk + vec3(0.20, 0.28, 0.46) * isNight;

  vec4 fc = vec4(0.0, -1e9, 0.0, 0.0);
  float d = cloudDist(p, true, fc);
  if (d < 1.5) {
    float hi = d > -22.0 ? ss(-1.0, 1.0, cloudDist(p + LIGHT * 20.0)) : 0.0;
    vec3 cc = fc.y > -1e8 ? mix(cBody, cSh, fc.w * 0.72) : mix(cBody, cSh, 0.48);
    cc = mix(cc, cHi, hi);
    col = mix(col, cc, ss(1.0, -1.0, d));
  }

  // ---------- Đàn chim bay (xuất hiện ban ngày và hoàng hôn) ----------
  if (isNight < 0.5) {
    for (int i = 0; i < 8; i++) {
      if (float(i) >= uBirdN) break;
      vec4 b = uBird[i];
      vec2 d0 = p - b.xy;
      if (dot(d0, d0) < 400.0) {
        vec3 g = gullSD(rot(-b.z) * d0, b.w);
        vec3 wingC = isDusk > 0.5 ? vec3(0.32, 0.24, 0.34) : mix(vec3(0.84, 0.88, 0.92), vec3(0.24, 0.28, 0.34), ss(0.72, 0.8, g.z));
        vec3 bodyC = isDusk > 0.5 ? vec3(0.28, 0.20, 0.30) : vec3(0.99, 0.99, 1.0);
        col = mix(col, wingC, ss(0.6, -0.6, g.y));
        col = mix(col, bodyC, ss(0.6, -0.6, g.x));
      }
    }
  }

  col *= 1.0 + (hash2(floor(p * (0.8 * max(uScale, 1.0)))) - 0.5) * 0.028;
  if (uDebug > 0.5) col = mix(col, vec3(0.9, 0.15, 0.15), 1.0 - ss(0.0, 1.5, abs(d)));
  o = vec4(col, 1.0);
}
