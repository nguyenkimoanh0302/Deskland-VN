#version 300 es
// Deskland · Bể cá vàng (nhìn ngang qua thành kính). Dùng chung cho prototype/aquarium.html (WebGL2)
// và deskland-rs (OpenGL 3.3, include_str!). Toạ độ p tính theo px CSS, gốc ở góc trên-trái.
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uDebug;
uniform float uLight, uBubbles, uSurf;
uniform float uFishN;
uniform vec4 uFish[8];    // x, y, hướng (rad), pha vẫy đuôi
uniform vec4 uFishB[8];   // cỡ, kiểu đốm (0..1), biên độ uốn thân, độ hiện (0..1)
uniform float uPlantN;
uniform vec4 uPlant[32];  // gốc x, gốc y, chiều cao, hạt ngẫu nhiên
uniform vec4 uLine;       // phao x, lưỡi câu x, lưỡi câu y, độ hiện dây (0 = không có dây)
uniform vec4 uLineB;      // phao chìm (px), còn mồi (0/1), báo cắn câu (0..1), đang kéo cá (0/1)
uniform vec4 uFx[8];      // x, y, tuổi (s), loại: 0 tõm nước, 1 kéo cá lên, 2 bụi cát, 3 sóng dưới nước
out vec4 o;

const float TASKBAR = 48.0;
const vec3 W_TOP = vec3(0.40, 0.76, 0.94), W_MID = vec3(0.13, 0.50, 0.80), W_DEEP = vec3(0.05, 0.25, 0.50);
const vec3 SAND_HI = vec3(0.95, 0.76, 0.46), SAND_LO = vec3(0.78, 0.55, 0.30), SAND_RIM = vec3(0.58, 0.38, 0.20);

float hash1(float n) { return fract(sin(n * 127.1 + 311.7) * 43758.5453); }
float hash2(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}
float ss(float a, float b, float x) { float t = clamp((x - a) / (b - a), 0.0, 1.0); return t * t * (3.0 - 2.0 * t); }
vec4 N(vec2 p, float scale) { return texture(uNoise, p / scale + uSeed * 0.013); }
float fieldS(sampler2D t, vec2 p) { return texture(t, (p / uStep + 0.5) / uGrid).r; }
float segD(vec2 p, vec2 a, vec2 b) { vec2 pa = p - a, ba = b - a; return length(pa - ba * clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0)); }

// Mép cát đáy bể (CPU tính y hệt để đặt cây rong): đáy nằm trên thanh tác vụ
float sandTop(float x) {
  return uRes.y - TASKBAR - (46.0 + 18.0 * sin(x * 0.0042 + uSeed) + 10.0 * sin(x * 0.0113 + uSeed * 1.7 + 1.3));
}
// Mặt nước gợn nhẹ
float surfY(float x) { return uSurf + 2.2 * sin(x * 0.021 + uTime * 1.3) + 1.4 * sin(x * 0.053 - uTime * 2.0); }

// Vệt sáng khúc xạ (caustics) rọi trên cát
float caustic(vec2 q) {
  float t = uTime;
  vec2 a = q + vec2(sin(q.y * 1.3 + t * 0.6), cos(q.x * 1.1 - t * 0.5)) * 0.6;
  float c = abs(sin(a.x + t * 0.3) * sin(a.y - t * 0.25));
  return pow(1.0 - c, 7.0);
}
// Nhiễu tế bào: khoảng cách tới tâm gần nhất + mã ô
vec3 worley(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  float d1 = 8.0; vec2 id = vec2(0.0);
  for (int y = -1; y <= 1; y++) for (int x = -1; x <= 1; x++) {
    vec2 g = vec2(float(x), float(y)), c = i + g;
    vec2 r = g + vec2(hash2(c + uSeed), hash2(c + uSeed + 17.3)) * 0.8 + 0.1 - f;
    float d = dot(r, r);
    if (d < d1) { d1 = d; id = c; }
  }
  return vec3(sqrt(d1), id);
}

