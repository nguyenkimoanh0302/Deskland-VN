#version 300 es
// Deskland · Hiên nhà ngày hạ (Summer Porch Corgis)
// Dùng chung cho cả prototype/summer-porch.html (WebGL2) và deskland-rs (OpenGL 3.3).
// Bối cảnh màu nước / cel-shading Ghibli: hiên gỗ mái ngói âm dương, đàn Corgi ngủ trưa lăn mình
// trước quạt bàn quay đầu, đĩa dưa hấu, sách mở, chuông gió đồng, luống vườn xanh ôm biểu tượng.
precision highp float;

uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uDebug;

uniform float uDayMode;      // 0 = Trưa hè nắng vàng, 1 = Hoàng hôn mật ong, 2 = Đêm hè đom đóm
uniform float uSunbeams;     // 0..1 cường độ tia nắng & hạt nắng
uniform float uFanSpeed;     // 0..2.5 tốc độ gió quạt thực tế
uniform vec4 uFanState;      // x, y, góc quay đầu quạt (-1..1), pha cánh quạt
uniform vec4 uProps;         // x = góc chuông gió, y = tuổi sóng chuông (<0 tắt), z = số miếng dưa đã cắn (0..3), w = độ nảy chim xanh (0..1)
uniform float uCorgiN;
uniform vec4 uCorgiA[4];     // x, y, pose (0=sploot, 1=bellyUp, 2=sideNap), rollBounce (0..1)
uniform vec4 uCorgiB[4];     // cỡ, kiểu lông (0=vàng cam, 1=em bé nơ đỏ, 2=tam thể), pha thở, happyTimer (0..1)
uniform float uBugN;
uniform vec4 uBugs[10];      // x, y, hướng, pha cánh / độ sáng đom đóm
uniform vec4 uFx[6];         // x, y, tuổi (s), loại: 0=tim hồng, 1=gió quạt, 2=sóng chuông, 3=dưa hấu mát, 4=lá vườn

out vec4 o;

const float PI = 3.14159265;
const vec3 INK = vec3(0.235, 0.165, 0.118);

float ss(float a, float b, float x) {
  float t = clamp((x - a) / (b - a), 0.0, 1.0);
  return t * t * (3.0 - 2.0 * t);
}

