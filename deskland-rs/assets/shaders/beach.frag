#version 300 es
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uSpeed, uRunup, uShallow, uShoal, uGrain, uDebug, uWave, uSigma, uSpacing, uWaves;
uniform float uKayN;
uniform vec4 uKay[4];          // x, y, hướng, pha chèo
uniform vec4 uRip[6];
uniform float uBirdN;
uniform vec4 uBird[8];         // x, y, hướng, độ sải cánh
out vec4 o;

// ---- Bảng màu (sRGB) ----
// Biển: gần bờ -> xa bờ
const vec3 SEA0 = vec3(0.74, 0.93, 0.87), SEA1 = vec3(0.47, 0.87, 0.84), SEA2 = vec3(0.17, 0.76, 0.81);
const vec3 SEA3 = vec3(0.03, 0.66, 0.79), SEA4 = vec3(0.00, 0.56, 0.73);
// Cát: khô -> ướt
const vec3 SAND0 = vec3(0.97, 0.93, 0.88), SAND1 = vec3(0.91, 0.85, 0.84), SAND2 = vec3(0.82, 0.76, 0.79), SAND3 = vec3(0.71, 0.67, 0.75);
const vec3 FOAM = vec3(0.98, 0.99, 0.98);
// Cát ướt gần bờ
const vec3 WETSAND = vec3(0.93, 0.86, 0.72);

