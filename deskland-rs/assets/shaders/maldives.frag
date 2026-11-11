#version 300 es
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uSpeed, uRunup, uShallow, uShoal, uGrain, uDebug, uWave, uSigma, uSpacing, uWaves;
uniform float uKayN;
uniform vec4 uKay[4];
uniform float uFishN;
uniform vec4 uFish[6];         // x, y, hướng, pha
uniform float uTurtN;
uniform vec4 uTurt[4];         // x, y, hướng, pha bơi
uniform vec4 uRip[6];
uniform vec4 uSand[4];         // x, y, age, rot+1
uniform vec4 uBait;            // x, y, age, alpha
uniform float uBirdN;
uniform vec4 uBird[8];
out vec4 o;

// Bảng màu Maldives: xanh bạc hà trong vắt -> xanh ngọc bích; bãi cát trắng kem
const vec3 SEA0 = vec3(0.76, 0.96, 0.91), SEA1 = vec3(0.39, 0.91, 0.86), SEA2 = vec3(0.15, 0.85, 0.82);
const vec3 SEA3 = vec3(0.10, 0.77, 0.76), SEA4 = vec3(0.08, 0.69, 0.71);
const vec3 REEF_DARK = vec3(0.06, 0.44, 0.49), REEF_WARM = vec3(0.24, 0.56, 0.52);
const vec3 SAND0 = vec3(0.99, 0.96, 0.89), SAND1 = vec3(0.95, 0.90, 0.84), SAND2 = vec3(0.88, 0.83, 0.78), SAND3 = vec3(0.79, 0.74, 0.72);
const vec3 FOAM = vec3(0.99, 1.00, 0.99);
const vec3 WETSAND = vec3(0.94, 0.88, 0.75);

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
float hullSD(vec2 q) {
  float L = 22.0, Wd = 5.4;
  float t = clamp(q.x / L, -1.0, 1.0);
  float w = Wd * (1.0 - t * t);
  return max(abs(q.y) - w, abs(q.x) - L);
}
float segSD(vec2 p, vec2 a, vec2 b) { vec2 pa = p - a, ba = b - a; float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0); return length(pa - ba * h); }
vec2 paddleSD(vec2 q, float ph) {
  float a = 1.5708 + sin(ph) * 0.55;
  vec2 dir = vec2(cos(a), sin(a)), c = vec2(0.5, 0.0);
  float shaft = segSD(q, c - dir * 13.0, c + dir * 13.0) - 0.7;
  vec2 b1 = q - (c + dir * 13.5), b2 = q - (c - dir * 13.5);
  mat2 r = rot(-a);
  vec2 l1 = r * b1, l2 = r * b2;
  float blade = min(length(l1 / vec2(3.6, 1.6)) - 1.0, length(l2 / vec2(3.6, 1.6)) - 1.0) * 1.6;
  return vec2(shaft, blade);
}
// Rùa biển (nhìn từ trên xuống, x hướng về trước): trả về (khoảng cách mai, khoảng cách chân và đầu)
vec2 turtleSD(vec2 q, float ph) {
  float shell = (length(q / vec2(8.5, 6.8)) - 1.0) * 6.8;
  float head = length((q - vec2(10.0, 0.0)) / vec2(2.8, 2.0)) * 2.0 - 2.0;
  vec2 fq = vec2(q.x - 3.2, abs(q.y) - 5.2);
  float stroke = sin(ph) * 0.42;
  vec2 f1 = rot(-0.45 + stroke) * fq;
  float frontFlip = (length((f1 - vec2(0.0, 4.2)) / vec2(2.1, 5.8)) - 1.0) * 2.1;
  vec2 bq = vec2(q.x + 6.5, abs(q.y) - 3.6);
  vec2 b1 = rot(0.55 - stroke * 0.4) * bq;
  float backFlip = (length((b1 - vec2(0.0, 2.2)) / vec2(1.5, 3.0)) - 1.0) * 1.5;
  return vec2(shell, min(head, min(frontFlip, backFlip)));
}
// Cá nhỏ (nhìn từ trên xuống, x hướng về trước)
float fishSD(vec2 q, float wig) {
  q.y += sin(q.x * 0.65 - wig) * 0.55;
  float body = (length(q / vec2(3.8, 1.35)) - 1.0) * 1.35;
  vec2 tq = q - vec2(-3.8, 0.0);
  float tail = max(abs(tq.y) - (-tq.x) * 0.65, max(tq.x, -tq.x - 2.2));
  return min(body, tail);
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
  vec2 gp = floor(p * (0.78 * max(uScale, 1.0)));
  float g1 = hash2(gp + 0.5), g2 = hash2(gp + 7.3);
  float gAmt = uGrain;

  float s0 = mix(fieldS(uFA, p), fieldS(uFB, p), uMix);
  float s = s0 + (N(p, 700.0).r - 0.5) * 10.0;

  float wv = uWave, lam = uSpacing;
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
    float depth = pow(clamp(max(s - uShoal * 0.45, 0.0) / (uShallow * 1.3), 0.0, 1.0), 0.62);
    col = seaRamp(depth + (g1 - 0.5) * 0.08 * gAmt);

    // Rạn san hô tối màu dưới nước sâu
    float reefZone = ss(uShoal * 0.55, uShoal * 1.4, s);
    if (reefZone > 0.01) {
      vec4 rn1 = N(p + vec2(140.0, -90.0), 460.0), rn2 = N(p * 1.7 - 60.0, 160.0);
      float reefVal = rn1.r * 0.65 + rn2.g * 0.35 + (g1 - 0.5) * 0.08;
      float reefMask = ss(0.55, 0.68, reefVal) * reefZone;
      vec3 reefCol = mix(REEF_WARM, REEF_DARK, ss(0.58, 0.76, reefVal));
      col = mix(col, reefCol, reefMask * 0.52);
    }

    vec2 drift = vec2(t * 2.0, t * 0.8);
    float deepK = ss(uShallow * 0.30, uShallow * 1.05, s);
    col *= 1.0 + (N(p + drift * 4.0, 1500.0).r - 0.5) * 0.07 * ss(0.0, uShoal + 40.0, s);

    // Điểm lấp lánh ánh nắng
    float glitter = ss(0.32, 0.72, N(p + drift * 3.0, 520.0).g) * deepK;
    float spark = 0.0;
    for (int L = 0; L < 2; L++) {
      float fl = float(L), cs = L == 0 ? 23.0 : 37.0;
      mat2 R = rot(L == 0 ? 0.62 : -1.13);
      vec2 q = R * p / cs + fl * 17.3;
      vec2 cell = floor(q), f = fract(q);
      vec2 seedc = cell + fl * 51.7 + uSeed * 0.1;
      if (hash2(seedc) > (L == 0 ? 0.12 : 0.17) * glitter) continue;
      vec2 c = 0.15 + 0.7 * vec2(hash2(seedc + 1.3), hash2(seedc + 2.9));
      float life = fract(t * (0.3 + 0.35 * hash2(seedc + 8.1)) + hash2(seedc + 5.7));
      float tw = ss(0.0, 0.1, life) * (1.0 - ss(0.2, 0.45, life));
      float r = 0.7 + 0.9 * hash2(seedc + 3.3);
      spark = max(spark, (1.0 - ss(r * 0.5, r * 1.5, length((f - c) * cs))) * tw);
    }
    col = mix(col, vec3(0.96, 1.0, 1.0), spark * 0.85);

    // Bãi nông trong suốt
    float shoal = uShoal;
    float k = clamp(ss(shoal * 0.15, shoal * 1.25, s) + (g2 - 0.5) * 0.16 * gAmt, 0.0, 1.0);
    if (k < 1.0) {
      vec3 bed = mix(vec3(0.99, 0.95, 0.84), vec3(0.89, 0.93, 0.80), ss(0.0, shoal, s));
      vec3 tint = mix(vec3(0.94, 1.0, 0.97), vec3(0.58, 0.94, 0.90), ss(0.0, shoal * 0.9, s));
      col = mix(bed * tint, col, k);
    }
    col = mix(col, mix(sandRamp((g1 - 0.5) * 0.1 * gAmt), WETSAND, 0.6), 1.0 - ss(2.0, 38.0, s));

    // ---------- Đàn cá nhỏ bơi lội ----------
    for (int i = 0; i < 6; i++) {
      if (float(i) >= uFishN) break;
      vec4 fs = uFish[i];
      vec2 d0 = p - fs.xy;
      if (dot(d0, d0) > 3600.0) continue;
      mat2 rf = rot(-fs.z);
      for (int m = 0; m < 11; m++) {
        float fm = float(m);
        vec2 off = vec2((hash(vec2(fm, float(i) + 1.3)) - 0.5) * 52.0, (hash(vec2(fm, float(i) + 7.1)) - 0.5) * 32.0);
        off.x += sin(fs.w * 0.7 + fm * 1.9) * 4.5;
        off.y += cos(fs.w * 0.6 + fm * 2.3) * 3.0;
        vec2 fq = rf * d0 - off;
        if (dot(fq, fq) > 45.0) continue;
        float fsh = fishSD(fq - vec2(3.5, 5.5), fs.w + fm);
        col = mix(col, col * vec3(0.68, 0.82, 0.85), ss(0.9, -0.5, fsh) * 0.45);
        float fd = fishSD(fq, fs.w + fm);
        if (fd < 0.8) {
          vec3 fc = mod(fm, 3.0) < 0.5 ? vec3(1.0, 0.52, 0.24) : (mod(fm, 3.0) < 1.5 ? vec3(1.0, 0.88, 0.25) : vec3(0.95, 0.99, 1.0));
          col = mix(col, fc, ss(0.6, -0.6, fd) * 0.92);
        }
      }
    }

    // ---------- Rùa biển ----------
    for (int i = 0; i < 4; i++) {
      if (float(i) >= uTurtN) break;
      vec4 tr = uTurt[i];
      vec2 d0 = p - tr.xy;
      if (dot(d0, d0) > 900.0) continue;
      // Bóng dưới đáy biển
      vec2 qs = rot(-tr.z) * (d0 - vec2(6.5, 10.5));
      vec2 sds = turtleSD(qs, tr.w);
      col = mix(col, col * vec3(0.62, 0.78, 0.82), ss(1.5, -1.2, min(sds.x, sds.y)) * 0.58);
      // Thân rùa biển
      vec2 qt = rot(-tr.z) * d0;
      vec2 sdt = turtleSD(qt, tr.w);
      if (min(sdt.x, sdt.y) < 1.0) {
        vec3 limbC = vec3(0.36, 0.56, 0.34);
        col = mix(col, limbC, ss(0.6, -0.6, sdt.y));
        if (sdt.x < 0.8) {
          vec2 sc = worley(qt / 3.4);
          vec3 shellC = mix(vec3(0.24, 0.40, 0.24), vec3(0.48, 0.64, 0.36), ss(0.04, 0.18, sc.y - sc.x));
          shellC = mix(shellC, vec3(0.68, 0.78, 0.48), 1.0 - ss(-1.8, -0.2, sdt.x));
          col = mix(col, shellC, ss(0.6, -0.6, sdt.x));
        }
      }
    }

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

    // Mồi cá lấp lánh dưới làn nước xanh ngọc
    if (uBait.w > 0.01 && dot(p - uBait.xy, p - uBait.xy) < 1024.0) {
      for (int m = 0; m < 8; m++) {
        float fm = float(m);
        float a = fm * 0.7854 + uBait.z * 0.65;
        float r = (4.0 + 18.0 * hash(vec2(fm, 3.1))) * (0.65 + 0.35 * sin(uBait.z * 2.2 + fm));
        vec2 bp = uBait.xy + vec2(cos(a), sin(a)) * r;
        float crumb = (1.0 - ss(0.9, 2.3, length(p - bp))) * uBait.w;
        col = mix(col, vec3(1.0, 0.92, 0.52), crumb * 0.88);
      }
    }

    // Gợn sóng tương tác rõ nét
    float rip = 0.0;
    for (int i = 0; i < 6; i++) {
      vec4 r = uRip[i];
      if (r.w <= 0.0) continue;
      float dist = length(p - r.xy), rad = r.z * 78.0;
      rip = max(rip, (1.0 - ss(0.9, 2.8, abs(dist - rad))) * exp(-r.z * 1.15) * r.w);
      rip = max(rip, (1.0 - ss(0.8, 2.2, abs(dist - rad * 0.58))) * exp(-r.z * 1.35) * r.w * 0.72);
      rip = max(rip, (1.0 - ss(0.7, 1.8, abs(dist - rad * 0.26))) * exp(-r.z * 1.65) * r.w * 0.45);
    }
    col = mix(col, FOAM, clamp(rip * 0.75, 0.0, 0.92));
    col = mix(col, FOAM, step(g2, rip * 0.85) * 0.55);

    // Sóng biển
    float dd = s - 2.0;
    float ph = (s + t * uSpeed) / lam + along;
    float fr = fract(ph);
    float dpx = min(fr, 1.0 - fr) * lam;
    float vis = (1.0 - ss(lam * 0.45, lam * 1.35, s)) * ss(0.0, 4.0, dd) * energy * uWaves;
    float brk = 1.0 - ss(8.0, 55.0, s);
    float cw = wv * (1.0 + 0.8 * brk);
    float crest = (1.0 - ss(0.7 * cw, 1.9 * cw, dpx)) * vis * (0.55 + 0.4 * brk);
    float crestSpray = exp(-fr * lam / (7.0 * cw)) * vis * (0.25 + 0.4 * brk);
    float burst = exp(-fu * 4.5) * ss(0.0, 0.04, fu);
    float spray = exp(-dd / (6.0 * wv)) * burst * energy * 0.45 * uWaves;
    float foam = max(crest, step(g2, max(spray, crestSpray)));
    col = mix(col, FOAM, foam);
  } else {
    float wet = ss(-7.0 - 3.0 * shoreE, 1.0, s) * 0.2;
    col = mix(sandRamp((g1 - 0.5) * 0.1 * gAmt), WETSAND, wet * 3.0);
    col *= 1.0 - 0.03 * ss(0.4, 0.8, N(p, 900.0).r) * (1.0 - wet);
    // Vỏ sò hồng trắng / sao biển khi click lên bãi cát Maldives
    for (int i = 0; i < 4; i++) {
      vec4 sm = uSand[i];
      if (sm.w <= 0.0) continue;
      vec2 sq = rot(sm.w) * (p - sm.xy);
      float sr = length(sq);
      if (sr < 18.0) {
        float fade = (1.0 - ss(3.0, 4.8, sm.z)) * ss(0.0, 0.18, sm.z);
        float sa = atan(sq.y, sq.x);
        float starR = 6.0 + 3.0 * cos(sa * 5.0);
        col = mix(col, WETSAND * 0.88, (1.0 - ss(starR, starR + 3.0, sr)) * 0.42 * fade);
        vec3 shCol = mix(vec3(0.98, 0.52, 0.44), vec3(1.0, 0.90, 0.82), 1.0 - sr / max(starR, 1.0));
        col = mix(col, shCol, (1.0 - ss(starR - 0.8, starR + 0.6, sr)) * 0.92 * fade);
      }
    }
  }

  // Lớp nước mỏng tràn lên bãi cát
  float reachBase = min(uRunup * 0.25 * shoreE * (0.85 + 0.3 * N(p, 900.0).g), uSigma * 0.78) * uWaves;
  float wk = (1.0 - ss(2.0, 38.0, s)) * mix(1.0, energy, ss(2.0, 10.0, s)) * uWaves;
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
      float front = layers[j].x, a = layers[j].y;
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

  // Thuyền kayak
  for (int i = 0; i < 4; i++) {
    if (float(i) >= uKayN) break;
    vec4 k = uKay[i];
    vec2 q = rot(-k.z) * (p - k.xy);
    if (dot(q, q) > 900.0) continue;
    float sd = hullSD(q);
    vec2 pd = paddleSD(q, k.w);
    vec3 kc = col;
    float mh = ss(0.7, -0.7, sd);
    if (mh > 0.0) {
      float tt = clamp(q.x / 22.0, -1.0, 1.0);
      float wd = 5.4 * (1.0 - tt * tt);
      vec3 hull = mix(vec3(0.99, 0.84, 0.16), vec3(1.0, 0.93, 0.45), 1.0 - ss(0.2, 0.7, abs(q.y) / max(wd, 0.5)));
      hull = mix(hull, vec3(0.90, 0.64, 0.10), ss(0.78, 1.0, abs(q.y) / max(wd, 0.5)));
      hull = mix(hull, vec3(0.97, 0.96, 0.9), 1.0 - ss(1.0, 1.6, length(q - vec2(13.0, 0.0))));
      hull = mix(hull, vec3(0.97, 0.96, 0.9), 1.0 - ss(1.0, 1.6, length(q - vec2(-12.0, 0.0))));
      hull = mix(hull, vec3(0.30, 0.24, 0.20), 1.0 - ss(0.0, 0.6, length((q - vec2(-1.0, 0.0)) / vec2(6.0, 2.8)) - 1.0));
      kc = mix(kc, hull, mh);
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

  // Chim biển
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