float hash2(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

mat2 rot(float a) {
  float c = cos(a), s = sin(a);
  return mat2(c, -s, s, c);
}

vec4 N(vec2 p, float sc) {
  return texture(uNoise, p / sc + uSeed * 0.013);
}

float fieldS(sampler2D t, vec2 p) {
  return texture(t, (p / uStep + 0.5) / uGrid).r;
}

float sdSeg(vec2 p, vec2 a, vec2 b) {
  vec2 pa = p - a, ba = b - a;
  return length(pa - ba * clamp(dot(pa, ba) / max(dot(ba, ba), 1e-4), 0.0, 1.0));
}

float sdBox(vec2 p, vec2 b, float r) {
  vec2 q = abs(p) - b + r;
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

float sdEllipse(vec2 p, vec2 r) {
  float k0 = length(p / r);
  float k1 = length(p / (r * r));
  return k0 * (k0 - 1.0) / max(k1, 1e-4);
}

// Tô mảng phẳng kèm viền mực nâu ấm kiểu Ghibli
void strokeFill(inout vec3 col, float d, vec3 fillC, float strokeW, float alpha) {
  float aFill = ss(0.85, -0.85, d) * alpha;
  float aInk = ss(strokeW + 0.85, strokeW - 0.65, abs(d)) * alpha;
  col = mix(col, fillC, aFill);
  col = mix(col, INK, aInk * 0.86);
}

float cross2(vec2 a, vec2 b) { return a.x * b.y - a.y * b.x; }

// Kiểm tra điểm nằm trong tứ giác lồi (theo chiều kim đồng hồ trên hệ trục y hướng xuống)
float sdQuad(vec2 p, vec2 a, vec2 b, vec2 c, vec2 d) {
  vec2 e0 = b - a, e1 = c - b, e2 = d - c, e3 = a - d;
  vec2 v0 = p - a, v1 = p - b, v2 = p - c, v3 = p - d;
  float d0 = sdSeg(p, a, b), d1 = sdSeg(p, b, c), d2 = sdSeg(p, c, d), d3 = sdSeg(p, d, a);
  float md = min(min(d0, d1), min(d2, d3));
  bool s0 = cross2(e0, v0) >= 0.0;
  bool s1 = cross2(e1, v1) >= 0.0;
  bool s2 = cross2(e2, v2) >= 0.0;
  bool s3 = cross2(e3, v3) >= 0.0;
  return (s0 && s1 && s2 && s3) ? -md : md;
}

// SDF trái tim nhỏ cho hiệu ứng gãi bụng Corgi
float sdHeart(vec2 p) {
  p.x = abs(p.x);
  p.y = -p.y + 0.35;
  if (p.y + p.x > 1.0) return sqrt(dot(p - vec2(0.25, 0.75), p - vec2(0.25, 0.75))) - 0.3535;
  return sqrt(min(dot(p - vec2(0.0, 1.0), p - vec2(0.0, 1.0)),
                  dot(p - 0.5 * max(p.x + p.y, 0.0), p - 0.5 * max(p.x + p.y, 0.0)))) * sign(p.x - p.y);
}

// Vẽ chú Corgi mập mạp trên hiên gỗ (3 tư thế: 0=sploot đùi gà, 1=ngửa phơi bụng, 2=nằm nghiêng cuộn)
void drawCorgi(inout vec3 col, vec2 p, vec4 ca, vec4 cb, float s, float shade) {
  float sc = cb.x * s * 1.72;
  vec2 c = ca.xy;
  // Nhảy nhẹ khi lăn mình hoặc được gãi bụng
  float bounce = sin(ca.w * PI) * 8.0 * sc + sin(cb.w * PI * 3.0) * 3.5 * sc * ss(0.0, 0.25, cb.w);
  vec2 q = (p - vec2(c.x, c.y - bounce)) / sc;
  // Lật squash ngang khi đang lăn chuyển tư thế
  float rollSquash = 1.0 - 0.32 * sin(ca.w * PI);
  q.x /= max(rollSquash, 0.45);
  if (dot(q, q) > 6200.0) return;

  int pose = int(floor(ca.z + 0.5));
  int coat = int(floor(cb.y + 0.5));
  float br = sin(cb.z) * 1.3;
  float wiggle = sin(uTime * 14.0 + cb.z * 2.0) * (0.35 + 1.8 * cb.w + 1.4 * sin(ca.w * PI));

  vec3 furMain = coat == 2 ? vec3(0.30, 0.24, 0.21) : (coat == 1 ? vec3(0.86, 0.45, 0.18) : vec3(0.91, 0.56, 0.20));
  vec3 furWarm = vec3(0.93, 0.62, 0.26);
  vec3 cream   = vec3(0.99, 0.97, 0.91);
  vec3 pawPink = vec3(0.95, 0.64, 0.66);
  furMain *= shade;
  furWarm *= shade;
  cream   *= mix(1.0, shade, 0.72);

  // Bóng đổ mềm trên mặt gỗ
  float shD = sdEllipse((p - vec2(c.x + 2.0 * sc, c.y + 10.0 * sc)) / sc, vec2(46.0, 17.0));
  col = mix(col, col * 0.66, ss(4.0, -6.0, shD) * 0.55);

  if (pose == 0) {
    // --- TƯ THẾ 0: Nằm sấp duỗi hai chân đùi gà (Sploot) hướng đầu về trái-dưới ---
    vec2 r = rot(-0.24) * q;
    // Hai đùi gà phía sau duỗi thẳng
    float legL = sdBox(rot(0.18) * (r - vec2(34.0, -6.5 + wiggle * 0.6)), vec2(12.5, 5.2), 4.8);
    float legR = sdBox(rot(-0.14) * (r - vec2(35.5, 6.8 - wiggle * 0.6)), vec2(12.5, 5.2), 4.8);
    strokeFill(col, min(legL, legR) * sc, cream, 1.15, 1.0);
    // Đệm thịt hồng dưới bàn chân sau
    float pad1 = length(r - vec2(43.5, -8.0 + wiggle * 0.6)) - 2.4;
    float pad2 = length(r - vec2(44.5, 8.2 - wiggle * 0.6)) - 2.4;
    col = mix(col, pawPink, ss(0.8, -0.5, min(pad1, pad2) * sc));

    // Hai chân trước duỗi dưới cằm
    float fLeg = sdBox(r - vec2(-33.0, 6.5), vec2(11.0, 4.5), 4.2);
    strokeFill(col, fLeg * sc, cream, 1.1, 1.0);

    // Thân bánh mì tròn phập phồng thở
    float body = sdEllipse(r - vec2(4.0, -0.5 - br * 0.45), vec2(32.0, 16.5 + br * 0.55));
    vec3 bCol = furMain;
    if (coat == 2) bCol = mix(furMain, furWarm, ss(-8.0, -20.0, r.x));
    // Mảng mông trắng tròn & cổ trắng đặc trưng Corgi
    bCol = mix(bCol, cream, ss(14.0, 23.0, r.x));
    bCol = mix(bCol, cream, ss(-6.0, -14.0, r.x) * ss(2.0, 10.0, r.y));
    bCol = mix(bCol, cream, ss(7.0, 13.0, r.y));
    // Vệt nắng trên lưng
    bCol += vec3(0.08, 0.06, 0.02) * ss(2.0, -10.0, r.y);
    strokeFill(col, body * sc, bCol, 1.25, 1.0);

    // Đuôi mẩu tròn nhỏ nhúc nhích
    float tail = sdEllipse(r - vec2(33.5, -2.5 + wiggle * 0.9), vec2(5.2, 4.0));
    strokeFill(col, tail * sc, cream, 1.05, 1.0);

    // Đầu & mõm trắng nằm sát sàn
    vec2 hd = r - vec2(-25.5, 1.5);
    float earB = sdEllipse(rot(-0.45) * (hd - vec2(2.0, -11.5)), vec2(5.8, 9.5));
    strokeFill(col, earB * sc, furWarm * 0.92, 1.1, 1.0);
    float headD = min(sdEllipse(hd, vec2(14.5, 11.2)), sdEllipse(hd - vec2(-11.0, 3.2), vec2(8.5, 5.8)));
    vec3 hCol = mix(furWarm, cream, ss(-3.0, -11.0, hd.x) + ss(3.0, 8.0, hd.y) * 0.7);
    strokeFill(col, headD * sc, hCol, 1.2, 1.0);
    float earF = sdEllipse(rot(0.38 + wiggle * 0.04) * (hd - vec2(6.5, -10.0)), vec2(6.0, 10.2));
    vec3 earC = mix(furWarm, pawPink * 0.95, ss(2.5, -2.0, length((hd - vec2(6.5, -9.5)) * vec2(1.0, 0.65)) - 3.5));
    strokeFill(col, earF * sc, earC, 1.1, 1.0);

    // Nơ đỏ cho Corgi em bé
    if (coat == 1) {
      float bow = min(sdEllipse(r - vec2(-11.0, -8.0), vec2(5.5, 3.4)), sdEllipse(r - vec2(-4.0, -6.5), vec2(5.5, 3.4)));
      strokeFill(col, bow * sc, vec3(0.86, 0.22, 0.18) * shade, 1.0, 1.0);
    }

    // Mắt nhắm ngủ bình yên & chóp mũi đen
    float eye = abs(length(hd - vec2(-4.5, -0.5)) - 3.0) - 0.7;
    if ((hd - vec2(-4.5, -0.5)).y > 0.2) col = mix(col, INK, ss(0.9, -0.2, eye * sc));
    col = mix(col, INK, ss(0.8, -0.5, (length(hd - vec2(-18.2, 2.2)) - 2.2) * sc));

  } else if (pose == 1) {
    // --- TƯ THẾ 1: Nằm ngửa phơi bụng trắng tròn xoe, 4 chân ngắn giơ lên trời ---
    vec2 r = rot(0.22) * q;
    // Đôi tai xoè trên sàn gỗ phía trên đầu
    float earL = sdEllipse(rot(-0.55) * (r - vec2(-9.5, -26.0)), vec2(6.2, 9.5));
    float earR = sdEllipse(rot(0.55) * (r - vec2(9.5, -26.0)), vec2(6.2, 9.5));
    strokeFill(col, min(earL, earR) * sc, furWarm, 1.1, 1.0);

    // Thân quả lê mũm mĩm phơi bụng trắng
    vec2 bp = r - vec2(0.0, 2.0);
    float radX = mix(17.5, 25.5, ss(-18.0, 16.0, bp.y)) + br * 0.55;
    float body = sdEllipse(bp, vec2(radX, 27.5 + br * 0.4));
    // Hai mép sườn màu cam vàng ôm lấy bụng trắng ở giữa
    float sideMask = ss(radX * 0.56, radX * 0.86, abs(bp.x)) * ss(20.0, -14.0, bp.y);
    vec3 bCol = mix(cream, furMain, sideMask * 0.92);
    // Bóng khối mềm dưới bụng
    bCol *= 0.95 + 0.05 * ss(18.0, -12.0, bp.y);
    strokeFill(col, body * sc, bCol, 1.25, 1.0);

    // Hai đùi sau xoè sang hai bên phía dưới
    vec2 hL = rot(0.55) * (r - vec2(-19.5, 22.0));
    vec2 hR = rot(-0.55) * (r - vec2(19.5, 22.0));
    float legHL = sdBox(hL - vec2(0.0, wiggle * 0.7), vec2(7.5, 11.5), 6.5);
    float legHR = sdBox(hR + vec2(0.0, wiggle * 0.7), vec2(7.5, 11.5), 6.5);
    strokeFill(col, min(legHL, legHR) * sc, cream, 1.15, 1.0);
    // Đệm chân hồng xinh
    float pL = length(hL - vec2(0.0, 7.2 + wiggle * 0.7)) - 2.5;
    float pR = length(hR - vec2(0.0, 7.2 - wiggle * 0.7)) - 2.5;
    col = mix(col, pawPink, ss(0.8, -0.5, min(pL, pR) * sc));

    // Hai tay ngắn gập trước ngực co giật nhẹ khi mơ
    vec2 fL = rot(-0.35 + wiggle * 0.08) * (r - vec2(-12.5, -6.5));
    vec2 fR = rot(0.35 - wiggle * 0.08) * (r - vec2(12.5, -6.5));
    float pawFL = sdBox(fL, vec2(5.2, 8.8), 4.6);
    float pawFR = sdBox(fR, vec2(5.2, 8.8), 4.6);
    strokeFill(col, min(pawFL, pawFR) * sc, cream, 1.1, 1.0);

    if (coat == 1) {
      float ribbon = sdBox(r - vec2(0.0, -15.5), vec2(11.0, 3.2), 2.5);
      strokeFill(col, ribbon * sc, vec3(0.86, 0.22, 0.18) * shade, 0.95, 1.0);
    }

    // Mõm nhỏ ngửa lên trời & cái miệng chữ w ngủ say
    col = mix(col, INK, ss(0.8, -0.5, (length(r - vec2(0.0, -24.5)) - 2.2) * sc));
    float mouth = min(abs(length(r - vec2(-2.2, -21.8)) - 2.1), abs(length(r - vec2(2.2, -21.8)) - 2.1)) - 0.55;
    if (r.y > -22.2) col = mix(col, INK, ss(0.8, -0.2, mouth * sc));

  } else {
    // --- TƯ THẾ 2: Nằm nghiêng cuộn tròn quay mặt về phía quạt mát ---
    vec2 r = rot(-0.12) * q;
    // Bốn bàn chân trắng chụm phía trước bụng
    float paws = min(
      min(sdBox(r - vec2(12.0, 12.5 + wiggle * 0.5), vec2(7.5, 4.4), 3.8),
          sdBox(r - vec2(17.5, 8.0 - wiggle * 0.5), vec2(7.2, 4.2), 3.8)),
      min(sdBox(r - vec2(-14.0, 14.5 - wiggle * 0.4), vec2(8.5, 4.8), 4.0),
          sdBox(r - vec2(-8.0, 17.5 + wiggle * 0.4), vec2(8.0, 4.6), 4.0))
    );
    strokeFill(col, paws * sc, cream, 1.1, 1.0);

    // Thân cong hạt đậu tròn trịa
    vec2 bp = r - vec2(-2.0, -1.0 - br * 0.45);
    bp.y += 0.008 * bp.x * bp.x;
    float body = sdEllipse(bp, vec2(29.5, 17.5 + br * 0.55));
    vec3 bCol = mix(furMain, cream, ss(3.5, 11.5, bp.y));
    bCol = mix(bCol, cream, ss(12.0, 21.0, bp.x) * ss(-3.0, 8.0, bp.y));
    bCol += vec3(0.09, 0.06, 0.02) * ss(0.0, -13.0, bp.y);
    strokeFill(col, body * sc, bCol, 1.25, 1.0);

    // Đuôi cuộn tròn ở mông trái
    float tail = sdEllipse(rot(0.5) * (r - vec2(-29.5, 6.0)), vec2(6.8, 4.8));
    strokeFill(col, tail * sc, mix(furWarm, cream, 0.65), 1.1, 1.0);

    // Đầu tròn & tai cụp hướng về quạt
    vec2 hd = r - vec2(20.5, -5.5);
    float earB = sdEllipse(rot(-0.25) * (hd - vec2(-2.5, -12.0)), vec2(5.4, 8.8));
    strokeFill(col, earB * sc, furWarm * 0.9, 1.05, 1.0);
    float headD = min(sdEllipse(hd, vec2(13.8, 11.8)), sdEllipse(hd - vec2(10.5, 2.5), vec2(7.8, 5.5)));
    vec3 hCol = mix(furWarm, cream, ss(1.5, 8.5, hd.y) + ss(5.0, 12.0, hd.x) * 0.65);
    strokeFill(col, headD * sc, hCol, 1.2, 1.0);
    float earF = sdEllipse(rot(-0.55 - wiggle * 0.05) * (hd - vec2(-6.5, -9.8)), vec2(5.6, 9.6));
    strokeFill(col, earF * sc, furWarm, 1.1, 1.0);

    // Má hồng + mắt nhắm mỉm cười + mũi đen
    col = mix(col, pawPink, ss(4.2, 0.5, length(hd - vec2(5.0, 3.2))) * 0.42);
    float eye = abs(length(hd - vec2(4.8, -1.2)) - 3.0) - 0.65;
    if ((hd - vec2(4.8, -1.2)).y > 0.1) col = mix(col, INK, ss(0.9, -0.2, eye * sc));
    col = mix(col, INK, ss(0.8, -0.5, (length(hd - vec2(17.2, 1.2)) - 2.2) * sc));
  }
}

// Vẽ chiếc quạt bàn cổ điển quay đầu trái-phải trên hiên gỗ
void drawDeskFan(inout vec3 col, vec2 p, vec4 fs, float sIn, float shade) {
  float s = sIn * 1.28;
  vec2 base = fs.xy;
  vec2 q = (p - base) / s;
  if (abs(q.x) > 70.0 || q.y < -135.0 || q.y > 28.0) return;

  // Bóng chân đế quạt
  col = mix(col, col * 0.68, ss(3.0, -5.0, sdEllipse(q - vec2(2.0, 6.0), vec2(26.0, 9.0))) * 0.5);

  // Đế quạt vuông vát cổ điển có nút bấm màu xanh/đỏ
  vec2 bq = rot(0.18) * (q - vec2(0.0, 1.0));
  float standBase = sdBox(bq, vec2(20.0, 9.5), 3.5);
  strokeFill(col, standBase * s, vec3(0.76, 0.80, 0.75) * shade, 1.15, 1.0);
  col = mix(col, vec3(0.25, 0.58, 0.48) * shade, ss(2.2, 0.5, length(bq - vec2(-6.0, 1.5))));
  col = mix(col, vec3(0.84, 0.34, 0.28) * shade, ss(2.2, 0.5, length(bq - vec2(0.0, 1.5))));

  // Cổ quạt
  float neck = sdBox(q - vec2(1.5, -24.0), vec2(4.2, 24.0), 2.0);
  strokeFill(col, neck * s, vec3(0.68, 0.73, 0.69) * shade, 1.1, 1.0);

  // Lồng quạt elip co giãn ngang theo góc quay đầu fs.z (-1..1)
  float yaw = fs.z;
  vec2 hc = q - vec2(yaw * 5.5, -66.0);
  float cageRx = mix(30.0, 19.5, abs(yaw) * 0.75);
  float cageRy = 35.0;

  // Bầu động cơ phía sau
  float motor = sdEllipse(hc - vec2(-yaw * 10.0, 2.0), vec2(12.0, 11.0));
  strokeFill(col, motor * s, vec3(0.54, 0.60, 0.57) * shade, 1.05, 1.0);

  // Cánh quạt xanh ngọc quay mờ bên trong lồng
  vec2 bld = vec2(hc.x / max(cageRx / 32.0, 0.45), hc.y);
  float r = length(bld);
  if (r < 31.0) {
    float a = atan(bld.y, bld.x) + fs.w;
    float petal = sin(a * 3.0);
    float bladeMask = ss(0.15, 0.65, petal) * ss(30.0, 22.0, r) * ss(4.0, 8.0, r);
    float blurDisc = ss(29.0, 10.0, r) * clamp(uFanSpeed * 0.28, 0.0, 0.42);
    vec3 bladeCol = vec3(0.56, 0.74, 0.72) * shade;
    col = mix(col, bladeCol, max(bladeMask * 0.68, blurDisc));
  }

  // Nan lồng quạt mảnh + vành lồng quạt + tâm tròn
  float cageD = sdEllipse(hc, vec2(cageRx, cageRy));
  if (cageD < 1.5) {
    float ang = atan(hc.y / cageRy, hc.x / cageRx);
    float spokes = abs(sin(ang * 8.0));
    float ringMid = abs(cageD + 13.0);
    float wire = max(ss(0.14, 0.0, spokes) * ss(1.0, -3.0, cageD), ss(1.0, 0.0, ringMid));
    col = mix(col, INK, wire * 0.42);
  }
  float rim = abs(cageD) - 1.3;
  col = mix(col, vec3(0.82, 0.85, 0.80) * shade, ss(1.2, -0.4, rim * s));
  col = mix(col, INK, ss(1.6, 0.3, abs(rim) * s) * 0.85);

  float hub = sdEllipse(hc, vec2(5.2 * cageRx / 32.0, 6.0));
  strokeFill(col, hub * s, vec3(0.86, 0.88, 0.83) * shade, 1.0, 1.0);

  // Dải ruy-băng đỏ buộc trước lồng quạt bay phấp phới theo sức gió
  float windLift = clamp(uFanSpeed, 0.0, 2.0);
  vec2 rb0 = hc + vec2(-yaw * 8.0 - 6.0, 10.0);
  vec2 rb1 = rb0 + vec2((-14.0 - 9.0 * windLift), 8.0 - 5.0 * windLift + sin(uTime * 11.0) * (1.2 + 2.8 * windLift));
  float rib = sdSeg(hc, rb0, rb1) - 1.5;
  strokeFill(col, rib * s, vec3(0.90, 0.30, 0.25), 0.8, 0.95);
}

// Đĩa dưa hấu đỏ mát lạnh & quyển sách mở phấp phới trang giấy
void drawMelonAndBook(inout vec3 col, vec2 p, vec2 anchor, float bites, float sIn, float shade) {
  float s = sIn * 1.38;
  vec2 q = (p - anchor) / s;
  if (abs(q.x) > 85.0 || abs(q.y) > 48.0) return;

  // --- Đĩa sứ trắng & miếng dưa hấu đỏ (bên phải) ---
  vec2 mq = q - vec2(26.0, -6.0);
  float plate = sdEllipse(mq - vec2(0.0, 6.0), vec2(24.0, 9.5));
  strokeFill(col, plate * s, vec3(0.92, 0.95, 0.96) * shade, 1.1, 1.0);

  // Miếng dưa hình bán nguyệt (vỏ xanh, cùi trắng, ruột đỏ mọng + hạt đen)
  vec2 wq = rot(-0.12) * (mq - vec2(0.0, -2.0));
  float r = length(wq * vec2(0.88, 1.18));
  float semi = max(r - 19.5, -wq.y - 2.0);
  // Vết cắn tròn trên chóp dưa hấu khi người dùng click ăn dưa
  if (bites > 0.5) semi = max(semi, -(length(wq - vec2(-2.0, -1.0)) - 7.5));
  if (bites > 1.5) semi = max(semi, -(length(wq - vec2(6.5, 1.5)) - 7.8));
  if (bites > 2.5) semi = max(semi, -(length(wq - vec2(-9.0, 2.5)) - 7.5));

  vec3 mCol = vec3(0.93, 0.27, 0.30);
  mCol = mix(mCol, vec3(0.95, 0.97, 0.88), ss(14.5, 16.5, r));
  mCol = mix(mCol, vec3(0.24, 0.58, 0.28), ss(16.8, 18.2, r));
  // Hạt dưa đen nhỏ
  float seedD = min(length(wq - vec2(-5.5, 5.5)), min(length(wq - vec2(1.5, 7.0)), length(wq - vec2(7.0, 4.5)))) - 1.15;
  if (r < 14.0 && semi < -1.0) mCol = mix(mCol, INK, ss(0.7, -0.3, seedD * s));
  strokeFill(col, semi * s, mCol * shade, 1.1, 1.0);

  // --- Quyển sách mở trang giấy phấp phới theo gió quạt (bên trái) ---
  vec2 bq = rot(0.32) * (q - vec2(-24.0, 12.0));
  float cover = sdBox(bq, vec2(20.5, 13.5), 2.0);
  strokeFill(col, cover * s, vec3(0.36, 0.46, 0.58) * shade, 1.1, 1.0);
  float pageFlutter = sin(uTime * (3.5 + uFanSpeed * 4.5)) * (1.2 + uFanSpeed * 1.8);
  float pagesL = sdBox(bq - vec2(-9.0, -1.2 - abs(pageFlutter) * 0.25), vec2(8.8, 11.5), 1.2);
  float pagesR = sdBox(bq - vec2(9.0, -1.2), vec2(8.8, 11.5), 1.2);
  strokeFill(col, min(pagesL, pagesR) * s, vec3(0.98, 0.97, 0.92) * shade, 0.95, 1.0);
  // Nếp gáy sách & dòng chữ mờ
  col = mix(col, INK, ss(1.0, 0.0, abs(bq.x) * s) * ss(0.0, -2.0, min(pagesL, pagesR) * s) * 0.65);
  float lines = abs(fract(bq.y * 0.32) - 0.5);
  if (min(pagesL, pagesR) < -2.5 && abs(bq.x) > 2.5) {
    col = mix(col, INK, ss(0.14, 0.02, lines) * 0.22);
  }
}

void main() {
  float W = uRes.x, H = uRes.y;
  vec2 p = vec2(gl_FragCoord.x, H * uScale - gl_FragCoord.y) / uScale;
  float s = min(W / 1536.0, H / 864.0);
  float nx = p.x / W, ny = p.y / H;
  float t = uTime;

  // Trường khoảng cách biểu tượng (<0 là bên trong cụm icon)
  float dIcon = mix(fieldS(uFA, p), fieldS(uFB, p), uMix);
  vec4 nz = N(p, 220.0 * s);
  vec4 nzFine = N(p, 64.0 * s);

  // Hệ số ánh sáng theo buổi (0: Trưa hè, 1: Hoàng hôn, 2: Đêm đom đóm)
  float isSunset = clamp(1.0 - abs(uDayMode - 1.0), 0.0, 1.0);
  float isNight  = ss(1.2, 1.9, uDayMode);
  vec3 tintEnv = mix(vec3(1.0, 1.0, 0.98), vec3(1.04, 0.86, 0.68), isSunset);
  tintEnv = mix(tintEnv, vec3(0.36, 0.44, 0.64), isNight);
  // Ánh đèn vàng ấm hắt ra từ hiên nhà vào ban đêm
  float lanternGlow = isNight * exp(-length((p - vec2(0.58 * W, 0.46 * H)) / vec2(0.38 * W, 0.34 * H)) * 1.45);
  vec3 porchShade = tintEnv + vec3(0.62, 0.44, 0.20) * lanternGlow;

  // =========================================================================
  // 1. BẦU TRỜI & RUỘNG BẬC THANG MÙA HẠ PHÍA XA (Góc trên bên trái)
  // =========================================================================
  vec3 skyTop = mix(vec3(0.55, 0.86, 0.84), vec3(0.95, 0.56, 0.38), isSunset);
  skyTop = mix(skyTop, vec3(0.08, 0.13, 0.25), isNight);
  vec3 skyHor = mix(vec3(0.95, 0.97, 0.78), vec3(1.0, 0.82, 0.54), isSunset);
  skyHor = mix(skyHor, vec3(0.16, 0.24, 0.38), isNight);
  vec3 col = mix(skyTop, skyHor, ss(0.0, 0.38, ny));

  // Sao lấp lánh nhẹ ở góc trời đêm
  if (isNight > 0.2 && ny < 0.28 && nx < 0.38) {
    vec2 sc = floor(p / (18.0 * s));
    float sh = hash2(sc + 7.3);
    if (sh > 0.92) {
      vec2 sp = (sc + 0.5) * 18.0 * s;
      float tw = 0.5 + 0.5 * sin(t * 2.6 + sh * 30.0);
      col += vec3(0.95, 0.96, 0.85) * ss(2.2 * s, 0.0, length(p - sp)) * tw * isNight;
    }
  }

  // Đồi xa & đường cong ruộng bậc thang ngập nắng
  float hill1 = H * (0.14 - 0.05 * sin(nx * 5.2 + 0.4));
  if (p.y > hill1 && nx < 0.50) {
    vec3 hCol = vec3(0.62, 0.82, 0.62) * tintEnv;
    // Các bậc ruộng uốn lượn óng ánh nước & lúa non
    float terrace = fract((p.y - hill1) / (15.0 * s) + 0.22 * sin(nx * 14.0 + p.y * 0.01));
    vec3 paddyWater = mix(vec3(0.86, 0.95, 0.68), vec3(0.58, 0.82, 0.52), ss(0.18, 0.78, terrace));
    hCol = mix(hCol, paddyWater * tintEnv, ss(0.16, 0.24, ny) * ss(0.46, 0.22, nx));
    float ridge = ss(0.08, 0.0, abs(terrace - 0.82)) * ss(0.45, 0.15, nx);
    hCol = mix(hCol, vec3(0.38, 0.58, 0.34) * tintEnv, ridge * 0.55);
    col = mix(col, hCol, ss(hill1 - 2.0, hill1 + 3.0, p.y));
  }

  // =========================================================================
  // 2. NỀN ĐẤT SÂN NẮNG, KHU VƯỜN QUÊ & LUỐNG VƯỜN ÔM BIỂU TƯỢNG DESKTOP
  // =========================================================================
  float groundY = H * (0.33 + 0.04 * sin(nx * 4.0));
  if (p.y > groundY) {
    // Màu đất cát sân quê nắng ấm có chất liệu màu nước
    vec3 soilLight = vec3(0.89, 0.79, 0.49) * porchShade;
    vec3 soilWarm  = vec3(0.76, 0.62, 0.35) * porchShade;
    vec3 yard = mix(soilLight, soilWarm, nz.r * 0.65 + 0.35 * ss(0.45, 1.0, ny));
    yard *= 0.94 + 0.12 * nzFine.g;

    // Thảm cỏ dại ven lối đi bên trái
    float leftGrassMask = ss(0.34, 0.16, nx + 0.08 * (nz.g - 0.5)) * ss(0.34, 0.44, ny);
    vec3 lushGreen = mix(vec3(0.44, 0.68, 0.26), vec3(0.62, 0.80, 0.30), nzFine.r) * porchShade;
    yard = mix(yard, lushGreen, leftGrassMask * 0.82);

    // --- LUỐNG VƯỜN THÍCH ỨNG ÔM CỤM BIỂU TƯỢNG DESKTOP (dIcon < 0) ---
    // Khi icon nằm ngoài hiên nhà: biến vùng quanh icon thành luống rau xanh viền đá cuội & hoa vàng!
    float gardenPatch = ss(16.0 * s, -6.0 * s, dIcon);
    if (gardenPatch > 0.005) {
      vec3 bedSoil = vec3(0.46, 0.35, 0.22) * porchShade;
      vec3 bedLeaf = mix(vec3(0.30, 0.55, 0.22), vec3(0.48, 0.72, 0.27), nzFine.b) * porchShade;
      // Lá tròn xếp lớp mềm mại ở rìa luống, giữa luống dịu màu để chữ tên icon sáng rõ
      float rimLeaf = ss(-26.0 * s, -2.0 * s, dIcon);
      float leafBump = sin(atan(nzFine.r - 0.5, nzFine.g - 0.5) * 5.0 + p.x * 0.08);
      vec3 bedCol = mix(mix(bedSoil, bedLeaf * 0.82, 0.65), bedLeaf * (1.0 + 0.08 * leafBump), rimLeaf);
      yard = mix(yard, bedCol, gardenPatch * 0.92);
      // Viền mực nhẹ + chấm hoa bí vàng li ti dọc mép luống
      float borderBand = ss(8.0 * s, 0.0, abs(dIcon - 2.0 * s));
      yard = mix(yard, INK, borderBand * 0.28);
      vec2 fc = floor(p / (26.0 * s));
      float fh = hash2(fc + 19.4);
      if (fh > 0.72 && abs(dIcon + 4.0 * s) < 12.0 * s) {
        vec2 fp = (fc + 0.5) * 26.0 * s;
        yard = mix(yard, vec3(1.0, 0.86, 0.26) * porchShade, ss(3.8 * s, 1.2 * s, length(p - fp)));
      }
    }

    col = mix(col, yard, ss(groundY - 2.0, groundY + 4.0, p.y));
  }

  // Bụi cây sáng nắng xếp lớp rậm rạp dọc hàng rào sau lưng hiên bên trái
  if (nx < 0.48 && ny > 0.14 && ny < 0.58) {
    for (int i = 0; i < 8; i++) {
      float fi = float(i);
      vec2 bc = vec2((0.03 + fi * 0.058) * W, (0.37 - fi * 0.017 + 0.018 * sin(fi * 2.3)) * H);
      bc.x += sin(t * 1.4 + fi) * 2.2 * s;
      float br = (56.0 + 13.0 * sin(fi * 3.7)) * s;
      float bd = length(p - bc) - br + (nzFine.r - 0.5) * 18.0 * s;
      if (bd < 2.0 * s) {
        vec3 bushC = mix(vec3(0.30, 0.58, 0.20), vec3(0.74, 0.90, 0.34), ss(br * 0.45, -br * 0.65, p.y - bc.y)) * porchShade;
        strokeFill(col, bd, bushC, 1.0 * s, 0.96);
      }
    }
  }

  // Giàn đậu leo chữ A bằng tre bên trái sân (giống khung hình mẫu)
  if (nx < 0.26 && ny > 0.33 && ny < 0.72) {
    float sway = sin(t * 1.6) * 2.0 * s;
    for (int k = 0; k < 2; k++) {
      float ox = (0.055 + float(k) * 0.095) * W;
      float oy = (0.56 + float(k) * 0.025) * H;
      vec2 top = vec2(ox + sway, oy - 130.0 * s);
      float pole1 = sdSeg(p, top, vec2(ox - 34.0 * s, oy + 46.0 * s)) - 2.4 * s;
      float pole2 = sdSeg(p, top, vec2(ox + 32.0 * s, oy + 52.0 * s)) - 2.4 * s;
      float bar   = sdSeg(p, top + vec2(-15.0 * s, 55.0 * s), top + vec2(15.0 * s, 58.0 * s)) - 1.8 * s;
      strokeFill(col, min(min(pole1, pole2), bar), vec3(0.74, 0.55, 0.28) * porchShade, 0.9 * s, 1.0);
      // Lá đậu xanh leo quanh cọc tre
      for (int j = 0; j < 5; j++) {
        float fj = float(j) / 4.0;
        vec2 lpL = mix(top + vec2(0.0, 18.0 * s), vec2(ox - 28.0 * s, oy + 32.0 * s), fj);
        vec2 lpR = mix(top + vec2(0.0, 22.0 * s), vec2(ox + 26.0 * s, oy + 36.0 * s), fj);
        float ld = min(length(p - lpL) - 11.0 * s, length(p - lpR) - 11.0 * s) + (nzFine.g - 0.5) * 6.0 * s;
        strokeFill(col, ld, mix(vec3(0.38, 0.68, 0.24), vec3(0.68, 0.84, 0.32), 1.0 - fj) * porchShade, 0.85 * s, 0.96);
      }
    }
  }

  // =========================================================================
  // 3. HẬU CẢNH TRONG NHÀ, VÁCH GỖ, CỬA XANH HÉ MỞ & DÂY PHƠI KHĂN TRẮNG
  // =========================================================================
  // Khu vực vách sau nhà (từ cột giữa lùi vào trong)
  vec2 wTL = vec2(0.455 * W, 0.13 * H);
  vec2 wTR = vec2(1.01 * W, 0.24 * H);
  vec2 wBR = vec2(1.01 * W, 0.555 * H);
  vec2 wBL = vec2(0.455 * W, 0.405 * H);
  float wallQuad = sdQuad(p, wTL, wTR, wBR, wBL);
  if (wallQuad < 2.0 * s) {
    // Vách vữa kem ấm bên trái (0.455..0.565)
    vec3 wCol = vec3(0.84, 0.81, 0.67) * porchShade * (0.94 + 0.08 * nzFine.r);
    // Gian phòng mát mẻ bên trong khung cửa mở (0.645..0.80)
    if (nx > 0.645 && nx < 0.805) {
      wCol = vec3(0.66, 0.75, 0.60) * porchShade;
      // Chiếc bàn trà gỗ thấp kèm chân bàn & miếng dưa nhỏ trong nhà
      float leg1 = sdBox(p - vec2(0.662 * W, 0.345 * H), vec2(3.5 * s, 14.0 * s), 1.0 * s);
      float leg2 = sdBox(p - vec2(0.712 * W, 0.355 * H), vec2(3.5 * s, 14.0 * s), 1.0 * s);
      strokeFill(wCol, min(leg1, leg2), vec3(0.50, 0.40, 0.24) * porchShade, 0.9 * s, 1.0);
      float tbl = sdQuad(p, vec2(0.655 * W, 0.295 * H), vec2(0.728 * W, 0.310 * H), vec2(0.715 * W, 0.345 * H), vec2(0.648 * W, 0.330 * H));
      strokeFill(wCol, tbl, vec3(0.64, 0.53, 0.32) * porchShade, 0.95 * s, 1.0);
      float miniMelon = max(length((p - vec2(0.680 * W, 0.305 * H)) / s) - 9.5, (p.y - 0.305 * H) / s);
      strokeFill(wCol, miniMelon * s, vec3(0.90, 0.34, 0.32) * porchShade, 0.8 * s, 1.0);
    }
    // Ô cửa trượt nan gỗ shoji bên phải (0.805..1.0)
    if (nx >= 0.805) {
      wCol = mix(vec3(0.62, 0.73, 0.72), vec3(0.46, 0.56, 0.54), ss(0.84, 0.96, nx)) * porchShade;
      float gridX = abs(fract((nx - 0.805) * 28.0) - 0.5);
      float gridY = abs(fract(ny * 22.0) - 0.5);
      if (nx < 0.865 && (gridX < 0.08 || gridY < 0.08)) wCol = vec3(0.48, 0.42, 0.30) * porchShade;
    }
    // Cánh cửa gỗ màu xanh da trời nhạt mở hé (0.562..0.656) + khoen nắm cửa tròn
    float doorD = sdQuad(p, vec2(0.562 * W, 0.145 * H), vec2(0.656 * W, 0.165 * H), vec2(0.656 * W, 0.432 * H), vec2(0.562 * W, 0.415 * H));
    if (doorD < 1.5 * s) {
      vec3 doorC = mix(vec3(0.68, 0.80, 0.82), vec3(0.57, 0.70, 0.73), ss(0.56, 0.65, nx)) * porchShade;
      strokeFill(wCol, doorD, doorC, 1.2 * s, 1.0);
      float ring = abs(length(p - vec2(0.583 * W, 0.215 * H)) - 6.5 * s) - 1.3 * s;
      wCol = mix(wCol, INK, ss(1.0 * s, -0.2 * s, ring) * 0.8);
    }
    // Chùm bắp ngô vàng & củ cải trắng treo khô bên vách gỗ (0.512, 0.235)
    vec2 cornP = (p - vec2(0.512 * W, 0.238 * H)) / s;
    float corn = min(
      min(sdEllipse(rot(0.24) * (cornP - vec2(-8.5, 2.0)), vec2(6.2, 20.0)),
          sdEllipse(rot(0.02) * (cornP - vec2(0.0, 5.0)), vec2(6.2, 21.0))),
      sdEllipse(rot(-0.22) * (cornP - vec2(8.0, 1.0)), vec2(5.8, 18.5))
    );
    strokeFill(wCol, corn * s, vec3(0.94, 0.81, 0.34) * porchShade, 1.0 * s, 1.0);
    float radD = sdBox(rot(-0.18) * (cornP - vec2(17.5, -2.0)), vec2(4.5, 16.0), 3.5);
    strokeFill(wCol, radD * s, vec3(0.90, 0.92, 0.88) * porchShade, 0.95 * s, 1.0);

    col = mix(col, wCol, ss(1.5 * s, -1.5 * s, wallQuad));
  }

  // =========================================================================
  // 4. SÀN HIÊN GỖ ẤM, BẬC THỀM ĐÁ, DÉP LÊ & ỦNG ĐI MƯA
  // =========================================================================
  // Bậc thềm đá xám trước hiên
  vec2 stTL = vec2(0.330 * W, 0.665 * H);
  vec2 stTR = vec2(0.648 * W, 0.772 * H);
  vec2 stBR = vec2(0.625 * W, 0.835 * H);
  vec2 stBL = vec2(0.328 * W, 0.722 * H);
  float stoneTop = sdQuad(p, vec2(0.355 * W, 0.635 * H), vec2(0.652 * W, 0.732 * H), stTR, stTL);
  float stoneFront = sdQuad(p, stTL, stTR, stBR, stBL);
  strokeFill(col, stoneTop, vec3(0.69, 0.67, 0.61) * porchShade * (0.94 + 0.1 * nzFine.g), 1.2 * s, 1.0);
  strokeFill(col, stoneFront, vec3(0.54, 0.51, 0.46) * porchShade * (0.92 + 0.12 * nzFine.r), 1.2 * s, 1.0);

  // Hốc tối gầm hiên + củ khoai/gà con vàng tròn ló ra ở góc trái gầm hiên!
  float underPorch = sdQuad(p, vec2(0.292 * W, 0.585 * H), vec2(0.765 * W, 0.745 * H), vec2(0.765 * W, 0.795 * H), vec2(0.292 * W, 0.638 * H));
  col = mix(col, vec3(0.22, 0.20, 0.17) * porchShade, ss(2.0 * s, -2.0 * s, underPorch) * 0.88);
  vec2 peep = (p - vec2(0.320 * W, 0.658 * H)) / s;
  float peepD = sdEllipse(rot(-0.2) * peep, vec2(11.0, 8.5));
  strokeFill(col, peepD * s, vec3(0.92, 0.76, 0.26) * porchShade, 1.0 * s, 1.0);

  // Đôi dép đen & dép sọc trên bậc đá + Đôi ủng đi mưa màu xanh cổ vịt
  vec2 sl1 = rot(0.48) * ((p - vec2(0.396 * W, 0.686 * H)) / s);
  vec2 sl2 = rot(-0.22) * ((p - vec2(0.442 * W, 0.698 * H)) / s);
  float slipD = min(sdEllipse(sl1, vec2(19.5, 8.8)), sdEllipse(sl2, vec2(19.5, 8.8)));
  strokeFill(col, slipD * s, vec3(0.28, 0.27, 0.26) * porchShade, 1.1 * s, 1.0);
  vec2 sd1 = rot(0.32) * ((p - vec2(0.553 * W, 0.744 * H)) / s);
  vec2 sd2 = rot(0.28) * ((p - vec2(0.588 * W, 0.738 * H)) / s);
  float sandD = min(sdEllipse(sd1, vec2(13.5, 6.8)), sdEllipse(sd2, vec2(13.5, 6.8)));
  strokeFill(col, sandD * s, vec3(0.86, 0.58, 0.44) * porchShade, 1.0 * s, 1.0);

  for (int b = 0; b < 2; b++) {
    vec2 bp = (p - vec2((0.642 + float(b) * 0.033) * W, (0.785 + float(b) * 0.012) * H)) / s;
    float shaft = sdBox(bp - vec2(0.0, -18.0), vec2(9.0, 22.0), 3.0);
    float foot  = sdEllipse(bp - vec2(-3.5, 4.0), vec2(12.5, 7.5));
    strokeFill(col, min(shaft, foot) * s, vec3(0.30, 0.56, 0.58) * porchShade, 1.15 * s, 1.0);
  }

  // --- Sàn hiên gỗ chính (nơi đàn Corgi nằm ngủ) & hiên phụ bên phải ---
  vec2 pBL = vec2(0.455 * W, 0.402 * H);
  vec2 pBR = vec2(0.805 * W, 0.505 * H);
  vec2 pFR = vec2(0.765 * W, 0.728 * H);
  vec2 pFL = vec2(0.292 * W, 0.562 * H);
  float deckMain = sdQuad(p, pBL, pBR, pFR, pFL);
  if (deckMain < 2.0 * s) {
    // Các tấm ván gỗ chạy dọc suốt chiều dài hiên nhà (song song mép hiên như bản Ghibli gốc)
    vec2 frontDir = normalize(pFR - pFL);
    vec2 frontNrm = vec2(frontDir.y, -frontDir.x);
    float plankCoord = dot(p - pFL, frontNrm) / (23.0 * s);
    float groove = ss(0.07, 0.0, abs(fract(plankCoord) - 0.5) - 0.43);
    float plankTone = 0.95 + 0.08 * hash2(vec2(floor(plankCoord), 3.1));
    float warmSun = ss(0.58, 0.34, nx) * (1.0 - isNight * 0.8);
    vec3 woodC = mix(vec3(0.43, 0.47, 0.33), vec3(0.80, 0.70, 0.36), warmSun) * porchShade * plankTone;
    woodC *= 0.93 + 0.12 * nzFine.r;
    woodC = mix(woodC, INK, groove * 0.58);

    // Nếu người dùng kéo biểu tượng Desktop đè lên sàn gỗ: trải nhẹ chiếu cói sáng dưới icon
    float matMask = ss(10.0 * s, -6.0 * s, dIcon);
    if (matMask > 0.01) {
      vec3 tatami = vec3(0.88, 0.83, 0.58) * porchShade;
      woodC = mix(woodC, tatami, matMask * 0.78);
    }
    strokeFill(col, deckMain, woodC, 1.35 * s, 1.0);
  }

  // Cạnh diềm gỗ mặt trước hiên chính
  float fascia = sdQuad(p, pFL, pFR, pFR + vec2(0.0, 26.0 * s), pFL + vec2(0.0, 24.0 * s));
  strokeFill(col, fascia, vec3(0.55, 0.50, 0.31) * porchShade, 1.25 * s, 1.0);

  // Sàn hiên phụ bên phải (nơi đặt mẹt rau & chú chim non màu xanh)
  vec2 sBL = vec2(0.805 * W, 0.485 * H);
  vec2 sBR = vec2(1.015 * W, 0.548 * H);
  vec2 sFR = vec2(1.015 * W, 0.745 * H);
  vec2 sFL = vec2(0.800 * W, 0.668 * H);
  float deckSide = sdQuad(p, sBL, sBR, sFR, sFL);
  if (deckSide < 2.0 * s) {
    vec2 sDir = normalize(sFR - sFL);
    vec2 sNrm = vec2(sDir.y, -sDir.x);
    vec3 sWood = vec3(0.38, 0.45, 0.36) * porchShade * (0.94 + 0.1 * nzFine.b);
    float groove2 = ss(0.07, 0.0, abs(fract(dot(p - sFL, sNrm) / (23.0 * s)) - 0.5) - 0.43);
    sWood = mix(sWood, INK, groove2 * 0.55);
    strokeFill(col, deckSide, sWood, 1.3 * s, 1.0);
  }
  float sideFascia = sdQuad(p, sFL, sFR, sFR + vec2(0.0, 28.0 * s), sFL + vec2(0.0, 28.0 * s));
  strokeFill(col, sideFascia, vec3(0.46, 0.45, 0.32) * porchShade, 1.2 * s, 1.0);

  // Mẹt tre tròn đựng đậu xanh + chú chim non xanh tròn xoe nhảy lích chích bên phải!
  vec2 basketP = (p - vec2(0.892 * W, 0.595 * H)) / s;
  if (dot(basketP, basketP) < 3600.0) {
    float bsk = sdEllipse(basketP, vec2(44.0, 27.0));
    vec3 bskC = mix(vec3(0.68, 0.56, 0.35), vec3(0.56, 0.45, 0.27), ss(34.0, 44.0, length(basketP * vec2(1.0, 1.5))));
    strokeFill(col, bsk * s, bskC * porchShade, 1.15 * s, 1.0);
    // Quả đậu xanh trong mẹt
    for (int k = 0; k < 5; k++) {
      float fk = float(k);
      vec2 pod = rot(0.3 * fk - 0.5) * (basketP - vec2(-12.0 + fk * 6.0, -4.0 + sin(fk * 2.0) * 5.0));
      strokeFill(col, sdEllipse(pod, vec2(16.0, 3.8)) * s, vec3(0.36, 0.58, 0.28) * porchShade, 0.8 * s, 1.0);
    }
  }
  // Chú chim non màu xanh da trời tròn xoe (nhảy lên khi bấm chuông gió!)
  float hopY = sin(uProps.w * PI) * 14.0 * s + max(0.0, sin(t * 2.8)) * 2.2 * s;
  vec2 birdP = (p - vec2(0.848 * W, 0.676 * H - hopY)) / s;
  if (dot(birdP, birdP) < 900.0) {
    float bd = min(sdEllipse(birdP, vec2(13.5, 10.5)), sdEllipse(birdP - vec2(-11.0, 4.0), vec2(6.0, 3.2)));
    strokeFill(col, bd * s, vec3(0.46, 0.72, 0.82) * porchShade, 1.05 * s, 1.0);
    col = mix(col, INK, ss(1.8, 0.5, length(birdP - vec2(5.5, -1.5))));
    col = mix(col, vec3(0.95, 0.72, 0.24), ss(2.8, 0.6, length(birdP - vec2(13.5, 1.0))));
  }

  // =========================================================================
  // 5. ĐÀN CORGI NGỦ TRƯA, ĐĨA DƯA HẤU + SÁCH MỞ & QUẠT BÀN QUAY ĐẦU
  // =========================================================================
  for (int i = 0; i < 4; i++) {
    if (float(i) >= uCorgiN) break;
    float localSun = mix(0.86, 1.05, ss(0.62, 0.36, uCorgiA[i].x / W));
    drawCorgi(col, p, uCorgiA[i], uCorgiB[i], s, localSun * (1.0 - isNight * 0.38));
  }

  drawMelonAndBook(col, p, vec2(0.702 * W, 0.592 * H), uProps.z, s, 0.92 * (1.0 - isNight * 0.35));
  drawDeskFan(col, p, uFanState, s, 0.94 * (1.0 - isNight * 0.35));

  // =========================================================================
  // 6. CỘT GỖ MỘC, CHUÔNG GIÓ ĐỒNG, KHĂN PHƠI & MÁI NGÓI ÂM DƯƠNG
  // =========================================================================
  // Hai cột gỗ trụ chính đỡ mái hiên (Cột trái x=0.273, Cột phải x=0.780)
  // Đế đá chân cột trái
  float plinthL = sdBox(p - vec2(0.273 * W, 0.622 * H), vec2(26.0 * s, 18.0 * s), 3.0 * s);
  strokeFill(col, plinthL, vec3(0.58, 0.56, 0.52) * porchShade, 1.2 * s, 1.0);

  float colL = sdBox(p - vec2(0.273 * W, 0.345 * H), vec2(20.0 * s, 265.0 * s), 2.0 * s);
  vec3 cWoodL = mix(vec3(0.76, 0.67, 0.32), vec3(0.56, 0.50, 0.26), ss(0.260, 0.285, nx)) * porchShade;
  strokeFill(col, colL, cWoodL * (0.94 + 0.1 * nzFine.g), 1.35 * s, 1.0);

  // Cột gỗ bên phải (che một phần tiền cảnh tạo chiều sâu đúng khung hình gốc)
  float plinthR = sdBox(p - vec2(0.780 * W, 0.782 * H), vec2(28.0 * s, 16.0 * s), 3.0 * s);
  strokeFill(col, plinthR, vec3(0.54, 0.52, 0.47) * porchShade, 1.2 * s, 1.0);
  float colR = sdBox(p - vec2(0.780 * W, 0.455 * H), vec2(23.0 * s, 315.0 * s), 2.0 * s);
  vec3 cWoodR = mix(vec3(0.54, 0.55, 0.34), vec3(0.38, 0.42, 0.28), ss(0.766, 0.794, nx)) * porchShade;
  strokeFill(col, colR, cWoodR * (0.94 + 0.1 * nzFine.r), 1.35 * s, 1.0);

  // Chuông gió đồng treo dưới hiên trái (x=0.346, y=0.17) + thẻ gỗ hình khúc xương Corgi!
  vec2 chimeTop = vec2(0.346 * W, 0.095 * H);
  if (abs(p.x - chimeTop.x) < 48.0 * s && p.y > chimeTop.y && p.y < chimeTop.y + 150.0 * s) {
    float ca = uProps.x;
    vec2 bellC = chimeTop + rot(ca * 0.5) * vec2(0.0, 52.0 * s);
    vec2 tagC  = bellC + rot(ca * 1.35) * vec2(0.0, 42.0 * s);
    col = mix(col, INK, ss(1.2 * s, 0.0, sdSeg(p, chimeTop, bellC) - 0.6 * s));
    col = mix(col, INK, ss(1.2 * s, 0.0, sdSeg(p, bellC, tagC) - 0.6 * s));
    float bellD = sdBox(rot(ca * 0.6) * (p - bellC), vec2(10.5 * s, 13.5 * s), 5.0 * s);
    strokeFill(col, bellD, vec3(0.74, 0.62, 0.34) * porchShade, 1.05 * s, 1.0);
    // Thẻ đón gió hình khúc xương nhỏ
    vec2 bq = rot(ca * 1.4) * (p - tagC) / s;
    float bone = min(sdBox(bq, vec2(9.5, 3.0), 1.5),
                 min(length(bq - vec2(-9.5, 0.0)) - 4.2, length(bq - vec2(9.5, 0.0)) - 4.2));
    strokeFill(col, bone * s, vec3(0.82, 0.68, 0.42) * porchShade, 0.95 * s, 1.0);
  }

  // Hai chiếc khăn bông trắng phơi trên dây đung đưa ngoài hiên phải
  if (nx > 0.80 && ny > 0.22 && ny < 0.52) {
    vec2 lA = vec2(0.802 * W, 0.258 * H), lB = vec2(1.01 * W, 0.296 * H);
    col = mix(col, INK, ss(1.2 * s, 0.0, sdSeg(p, lA, lB) - 0.7 * s) * 0.8);
    float breeze = (0.35 + uFanSpeed * 0.45) * sin(t * 2.4 + p.y * 0.03);
    for (int k = 0; k < 2; k++) {
      float tx0 = (0.865 + float(k) * 0.078) * W;
      float ty0 = mix(lA.y, lB.y, (tx0 - lA.x) / (lB.x - lA.x));
      vec2 tp = p - vec2(tx0 + breeze * (p.y - ty0) * 0.14 - (p.y - ty0) * 0.12, ty0 + (62.0 + float(k) * 14.0) * s);
      float td = sdBox(tp, vec2((26.0 + float(k) * 5.0) * s, (58.0 + float(k) * 14.0) * s), 2.0 * s);
      vec3 twCol = mix(vec3(0.84, 0.89, 0.92), vec3(0.97, 0.98, 0.99), ss(20.0 * s, -20.0 * s, tp.x)) * porchShade;
      strokeFill(col, td, twCol, 1.1 * s, 1.0);
    }
  }

  // --- Mái ngói âm dương truyền thống phủ dọc cạnh trên ---
  float eaveY = H * (0.048 + 0.182 * (nx - 0.12));
  if (nx > 0.125 && p.y < eaveY + 26.0 * s) {
    // Chu kỳ viên ngói ống lượn sóng dọc diềm mái
    float tileIdx = (p.x - 0.13 * W) / (34.0 * s) + (eaveY - p.y) * 0.012 / s;
    float tileFrac = fract(tileIdx);
    float scallop = sin(tileFrac * PI) * 8.5 * s;
    float roofEdge = p.y - (eaveY + scallop);
    if (roofEdge < 2.0 * s) {
      // Nắng vàng chiếu ấm nửa mái trái, bóng tím lam mát ở nửa mái phải
      vec3 tileWarm = vec3(0.92, 0.84, 0.64) * porchShade;
      vec3 tileCool = vec3(0.58, 0.60, 0.64) * porchShade;
      vec3 rCol = mix(tileWarm, tileCool, ss(0.32, 0.78, nx));
      // Khối tròn từng ống ngói & hàng ngói xếp lớp
      float ridgeShade = sin(tileFrac * PI);
      float rowStep = fract((eaveY - p.y) / (28.0 * s));
      rCol *= 0.86 + 0.20 * ridgeShade;
      rCol = mix(rCol, INK, ss(0.08, 0.0, abs(tileFrac - 0.02)) * 0.42);
      rCol = mix(rCol, INK, ss(0.07, 0.0, rowStep) * 0.25);
      // Đầu ngói tròn dọc viền mái hiên
      if (p.y > eaveY - 8.0 * s) {
        rCol *= 0.82;
      }
      strokeFill(col, roofEdge, rCol, 1.35 * s, 1.0);
    }
  }

  // =========================================================================
  // 7. TIỀN CẢNH: LUỐNG BÍ XANH TRÁI, NONG PHƠI ỚT ĐỎ & CỤM CHUM SÀNH PHẢI
  // =========================================================================
  // Luống lá bí xanh mướt xếp cụm rậm rạp góc dưới bên trái
  if (nx < 0.39 && ny > 0.68) {
    float patchShadow = sdEllipse((p - vec2(0.17 * W, 0.89 * H)) / s, vec2(255.0, 115.0));
    col = mix(col, vec3(0.28, 0.38, 0.16) * porchShade, ss(12.0, -24.0, patchShadow) * 0.72);
    for (int i = 0; i < 16; i++) {
      float fi = float(i);
      vec2 lc = vec2((0.03 + mod(fi * 0.056, 0.30)) * W, (0.76 + floor(fi / 5.0) * 0.068 + 0.018 * sin(fi * 1.7)) * H);
      lc.x += sin(t * 1.5 + fi) * 1.8 * s;
      float lr = (38.0 + 10.0 * sin(fi * 2.3)) * s;
      float ld = length(p - lc) - lr + (nzFine.r - 0.5) * 14.0 * s;
      vec3 lCol = mix(vec3(0.32, 0.62, 0.22), vec3(0.76, 0.89, 0.31), ss(lr, -lr, p.y - lc.y)) * porchShade;
      strokeFill(col, ld, lCol, 1.05 * s, 1.0);
    }
  }

  // Chiếu cói phơi ớt đỏ rực dưới nắng giữa sân trước (x=0.44..0.65, y=0.86..0.99)
  vec2 mTL = vec2(0.490 * W, 0.875 * H);
  vec2 mTR = vec2(0.648 * W, 0.945 * H);
  vec2 mBR = vec2(0.610 * W, 1.015 * H);
  vec2 mBL = vec2(0.435 * W, 0.985 * H);
  float chiliMat = sdQuad(p, mTL, mTR, mBR, mBL);
  if (chiliMat < 2.0 * s) {
    vec3 matC = vec3(0.95, 0.86, 0.48) * porchShade * (0.94 + 0.1 * nzFine.g);
    // Những quả ớt đỏ phơi trên chiếu
    vec2 cc = floor(p / (13.0 * s));
    float ch = hash2(cc + 4.2);
    if (ch > 0.42 && chiliMat < -6.0 * s) {
      vec2 cp = (p - (cc + 0.5) * 13.0 * s) / s;
      cp = rot(ch * 6.28) * cp;
      float cd = sdEllipse(cp, vec2(4.8, 1.8));
      matC = mix(matC, vec3(0.88, 0.25, 0.14) * porchShade, ss(0.9, -0.4, cd));
    }
    strokeFill(col, chiliMat, matC, 1.15 * s, 1.0);
  }

  // Cụm chum sành / vại gốm bóng nâu ấm ở góc dưới bên phải
  if (nx > 0.66 && ny > 0.76) {
    vec3 jarBase = vec3(0.58, 0.40, 0.20) * porchShade;
    vec3 jarRim  = vec3(0.76, 0.56, 0.30) * porchShade;
    vec2 jars[4] = vec2[4](
      vec2(0.835 * W, 0.915 * H),
      vec2(0.905 * W, 0.885 * H),
      vec2(0.730 * W, 0.955 * H),
      vec2(0.962 * W, 0.945 * H)
    );
    float jRad[4] = float[4](34.0, 42.0, 38.0, 48.0);
    for (int j = 0; j < 4; j++) {
      vec2 jp = (p - jars[j]) / s;
      float jr = jRad[j];
      float jd = sdEllipse(jp, vec2(jr, jr * 1.08));
      vec3 jc = mix(jarBase, jarRim, ss(jr * 0.5, -jr * 0.7, jp.x + jp.y * 0.6));
      strokeFill(col, jd * s, jc, 1.3 * s, 1.0);
      // Miệng chum / nắp gỗ
      float mouth = sdEllipse(jp - vec2(0.0, -jr * 0.78), vec2(jr * 0.66, jr * 0.28));
      vec3 topC = j == 0 ? vec3(0.32, 0.62, 0.28) * porchShade : vec3(0.32, 0.22, 0.12) * porchShade;
      strokeFill(col, mouth * s, topC, 1.1 * s, 1.0);
    }
  }

  // =========================================================================
  // 8. BƯỚM HÈ / ĐOM ĐÓM, HIỆU ỨNG TƯƠNG TÁC CLICK & TIA NẮNG XIÊN GHIBLI
  // =========================================================================
  for (int i = 0; i < 10; i++) {
    if (float(i) >= uBugN) break;
    vec4 bg = uBugs[i];
    if (isNight > 0.5) {
      // Đom đóm lập lòe vàng lục ấm áp vào đêm hè
      float d = length(p - bg.xy);
      float pulse = 0.45 + 0.55 * sin(bg.w);
      col += vec3(0.78, 0.96, 0.36) * exp(-d / (14.0 * s)) * pulse * 0.85;
      col = mix(col, vec3(1.0, 1.0, 0.82), ss(2.8 * s, 0.6 * s, d) * pulse);
    } else {
      // Bướm trắng / vàng dập dờn trong nắng hè
      vec2 bq = (p - bg.xy) / s;
      if (dot(bq, bq) > 260.0) continue;
      bq.x *= bg.z >= 0.0 ? 1.0 : -1.0;
      float flap = 0.30 + 0.70 * abs(sin(bg.w));
      float w1 = sdEllipse(vec2(bq.x - 2.6, bq.y / flap + 1.8), vec2(5.4, 4.4));
      float w2 = sdEllipse(vec2(bq.x + 2.2, bq.y / flap - 1.5), vec2(4.2, 3.5));
      vec3 bfCol = mod(float(i), 2.0) < 0.5 ? vec3(1.0, 0.95, 0.72) : vec3(1.0, 0.84, 0.36);
      strokeFill(col, min(w1, w2) * s, bfCol, 0.95, 0.96);
    }
  }

  // Các hiệu ứng phản hồi khi người dùng click chuột
  for (int i = 0; i < 6; i++) {
    vec4 fx = uFx[i];
    if (fx.z < 0.0) continue;
    float age = fx.z;
    int kind = int(floor(fx.w + 0.5));
    if (kind == 0) {
      // Trái tim hồng bay lên khi gãi bụng Corgi
      float fade = 1.0 - ss(0.8, 1.6, age);
      for (int h = 0; h < 2; h++) {
        float fh = float(h);
        vec2 hp = fx.xy + vec2(sin(age * 4.5 + fh * 2.5) * 14.0 * s + (fh - 0.5) * 18.0 * s, -age * (38.0 + fh * 14.0) * s);
        float hd = sdHeart((p - hp) / ((10.5 - fh * 2.5) * s));
        strokeFill(col, hd * 9.0 * s, vec3(0.98, 0.42, 0.54), 0.85, fade);
      }
    } else if (kind == 1) {
      // Làn gió mát tỏa ra khi bấm đổi tốc độ quạt
      float fade = 1.0 - ss(0.3, 1.1, age);
      for (int w = 0; w < 3; w++) {
        float fw = float(w) - 1.0;
        vec2 a0 = fx.xy + vec2(-18.0 * s - age * 55.0 * s, fw * 16.0 * s + age * 14.0 * s);
        vec2 a1 = a0 + vec2(-32.0 * s, 9.0 * s);
        float wd = sdSeg(p, a0, a1) - 1.4 * s;
        col = mix(col, vec3(0.88, 0.98, 1.0), ss(1.5 * s, 0.0, wd) * fade * 0.72);
      }
    } else if (kind == 2) {
      // Vòng sóng âm thanh vàng óng lan tỏa từ chuông gió đồng
      float fade = 1.0 - ss(0.4, 1.5, age);
      float rad = (12.0 + age * 68.0) * s;
      float ring = abs(length(p - fx.xy) - rad) - 1.4 * s;
      col += vec3(1.0, 0.92, 0.55) * ss(2.2 * s, 0.0, ring) * fade * 0.65;
    } else if (kind == 3) {
      // Tia lấp lánh mát lạnh khi cắn miếng dưa hấu
      float fade = 1.0 - ss(0.25, 0.95, age);
      for (int k = 0; k < 5; k++) {
        float ang = float(k) * 1.256 + 0.4;
        vec2 sp = fx.xy + vec2(cos(ang), sin(ang)) * (8.0 + age * 38.0) * s;
        col = mix(col, vec3(1.0, 0.40, 0.45), ss(3.2 * s, 0.5 * s, length(p - sp)) * fade);
      }
    } else if (kind == 4) {
      // Vòng sáng & lá xanh bay nhẹ khi thả bướm ngoài vườn
      float fade = 1.0 - ss(0.3, 1.2, age);
      float ring = abs(length(p - fx.xy) - (8.0 + age * 36.0) * s) - 1.2 * s;
      col += vec3(0.85, 0.96, 0.52) * ss(2.0 * s, 0.0, ring) * fade * 0.55;
    }
  }

  // --- Tia nắng hè rọi xiên từ góc trên trái & hạt nắng lơ lửng (God-rays) ---
  if (uSunbeams > 0.01) {
    float rayCoord = (nx * 0.82 - ny * 0.48) * 9.0;
    float rays = 0.5 + 0.5 * sin(rayCoord + sin(rayCoord * 2.3 + t * 0.25) * 0.6);
    float rayMask = pow(rays, 3.2) * ss(0.88, 0.05, nx * 0.65 + ny * 0.45);
    vec3 rayCol = mix(vec3(1.0, 0.97, 0.68), vec3(1.0, 0.72, 0.40), isSunset);
    rayCol = mix(rayCol, vec3(0.45, 0.60, 0.90), isNight * 0.85);
    col += rayCol * rayMask * uSunbeams * (0.24 - 0.14 * isNight);

    // Ánh sáng ấm mờ ở góc trên trái như khung hình mẫu
    float sunCorner = exp(-length(vec2(nx, ny) - vec2(0.14, 0.22)) * 2.8);
    col += rayCol * sunCorner * uSunbeams * (0.20 - 0.15 * isNight);
  }

  if (uDebug > 0.5) {
    col = mix(col, vec3(0.95, 0.2, 0.2), ss(2.0, 0.0, abs(dIcon)));
  }

  o = vec4(clamp(col, 0.0, 1.0), 1.0);
}