float hash(vec2 p) { p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
float hash2(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}
float ss(float a, float b, float x) { float t = clamp((x - a) / (b - a), 0.0, 1.0); return t * t * (3.0 - 2.0 * t); }
vec4 N(vec2 p, float scale) { return texture(uNoise, p / scale + uSeed * 0.013); }
float fieldS(sampler2D t, vec2 p) { return texture(t, (p / uStep + 0.5) / uGrid).r; }

vec3 seaRamp(float v) {
  v = clamp(v, 0.0, 1.0) * 4.0;
  if (v < 1.0) return mix(SEA0, SEA1, v);
  if (v < 2.0) return mix(SEA1, SEA2, v - 1.0);
  if (v < 3.0) return mix(SEA2, SEA3, v - 2.0);
  return mix(SEA3, SEA4, v - 3.0);
}
vec3 sandRamp(float v) {
  v = clamp(v, 0.0, 1.0) * 3.0;
  if (v < 1.0) return mix(SAND0, SAND1, v);
  if (v < 2.0) return mix(SAND1, SAND2, v - 1.0);
  return mix(SAND2, SAND3, v - 2.0);
}
mat2 rot(float a) { float c = cos(a), s = sin(a); return mat2(c, s, -s, c); }
// Nhiễu tế bào Worley: x = F1, y = F2
vec2 worley(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  float d1 = 8.0, d2 = 8.0;
  for (int y = -1; y <= 1; y++) for (int x = -1; x <= 1; x++) {
    vec2 g = vec2(float(x), float(y)), c = i + g;
    vec2 r = g + vec2(hash(c + uSeed), hash(c + uSeed + 17.3)) * 0.8 + 0.1 - f;
    float d = dot(r, r);
    if (d < d1) { d2 = d1; d1 = d; } else if (d < d2) d2 = d;
  }
  return vec2(sqrt(d1), sqrt(d2));
}
// Thân thuyền kayak
float hullSD(vec2 q) {
  float L = 22.0, Wd = 5.4;
  float t = clamp(q.x / L, -1.0, 1.0);
  float w = Wd * (1.0 - t * t);
  return max(abs(q.y) - w, abs(q.x) - L);
}
float segSD(vec2 p, vec2 a, vec2 b) { vec2 pa = p - a, ba = b - a; float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0); return length(pa - ba * h); }
// Mái chèo
vec2 paddleSD(vec2 q, float ph) {
  float a = 1.5708 + sin(ph) * 0.55;
  vec2 dir = vec2(cos(a), sin(a)) , c = vec2(0.5, 0.0);
  float shaft = segSD(q, c - dir * 13.0, c + dir * 13.0) - 0.7;
  vec2 b1 = q - (c + dir * 13.5), b2 = q - (c - dir * 13.5);
  mat2 r = rot(-a);
  vec2 l1 = r * b1, l2 = r * b2;
  float blade = min(length(l1 / vec2(3.6, 1.6)) - 1.0, length(l2 / vec2(3.6, 1.6)) - 1.0) * 1.6;
  return vec2(shaft, blade);
}
// Hải âu (nhìn từ trên xuống)
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
  vec2 gp = floor(p * (0.78 * max(uScale, 1.0)));
  float g1 = hash2(gp + 0.5), g2 = hash2(gp + 7.3);
  float gAmt = uGrain;

  float s0 = mix(fieldS(uFA, p), fieldS(uFB, p), uMix);
  float s = s0 + (N(p, 700.0).r - 0.5) * 10.0;

  float wv = uWave;
  float lam = uSpacing;
  float along = N(p, 4200.0).b * 1.2;
  float u = t * uSpeed / lam + along;
  float fu = fract(u), cyc = floor(u);

  vec2 gr = vec2(dFdx(s0), -dFdy(s0));
  vec2 nrm = length(gr) > 1e-4 ? normalize(gr) : vec2(0.0, 1.0);
  float sIn = mix(fieldS(uFA, p - nrm * 45.0), fieldS(uFB, p - nrm * 45.0), uMix);
  float sOut = mix(fieldS(uFA, p + nrm * 70.0), fieldS(uFB, p + nrm * 70.0), uMix);
  float energy = mix(ss(2.0, -18.0, sIn), 1.0, ss(15.0, 40.0, s)) * ss(25.0, 70.0, sOut);
  float shoreE = 0.0, sW = s;
  if (s < 4.0 && s > -uSigma * 0.78 - 12.0) {
    #define SF(q) mix(fieldS(uFA, q), fieldS(uFB, q), uMix)
    float fR = SF(p + vec2(10.0, 0.0)), fL = SF(p - vec2(10.0, 0.0)), fD = SF(p + vec2(0.0, 10.0)), fU = SF(p - vec2(0.0, 10.0));
    vec2 g10 = vec2(fR - fL, fD - fU);
    vec2 n10 = length(g10) > 1e-3 ? normalize(g10) : nrm;
    vec2 sp = p - n10 * s0;
    float eIn = SF(sp - n10 * 45.0), eOut = SF(sp + n10 * 70.0);
    shoreE = ss(2.0, -18.0, eIn) * ss(25.0, 70.0, eOut);
    sW = mix(s, (fR + fL + fD + fU) * 0.25 + (s - s0), ss(-2.0, -16.0, s));
  }

  vec3 col;
  if (s > 2.0) {
    // ---------- Biển ----------
    float depth = pow(clamp(max(s - uShoal * 0.45, 0.0) / (uShallow * 1.3), 0.0, 1.0), 0.62);
    col = seaRamp(depth + (g1 - 0.5) * 0.09 * gAmt);

    vec2 drift = vec2(t * 2.2, t * 0.9);
    float deepK = ss(uShallow * 0.35, uShallow * 1.1, s);
    col *= 1.0 + (N(p + drift * 4.0, 1600.0).r - 0.5) * 0.07 * ss(0.0, uShoal + 40.0, s);
    float glitter = ss(0.32, 0.72, N(p + drift * 3.0, 520.0).g) * deepK;
    float spark = 0.0;
    for (int L = 0; L < 2; L++) {
      float fl = float(L), cs = L == 0 ? 23.0 : 37.0;
      mat2 R = rot(L == 0 ? 0.62 : -1.13);
      vec2 q = R * p / cs + fl * 17.3;
      vec2 cell = floor(q), f = fract(q);
      vec2 seedc = cell + fl * 51.7 + uSeed * 0.1;
      if (hash2(seedc) > (L == 0 ? 0.11 : 0.16) * glitter) continue;
      vec2 c = 0.15 + 0.7 * vec2(hash2(seedc + 1.3), hash2(seedc + 2.9));
      float life = fract(t * (0.3 + 0.35 * hash2(seedc + 8.1)) + hash2(seedc + 5.7));
      float tw = ss(0.0, 0.1, life) * (1.0 - ss(0.2, 0.45, life));
      float r = 0.7 + 0.9 * hash2(seedc + 3.3);
      spark = max(spark, (1.0 - ss(r * 0.5, r * 1.5, length((f - c) * cs))) * tw);
    }
    col = mix(col, vec3(0.95, 1.0, 1.0), spark * 0.85);

    // Bãi nông trong suốt
    float shoal = uShoal;
    float k = clamp(ss(shoal * 0.15, shoal * 1.25, s) + (g2 - 0.5) * 0.16 * gAmt, 0.0, 1.0);
    if (k < 1.0) {
      vec3 bed = mix(vec3(0.97, 0.91, 0.79), vec3(0.88, 0.90, 0.77), ss(0.0, shoal, s));
      vec3 tint = mix(vec3(0.93, 0.99, 0.96), vec3(0.62, 0.92, 0.88), ss(0.0, shoal * 0.9, s));
      col = mix(bed * tint, col, k);
    }
    col = mix(col, mix(sandRamp((g1 - 0.5) * 0.1 * gAmt), WETSAND, 0.6), 1.0 - ss(2.0, 38.0, s));

    // Bóng thuyền kayak
    float shadow = 0.0;
    for (int i = 0; i < 4; i++) {
      if (float(i) >= uKayN) break;
      vec4 k = uKay[i];
      vec2 q2 = rot(-k.z) * (p - k.xy - vec2(8.0, 13.0));
      float sd = hullSD(q2);
      vec2 pd = paddleSD(q2, k.w);
      shadow = max(shadow, ss(1.6, -1.6, min(sd, min(pd.x, pd.y))));
    }
    col = mix(col, col * vec3(0.62, 0.78, 0.84), shadow * 0.7);

    // Gợn sóng
    float rip = 0.0;
    for (int i = 0; i < 6; i++) {
      vec4 r = uRip[i];
      if (r.w <= 0.0) continue;
      float dist = length(p - r.xy), rad = r.z * 80.0;
      rip = max(rip, (1.0 - ss(0.8, 2.0, abs(dist - rad))) * exp(-r.z * 1.4) * r.w);
      rip = max(rip, (1.0 - ss(0.6, 1.6, abs(dist - rad * 0.6))) * exp(-r.z * 1.8) * r.w * 0.6);
    }
    col = mix(col, FOAM, step(g2, rip * 0.95));

    // ---------- Sóng biển ----------
    float dd = s - 2.0;
    float ph = (s + t * uSpeed) / lam + along;
    float fr = fract(ph);
    float dpx = min(fr, 1.0 - fr) * lam;
    float vis = (1.0 - ss(lam * 0.45, lam * 1.35, s)) * ss(0.0, 4.0, dd) * energy;
    float brk = 1.0 - ss(8.0, 55.0, s);
    float cw = wv * (1.0 + 0.8 * brk);
    float crest = (1.0 - ss(0.7 * cw, 1.9 * cw, dpx)) * vis * (0.55 + 0.4 * brk);
    float crestSpray = exp(-fr * lam / (7.0 * cw)) * vis * (0.25 + 0.4 * brk);
    float burst = exp(-fu * 4.5) * ss(0.0, 0.04, fu);
    float spray = exp(-dd / (6.0 * wv)) * burst * energy * 0.45;
    float foam = max(crest, step(g2, max(spray, crestSpray)));
    col = mix(col, FOAM, foam);
  } else {
    // ---------- Bãi cát ----------
    float wet = ss(-7.0 - 3.0 * shoreE, 1.0, s) * 0.2;
    col = mix(sandRamp((g1 - 0.5) * 0.1 * gAmt), WETSAND, wet * 3.0);
    col *= 1.0 - 0.03 * ss(0.4, 0.8, N(p, 900.0).r) * (1.0 - wet);
  }

  // ---------- Lớp nước mỏng tràn lên bãi cát ----------
  float reachBase = min(uRunup * 0.25 * shoreE * (0.85 + 0.3 * N(p, 900.0).g), uSigma * 0.78);
  float wk = (1.0 - ss(2.0, 38.0, s)) * mix(1.0, energy, ss(2.0, 10.0, s));
  if ((s < 4.0 && reachBase > 0.5) || (s >= 4.0 && wk > 0.01)) {
    const vec3 FILM = WETSAND, MARK = WETSAND * 0.97;
    float onSand = step(s, 4.0);
    vec2 wl = worley((p - nrm * fu * 8.0) / 7.0);
    float laceNet = (1.0 - ss(0.03, 0.1, wl.y - wl.x)) * ss(0.4, 0.62, N(p + vec2(t * 2.0, 0.0), 120.0).g);
    float wetMem = 0.0, filmA = 0.0;
    vec4 layers[3];
    for (int j = 0; j < 3; j++) {
      float a = fu + float(j);
      float amp = reachBase * (0.5 + 0.5 * hash(vec2(cyc - float(j), 3.7)));
      float front = a < 0.45 ? mix(3.0, -amp, 1.0 - pow(1.0 - a / 0.45, 2.2))
                             : mix(-amp, 3.0, ss(0.45, 1.4, a));
      float reached = ss(-amp - 4.0, -amp + 12.0, sW);
      wetMem = max(wetMem, reached * (1.0 - ss(0.45, 2.8, a)) * ss(0.2, 0.45, a));
      float alpha = a < 0.45 ? 0.42 : 0.42 * (1.0 - ss(0.45, 1.4, a));
      float feather = 1.0 + 16.0 * ss(0.45, 1.1, a);
      filmA = max(filmA, ss(front - 1.0, front + feather, sW) * alpha * 1.6);
      layers[j].w = feather;
      layers[j].xyz = vec3(front, a, amp);
    }
    col = mix(col, MARK, wetMem * 0.18 * wk);
    col = mix(col, mix(col * vec3(0.98, 0.96, 0.90), FILM, 0.6), filmA * wk);
    for (int j = 2; j >= 0; j--) {
      float front = layers[j].x, a = layers[j].y, amp = layers[j].z;
      float cov = ss(front - 1.0, front + layers[j].w, sW);
      float lace = laceNet * cov * ss(0.4, 0.6, a) * (1.0 - ss(0.7, 1.4, a)) * wk;
      col = mix(col, FOAM, lace * 0.75);
      float edgeK = a < 0.45 ? 1.0 : 1.0 - ss(0.45, 1.05, a);
      float d = sW - front;
      float lineE = (1.0 - ss(0.8 * wv, 1.9 * wv, abs(d - 0.9 * wv))) * edgeK * onSand;
      float sprayE = exp(-max(d, 0.0) / (5.0 * wv)) * cov * edgeK * 0.5 * onSand;
      col = mix(col, FOAM, max(lineE * 0.95, step(g1, sprayE)));
    }
  }

  // ---------- Thuyền kayak ----------
  for (int i = 0; i < 4; i++) {
    if (float(i) >= uKayN) break;
    vec4 k = uKay[i];
    vec2 q = rot(-k.z) * (p - k.xy);
    if (dot(q, q) > 900.0) continue;
    float sd = hullSD(q);
    vec2 pd = paddleSD(q, k.w);
    vec3 kc = col;
    float a = 0.0;
    float mh = ss(0.7, -0.7, sd);
    if (mh > 0.0) {
      float tt = clamp(q.x / 22.0, -1.0, 1.0);
      float wd = 5.4 * (1.0 - tt * tt);
      vec3 hull = mix(vec3(0.99, 0.84, 0.16), vec3(1.0, 0.93, 0.45), 1.0 - ss(0.2, 0.7, abs(q.y) / max(wd, 0.5)));
      hull = mix(hull, vec3(0.90, 0.64, 0.10), ss(0.78, 1.0, abs(q.y) / max(wd, 0.5)));
      hull = mix(hull, vec3(0.97, 0.96, 0.9), 1.0 - ss(1.0, 1.6, length(q - vec2(13.0, 0.0))));
      hull = mix(hull, vec3(0.97, 0.96, 0.9), 1.0 - ss(1.0, 1.6, length(q - vec2(-12.0, 0.0))));
      hull = mix(hull, vec3(0.30, 0.24, 0.20), 1.0 - ss(0.0, 0.6, length((q - vec2(-1.0, 0.0)) / vec2(6.0, 2.8)) - 1.0));
      kc = mix(kc, hull, mh); a = max(a, mh);
    }
    float body = ss(0.5, -0.5, length(q - vec2(-1.5, 0.0)) - 3.3);
    kc = mix(kc, vec3(0.96, 0.55, 0.18), body);
    float head = ss(0.5, -0.5, length(q - vec2(0.2, 0.0)) - 2.0);
    kc = mix(kc, vec3(0.22, 0.17, 0.16), head);
    float sh = ss(0.5, -0.5, pd.x), bl = ss(0.5, -0.5, pd.y);
    kc = mix(kc, vec3(0.28, 0.24, 0.26), sh);
    kc = mix(kc, vec3(0.90, 0.28, 0.22), bl);
    col = kc;
  }

  // ---------- Chim biển ----------
  for (int i = 0; i < 8; i++) {
    if (float(i) >= uBirdN) break;
    vec4 b = uBird[i];
    vec2 d0 = p - b.xy, d1 = d0 - vec2(34.0, 55.0);
    if (dot(d1, d1) < 400.0) {
      vec3 g = gullSD(rot(-b.z) * d1, b.w);
      col = mix(col, col * vec3(0.70, 0.74, 0.84), ss(1.8, -1.8, min(g.x, g.y)) * 0.28);
    }
    if (dot(d0, d0) < 400.0) {
      vec3 g = gullSD(rot(-b.z) * d0, b.w);
      vec3 wingC = mix(vec3(0.80, 0.84, 0.88), vec3(0.20, 0.22, 0.26), ss(0.72, 0.8, g.z));
      col = mix(col, wingC, ss(0.6, -0.6, g.y));
      col = mix(col, vec3(0.99, 0.99, 0.98), ss(0.6, -0.6, g.x));
      col = mix(col, vec3(0.98, 0.78, 0.22), ss(0.5, -0.5, length(rot(-b.z) * d0 - vec2(7.2, 0.0)) - 0.9));
    }
  }

  if (uDebug > 0.5) col = mix(col, vec3(0.9, 0.15, 0.15), 1.0 - ss(0.0, 1.5, abs(s0)));
  col *= 1.0 + (g2 - 0.5) * 0.035 * gAmt;
  o = vec4(col, 1.0);
}