// Vẽ một chú cá vàng. q: toạ độ cục bộ (x về phía đầu, y hướng lên), đơn vị ~px ở cỡ 1.
// Trả về màu (rgb) và độ phủ (a).
vec4 goldfish(vec2 q, float ph, float spots, float bend) {
  // Thân uốn theo nhịp vẫy đuôi (càng về đuôi càng uốn)
  q.y += sin(ph) * bend * ss(4.0, -22.0, q.x) * (-q.x) * 0.16;
  vec4 acc = vec4(0.0);
  // Đuôi xòe (sau thân), có khe giữa, viền trắng ở mép
  vec2 tq = q - vec2(-12.5, 0.0);
  float r = length(tq);
  float a = atan(tq.y, -tq.x) - sin(ph) * 0.38;
  float L = 17.0 * (1.0 - 0.32 * exp(-a * a * 16.0)) * (1.0 + 0.06 * sin(a * 9.0 + ph));
  float tail = (1.0 - ss(-1.0, 1.0, r - L)) * (1.0 - ss(0.78, 0.95, abs(a))) * step(tq.x, 3.0);
  vec3 tc = mix(vec3(1.0, 0.52, 0.12), vec3(1.0, 0.93, 0.82), ss(0.45, 0.95, r / L));
  tc *= 0.92 + 0.08 * sin(a * 18.0);                       // gân vây
  acc = mix(acc, vec4(tc, 0.9), tail);
  // Vây lưng
  vec2 dq = q - vec2(-1.5, 7.0);
  float dorsH = 7.5 * (1.0 - abs(dq.x) / 7.5) * (0.9 + 0.1 * sin(ph * 0.7));
  float dors = step(0.0, dq.y) * (1.0 - ss(-0.6, 0.8, dq.y - dorsH)) * step(abs(dq.x), 7.5);
  acc = mix(acc, vec4(mix(vec3(0.98, 0.45, 0.10), vec3(1.0, 0.86, 0.70), ss(0.3, 1.0, dq.y / 7.5)), 0.9), dors);
  // Vây bụng / vây ngực vẫy
  vec2 pq = q - vec2(2.0, -6.5);
  pq.x -= sin(ph * 1.3) * 1.5;
  float pel = step(pq.y, 0.0) * (1.0 - ss(-0.6, 0.8, -pq.y - 5.5 * (1.0 - abs(pq.x) / 4.5))) * step(abs(pq.x), 4.5);
  acc = mix(acc, vec4(1.0, 0.70, 0.40, 0.85), pel);
  // Thân hình giọt nước: đầu tròn, thon về đuôi
  vec2 bq = vec2(q.x / (q.x > 0.0 ? 13.0 : 15.0), q.y / 9.5);
  float e = length(bq);
  float dB = (e - 1.0) * 9.5;
  float body = 1.0 - ss(-0.8, 0.8, dB);
  vec3 bc = mix(vec3(1.0, 0.72, 0.30), vec3(0.95, 0.40, 0.08), ss(-7.0, 6.0, q.y));   // bụng sáng, lưng đậm
  bc = mix(bc, vec3(0.86, 0.28, 0.05), ss(4.0, 9.0, q.y) * 0.6);
  // Vảy li ti
  vec2 sc = vec2(q.x * 0.55 + q.y * 0.3, q.y * 0.55);
  bc *= 0.94 + 0.06 * ss(0.3, 0.0, abs(fract(sc.x) - 0.5) * abs(fract(sc.y) - 0.5) * 4.0);
  // Đốm trắng (cá ranchu / oranda)
  float sp = ss(0.30, 0.45, 0.5 * sin(q.x * 0.24 + spots * 21.0) + 0.5 * sin(q.y * 0.30 + q.x * 0.11 + spots * 13.0) + (spots - 0.7) * 0.8);
  bc = mix(bc, vec3(1.0, 0.97, 0.92), sp * step(0.35, spots));
  // Mang + viền thân
  float gill = (1.0 - ss(0.0, 0.9, abs(length(q - vec2(11.5, 0.0)) - 7.0))) * step(q.x, 6.5) * step(abs(q.y), 6.0);
  bc = mix(bc, vec3(0.80, 0.30, 0.06), gill * 0.55);
  bc = mix(bc, vec3(0.70, 0.24, 0.05), ss(-1.6, 0.0, dB) * 0.55);
  // Bóng sáng lưng
  bc += vec3(0.20, 0.16, 0.10) * (1.0 - ss(0.0, 2.2, length((q - vec2(3.0, 5.0)) * vec2(0.35, 1.0))));
  acc = mix(acc, vec4(bc, 1.0), body);
  // Mắt
  float eye = 1.0 - ss(1.6, 2.4, length(q - vec2(8.3, 2.4)));
  acc.rgb = mix(acc.rgb, vec3(0.06, 0.05, 0.08), eye);
  acc.rgb = mix(acc.rgb, vec3(1.0), 1.0 - ss(0.4, 0.9, length(q - vec2(8.9, 3.0))));
  return acc;
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y * uScale - gl_FragCoord.y) / uScale;
  float t = uTime;
  float sy = surfY(p.x);

  // ---------- Nước: chuyển màu theo độ sâu + mảng sáng tối ----------
  float depth = clamp((p.y - sy) / max(uRes.y - sy, 1.0), 0.0, 1.0);
  vec3 col = mix(W_TOP, W_MID, ss(0.0, 0.5, depth));
  col = mix(col, W_DEEP, ss(0.42, 1.05, depth));
  col *= 0.93 + 0.14 * N(p, 1600.0).r;

  // Tia nắng rọi xiên từ mặt nước
  float u = p.x + (p.y - sy) * 0.42;
  float rays = (0.5 + 0.5 * sin(u * 0.010 + t * 0.22 + uSeed)) * (0.5 + 0.5 * sin(u * 0.026 - t * 0.37 + 2.0));
  rays = pow(rays, 3.0) * (1.0 - ss(0.0, 0.8, depth)) * uLight;
  col += vec3(0.60, 0.88, 1.0) * rays * 0.24;

  // Hạt lơ lửng trôi chậm
  vec2 spP = p + vec2(t * 3.0, -t * 5.0);
  vec2 cell = floor(spP / 34.0);
  if (hash2(cell + uSeed) > 0.82) {
    vec2 off = (vec2(hash2(cell + 1.3), hash2(cell + 2.7)) - 0.5) * 0.6;
    float d = length(fract(spP / 34.0) - 0.5 - off) * 34.0;
    col += vec3(0.75, 0.92, 1.0) * (1.0 - ss(0.3, 1.5, d)) * 0.22 * (1.0 - depth * 0.7);
  }

  // ---------- Cát: rạn cát ôm biểu tượng + đáy bể ----------
  float s = mix(fieldS(uFA, p), fieldS(uFB, p), uMix);
  float dBot = sandTop(p.x) - p.y;
  float dSand = min(s, dBot);
  col *= 1.0 - 0.22 * (1.0 - ss(0.0, 34.0, dSand));          // nước tối dần sát cát
  if (dSand < 1.5) {
    // Đáy bể tối dần xuống dưới; rạn cát quanh biểu tượng giữ sáng để biểu tượng dễ nhìn
    float inn = dBot < s ? clamp(-dBot / 70.0, 0.0, 1.0) : 0.15 * clamp(-s / 80.0, 0.0, 1.0);
    vec3 sc = mix(SAND_HI, SAND_LO, ss(0.0, 1.0, inn));
    sc *= 0.95 + 0.10 * N(p, 280.0).g;
    sc *= 1.0 + (hash2(floor(p * 1.4)) - 0.5) * 0.08;         // hạt cát
    sc += vec3(1.0, 0.95, 0.72) * caustic(p * 0.032) * 0.16 * (1.0 - inn) * (0.4 + uLight);
    // Mép trên (nước phía trên) được rọi sáng, mép dưới tối hơn
    float sUp = min(mix(fieldS(uFA, p - vec2(0.0, 7.0)), fieldS(uFB, p - vec2(0.0, 7.0)), uMix), sandTop(p.x) - p.y + 7.0) - dSand;
    sc *= 1.0 + 0.12 * clamp(sUp / 5.0, -1.0, 1.0) * (1.0 - ss(0.0, 22.0, -dSand));
    // Sỏi cuội sát mép
    vec3 wv = worley(p / 13.0);
    float hp = hash2(wv.yz + 4.1);
    if (hp > 0.62 && -dSand < 16.0) {
      float pr = 0.30 + 0.12 * hash2(wv.yz + 9.7);
      float pb = 1.0 - ss(pr - 0.06, pr, wv.x);
      vec3 pc = mix(vec3(0.55, 0.50, 0.46), vec3(0.85, 0.80, 0.72), hash2(wv.yz + 2.2));
      pc = mix(pc, pc * 0.7, ss(pr * 0.3, pr, wv.x));
      sc = mix(sc, pc, pb * 0.9);
    }
    float rim = 1.0 - ss(0.0, 3.0, -dSand);
    sc = mix(sc, SAND_RIM, rim * 0.6);
    col = mix(col, sc, ss(1.0, -1.0, dSand));
  }

  // ---------- Rong mọc trên cát ----------
  for (int i = 0; i < 32; i++) {
    if (float(i) >= uPlantN) break;
    vec4 pl = uPlant[i];
    if (abs(p.x - pl.x) > pl.z * 0.45 + 22.0 || p.y > pl.y + 4.0 || p.y < pl.y - pl.z - 12.0) continue;
    for (int k = 0; k < 3; k++) {
      float fk = float(k);
      float hk = pl.z * (0.62 + 0.38 * hash1(pl.w + fk * 3.1));
      float tt = (pl.y - p.y) / hk;
      if (tt < 0.0 || tt > 1.0) continue;
      float bx = pl.x + (fk - 1.0) * (5.0 + 3.0 * hash1(pl.w + fk));
      float sway = sin(t * 1.1 + pl.w * 3.0 + fk * 1.7 + tt * 2.3) * 7.0 * pow(tt, 1.5) + (fk - 1.0) * 7.0 * tt;
      float cx = bx + sway;
      float wdt = 4.4 * pow(1.0 - tt, 0.55) + 0.5;
      float d = abs(p.x - cx) - wdt;
      if (d < 1.0) {
        vec3 gc = mix(vec3(0.09, 0.42, 0.22), vec3(0.40, 0.80, 0.32), tt * (0.7 + 0.3 * hash1(pl.w + fk * 7.0)));
        gc += vec3(0.10, 0.12, 0.04) * (1.0 - ss(0.0, 1.2, abs(p.x - cx)));   // gân lá
        gc = mix(gc, gc * 0.62, ss(-1.8, 0.4, d));                          // viền tối
        col = mix(col, gc, ss(1.0, -0.4, d));
      }
    }
  }

  // ---------- Bong bóng nổi lên từ khóm rong ----------
  for (int i = 0; i < 32; i++) {
    if (float(i) >= uPlantN) break;
    vec4 pl = uPlant[i];
    if (hash1(pl.w * 7.3) > uBubbles) continue;
    for (int k = 0; k < 2; k++) {
      float sp = 0.07 + 0.06 * hash1(pl.w + float(k) * 5.0);
      float ph = fract(t * sp + float(k) * 0.5 + hash1(pl.w * 3.0));
      float y0 = pl.y - pl.z * 0.7, y1 = sy + 3.0;
      float by = mix(y0, y1, ph);
      float bx = pl.x + sin(ph * 11.0 + pl.w * 2.0) * 6.0;
      float rr = 2.4 + 2.2 * hash1(pl.w + float(k) * 9.0) + ph * 1.8;
      float d = length(p - vec2(bx, by)) - rr;
      if (d < 1.5) {
        float fade = 1.0 - ss(0.88, 1.0, ph);
        float ring = 1.0 - ss(0.0, 1.1, abs(d));
        col = mix(col, vec3(0.85, 0.96, 1.0), (ring * 0.7 + step(d, 0.0) * 0.10) * fade);
        col = mix(col, vec3(1.0), (1.0 - ss(0.0, 1.0, length(p - vec2(bx - rr * 0.35, by - rr * 0.35)))) * fade);
      }
    }
  }

  // ---------- Mặt nước + khoảng không phía trên ----------
  if (p.y < sy && dSand > 0.0) {                                   // rạn cát cao thì nhô lên khỏi mặt nước
    float k = clamp(p.y / max(sy, 1.0), 0.0, 1.0);
    col = mix(vec3(0.80, 0.92, 0.98), vec3(0.56, 0.82, 0.95), k);
  }
  col += vec3(0.85, 0.95, 1.0) * (1.0 - ss(0.0, 1.8, abs(p.y - sy))) * 0.55 * ss(-1.0, 2.0, dSand);          // vệt sáng mặt nước
  col += vec3(0.30, 0.45, 0.50) * (1.0 - ss(0.0, 16.0, p.y - sy)) * step(sy, p.y) * 0.25;

  // ---------- Dây câu, phao, lưỡi câu, mồi ----------
  if (uLine.w > 0.01) {
    float bxp = uLine.x;
    float by = surfY(bxp) + uLineB.x;
    vec2 hook = uLine.yz;
    float la = uLine.w;
    // Dây từ trên cao xuống phao (trong không khí) và từ phao xuống lưỡi câu (dưới nước)
    float d1 = segD(p, vec2(bxp + 30.0, -20.0), vec2(bxp, by - 6.0));
    col = mix(col, vec3(0.20, 0.24, 0.28), (1.0 - ss(0.3, 1.0, d1)) * 0.75 * la);
    float d2 = segD(p, vec2(bxp, by + 5.0), hook + vec2(0.0, -9.0));
    col = mix(col, vec3(0.88, 0.95, 1.0), (1.0 - ss(0.25, 0.9, d2)) * 0.55 * la);
    // Lưỡi câu chữ J
    float shank = segD(p, hook + vec2(0.0, -9.0), hook);
    float bend = abs(length(p - (hook + vec2(-3.4, 0.0))) - 3.4);
    bend = max(bend, -(p.y - hook.y));
    float barb = segD(p, hook + vec2(-6.8, 0.0), hook + vec2(-5.4, -3.0));
    float hd = min(min(shank, bend), barb);
    col = mix(col, vec3(0.72, 0.76, 0.80), (1.0 - ss(0.5, 1.2, hd)) * la);
    // Mồi giun
    if (uLineB.y > 0.5) {
      float wg = sin(t * 5.0) * 1.2;
      float worm = min(segD(p, hook + vec2(-7.0, 1.5 + wg), hook + vec2(-2.0, 3.0)), segD(p, hook + vec2(-2.0, 3.0), hook + vec2(2.0, 1.0 - wg)));
      col = mix(col, mix(vec3(0.95, 0.45, 0.55), vec3(0.75, 0.25, 0.35), ss(0.0, 2.0, worm)), (1.0 - ss(1.6, 2.4, worm)) * la);
    }
    // Phao đỏ trắng
    vec2 bq = p - vec2(bxp, by);
    float bd = length(bq * vec2(1.0, 0.9)) - 6.5;
    vec3 bobc = bq.y < 0.0 ? vec3(0.92, 0.16, 0.12) : vec3(0.97, 0.97, 0.95);
    bobc = mix(bobc, bobc * 0.6, ss(-2.0, 0.0, bd));
    bobc += 0.25 * (1.0 - ss(0.0, 2.0, length(bq - vec2(-2.2, -2.4))));
    float stick = segD(p, vec2(bxp, by - 6.0), vec2(bxp, by - 12.0));
    col = mix(col, vec3(0.92, 0.16, 0.12), (1.0 - ss(0.6, 1.3, stick)) * la);
    col = mix(col, bobc, (1.0 - ss(-0.6, 0.6, bd)) * la);
    // Báo cá cắn câu: dấu "!" vàng nhún nhảy + vòng sóng quanh phao
    if (uLineB.z > 0.01) {
      float bounce = abs(sin(t * 9.0)) * 5.0;
      vec2 ex = vec2(bxp + 22.0, by - 6.0 - bounce * 0.6);
      float bar = segD(p, ex + vec2(0.0, -7.0), ex + vec2(0.0, 3.0)) - 2.1;
      float dot1 = length(p - (ex + vec2(0.0, 8.5))) - 2.2;
      float ed = min(bar, dot1);
      vec3 ec = mix(vec3(1.0, 0.86, 0.18), vec3(0.35, 0.22, 0.05), ss(-1.2, 0.2, ed));
      col = mix(col, ec, (1.0 - ss(0.3, 1.2, ed)) * uLineB.z);
      float rr = 10.0 + fract(t * 1.6) * 26.0;
      float ring = abs(length((p - vec2(bxp, by)) * vec2(1.0, 2.6)) - rr);
      col = mix(col, vec3(1.0), (1.0 - ss(0.0, 1.4, ring)) * (1.0 - fract(t * 1.6)) * 0.7 * uLineB.z);
    }
  }

  // ---------- Cá vàng ----------
  for (int i = 0; i < 8; i++) {
    if (float(i) >= uFishN) break;
    vec4 F = uFish[i], B = uFishB[i];
    vec2 d = p - F.xy;
    float R = 34.0 * B.x;
    if (dot(d, d) > R * R || B.w < 0.01) continue;
    vec2 fw = vec2(cos(F.z), sin(F.z));
    vec2 up = fw.x >= 0.0 ? vec2(fw.y, -fw.x) : vec2(-fw.y, fw.x);
    vec2 q = vec2(dot(d, fw), dot(d, up)) / B.x;
    // Bóng mờ phía dưới
    vec2 sq = vec2(dot(d - vec2(5.0, 9.0), fw), dot(d - vec2(5.0, 9.0), up)) / B.x;
    col *= 1.0 - 0.12 * B.w * (1.0 - ss(0.4, 1.3, length(sq / vec2(16.0, 9.0))));
    vec4 fc = goldfish(q, F.w, B.y, B.z);
    col = mix(col, fc.rgb, fc.a * B.w);
  }

  // ---------- Hiệu ứng: tõm nước, kéo cá, bụi cát, sóng ----------
  for (int i = 0; i < 8; i++) {
    vec4 f = uFx[i];
    if (f.z < 0.0) continue;
    float age = f.z, kind = f.w;
    if (kind < 0.5 || kind > 0.5 && kind < 1.5) {
      // Vòng sóng loang trên mặt nước (+ giọt bắn khi kéo cá)
      float big = kind > 0.5 ? 1.8 : 1.0;
      float life = 1.1 * big;
      if (age < life) {
        float rr = (6.0 + age * 46.0) * big;
        float ring = abs(length((p - vec2(f.x, surfY(f.x) + 1.0)) * vec2(1.0, 3.2)) - rr);
        col = mix(col, vec3(1.0), (1.0 - ss(0.0, 1.6, ring)) * (1.0 - age / life) * 0.8);
      }
      if (kind > 0.5 && age < 1.2) {
        for (int j = 0; j < 9; j++) {
          float a = 0.5 + float(j) / 8.0 * 2.14;
          float v = 120.0 + 60.0 * hash1(float(j) + f.x);
          vec2 dp = vec2(f.x + cos(a) * v * age * 0.6, surfY(f.x) - (sin(a) * v * age - 260.0 * age * age));
          float dd = length(p - dp) - (2.6 - age * 1.2);
          col = mix(col, vec3(0.92, 0.98, 1.0), (1.0 - ss(-0.5, 0.8, dd)) * (1.0 - age / 1.2));
        }
      }
      if (kind > 0.5 && age < 1.8) {
        // Lấp lánh vàng: sao 4 cánh toả ra quanh chỗ kéo cá
        for (int j = 0; j < 7; j++) {
          float a = float(j) / 7.0 * 6.283 + age * 0.8;
          vec2 c = vec2(f.x, surfY(f.x) - 22.0) + vec2(cos(a), sin(a) * 0.7) * (18.0 + age * 55.0);
          vec2 sq = abs(p - c);
          float star = min(sq.x * 3.0 + sq.y, sq.y * 3.0 + sq.x) - (8.0 - age * 3.5);
          col = mix(col, vec3(1.0, 0.90, 0.35), (1.0 - ss(-0.5, 0.8, star)) * (1.0 - age / 1.8));
        }
      }
    } else if (kind < 2.5) {
      // Bụi cát tung lên khi bấm vào cát
      if (age < 1.2) {
        for (int j = 0; j < 7; j++) {
          float a = -0.4 - float(j) / 6.0 * 2.34;
          vec2 c = f.xy + vec2(cos(a), sin(a)) * (6.0 + age * (24.0 + 10.0 * hash1(float(j)))) + vec2(0.0, age * age * 18.0);
          float dd = length(p - c) - (3.2 + age * 2.5);
          col = mix(col, SAND_HI * 1.05, (1.0 - ss(-1.0, 1.0, dd)) * (1.0 - age / 1.2) * 0.75);
        }
      }
    } else if (age < 1.0) {
      // Sóng tròn dưới nước (cá giật mình)
      float ring = abs(length(p - f.xy) - (8.0 + age * 60.0));
      col = mix(col, vec3(0.80, 0.95, 1.0), (1.0 - ss(0.0, 2.0, ring)) * (1.0 - age) * 0.35);
    }
  }

  // ---------- Thành kính bể: tối dần ra mép + vệt phản chiếu cong ----------
  vec2 c = p / uRes - 0.5;
  float e = length(c * vec2(1.0, 1.18));
  col *= 1.0 - 0.30 * ss(0.42, 0.90, e);
  float th = atan(c.y, c.x);
  // Vệt phản chiếu rộng, mềm (như thành bể cong trong ảnh mẫu), không phải nét mảnh
  float streak = (1.0 - ss(0.0, 0.045, abs(e - 0.60))) * ss(-2.9, -2.6, th) * (1.0 - ss(-2.2, -1.95, th));
  streak += 0.7 * (1.0 - ss(0.0, 0.03, abs(e - 0.66))) * ss(-0.4, -0.1, th) * (1.0 - ss(0.3, 0.6, th));
  col += vec3(0.9, 0.97, 1.0) * streak * 0.07;

  if (uDebug > 0.5) col = mix(col, vec3(0.9, 0.15, 0.15), 1.0 - ss(0.0, 1.5, abs(s)));
  o = vec4(col, 1.0);
}
