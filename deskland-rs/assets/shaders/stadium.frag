#version 300 es
precision highp float;

uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uDebug;

uniform vec4 uPitch;        // x0, y0, x1, y1 (tọa độ CSS mặt sân cỏ chính)
uniform float uStripes;     // độ tương phản sọc cỏ (0..1)
uniform float uLighting;    // 0 = Ban ngày, 1 = Nắng chiều, 2 = Đèn pha đêm
uniform float uPlayerN;     // tổng số cầu thủ trên sân (6..22)
uniform vec4 uPlPos[22];    // x, y, dir (rad), stride_phase
uniform vec4 uPlMeta[22];   // kit (0=Đỏ, 1=Xanh, 2=GK Đỏ, 3=GK Xanh), speed (0..1), ring_glow, hair_seed
uniform vec4 uBall;         // x, y, z (độ cao), spin
uniform vec4 uBallTrail;    // prev_x, prev_y, trail_alpha, kick_age
uniform vec4 uTarget;       // click_x, click_y, age, active
uniform vec4 uGoalFx;       // goal_x, goal_y, age, side (-1=trái, +1=phải, 0=tắt)

out vec4 o;

const float PI = 3.14159265;
const vec3 OUTLINE = vec3(0.11, 0.14, 0.12);

float hash2(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

float ss(float a, float b, float x) {
  float t = clamp((x - a) / (b - a), 0.0, 1.0);
  return t * t * (3.0 - 2.0 * t);
}

mat2 rot(float a) {
  float c = cos(a), s = sin(a);
  return mat2(c, s, -s, c);
}

float sdBox(vec2 p, vec2 b, float r) {
  vec2 d = abs(p) - b + r;
  return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0) - r;
}

float sdSeg(vec2 p, vec2 a, vec2 b) {
  vec2 pa = p - a, ba = b - a;
  return length(pa - ba * clamp(dot(pa, ba) / max(dot(ba, ba), 1e-4), 0.0, 1.0));
}

float sdEllipse(vec2 p, vec2 r) {
  return (length(p / r) - 1.0) * min(r.x, r.y);
}

// Vẽ cầu môn 3D nhìn từ trên cao (cột dọc, xà ngang, lưới mắt cáo rung khi ghi bàn)
void drawGoal(inout vec3 col, vec2 p, vec4 pitch, float side, float s, vec4 goalFx) {
  float x0 = pitch.x, y0 = pitch.y, x1 = pitch.z, y1 = pitch.w;
  float ph = y1 - y0;
  float cy = 0.5 * (y0 + y1);
  float gh = ph * 0.22;
  float gd = 28.0 * s;

  float gx = side < 0.0 ? x0 - gd * 0.5 : x1 + gd * 0.5;
  vec2 bCenter = vec2(gx, cy);
  vec2 halfSz = vec2(gd * 0.5, gh * 0.5);
  vec2 dp = p - bCenter;
  if (abs(dp.x) > halfSz.x + 18.0 * s || abs(dp.y) > halfSz.y + 18.0 * s) return;

  // Độ rung lưới khi vừa có bàn thắng vào cầu môn bên này
  float netWave = 0.0;
  if (abs(goalFx.w - side) < 0.2 && goalFx.z < 2.0) {
    float env = exp(-goalFx.z * 2.4);
    netWave = sin(goalFx.z * 24.0 - length(p - goalFx.xy) * 0.12) * 3.8 * s * env;
  }

  // Bóng đổ của khung thành xuống thảm cỏ biên
  vec2 shOff = vec2(side < 0.0 ? -4.5 : 4.5, 5.5) * s;
  float dShadow = sdBox(p - (bCenter + shOff), halfSz, 2.0 * s);
  col = mix(col, col * 0.72, ss(3.0 * s, -2.0 * s, dShadow) * 0.55);

  // Vùng lòng cầu môn + mắt lưới đan chéo
  vec2 wp = dp;
  float depthT = clamp((side < 0.0 ? (x0 - p.x) : (p.x - x1)) / max(gd, 1.0), 0.0, 1.0);
  wp.x += side * netWave * depthT;
  wp.y += netWave * 0.45 * depthT;

  // Khung sau hơi thu nhỏ nhẹ tạo chiều sâu 3D nhìn từ trên xuống
  float taper = 1.0 - 0.07 * depthT;
  float inNet = max(abs(wp.x) - halfSz.x, abs(wp.y) - halfSz.y * taper);
  if (inNet < 1.5 * s) {
    float aBox = ss(1.2 * s, -0.8 * s, inNet);
    col = mix(col, col * 0.88, aBox * 0.45);
    float cell = 5.2 * s;
    float m1 = abs(fract((wp.x + wp.y) / cell + 0.5) - 0.5) * cell;
    float m2 = abs(fract((wp.x - wp.y) / cell + 0.5) - 0.5) * cell;
    float mesh = 1.0 - ss(0.35 * s, 1.15 * s, min(m1, m2));
    col = mix(col, vec3(0.93, 0.96, 0.98), mesh * aBox * 0.52);
  }

  // Khung sắt sau cầu môn (mảnh, xám bạc)
  float backX = side < 0.0 ? x0 - gd + netWave * side * 0.6 : x1 + gd + netWave * side * 0.6;
  vec2 topIn = vec2(side < 0.0 ? x0 : x1, cy - halfSz.y);
  vec2 botIn = vec2(side < 0.0 ? x0 : x1, cy + halfSz.y);
  vec2 topBack = vec2(backX, cy - halfSz.y * 0.93);
  vec2 botBack = vec2(backX, cy + halfSz.y * 0.93);
  float dBack = min(sdSeg(p, topIn, topBack), min(sdSeg(p, botIn, botBack), sdSeg(p, topBack, botBack)));
  col = mix(col, vec3(0.82, 0.86, 0.90), ss(1.5 * s, 0.4 * s, dBack) * 0.85);

  // Xà ngang & 2 cột dọc chính màu trắng sáng
  float dCross = sdSeg(p, topIn, botIn);
  col = mix(col, vec3(0.15, 0.20, 0.16), ss(3.1 * s, 1.6 * s, dCross) * 0.35);
  col = mix(col, vec3(0.99, 0.995, 1.0), ss(2.3 * s, 0.8 * s, dCross));
  float dPosts = min(length(p - topIn), length(p - botIn)) - 3.1 * s;
  col = mix(col, OUTLINE, ss(1.0 * s, -0.4 * s, dPosts - 0.8 * s) * 0.65);
  col = mix(col, vec3(1.0), ss(0.8 * s, -0.8 * s, dPosts));
}

// Vẽ cờ góc sân tung bay trong gió
void drawCornerFlag(inout vec3 col, vec2 p, vec2 corner, float dirX, float dirY, float s, float t) {
  vec2 dp = p - corner;
  if (abs(dp.x) > 22.0 * s || abs(dp.y) > 22.0 * s) return;
  vec2 tip = corner + vec2(dirX * 5.0 * s, -8.5 * s);
  float dPole = sdSeg(p, corner, tip);
  col = mix(col, vec3(0.96, 0.97, 0.99), ss(1.2 * s, 0.3 * s, dPole));
  float wave = sin(t * 5.5 + corner.x * 0.03 + corner.y * 0.02) * 1.8 * s;
  vec2 fEnd = tip + vec2(dirX * 8.5 * s, wave + dirY * 1.2 * s);
  float dFlag = sdSeg(p, tip + vec2(0.0, 1.5 * s), fEnd);
  vec3 flagCol = (dirX * dirY > 0.0) ? vec3(0.98, 0.78, 0.14) : vec3(0.95, 0.26, 0.24);
  col = mix(col, flagCol, ss(2.8 * s, 0.8 * s, dFlag));
}

// Vẽ cầu thủ nhìn từ trên cao (chân chạy động, áo đấu, viền cổ, đầu & tóc, bóng đổ)
void drawPlayer(inout vec3 col, vec2 p, vec4 pos, vec4 meta, float s, float lighting, vec4 pitch) {
  float ps = s * 1.28;
  vec2 dp = p - pos.xy;
  float bound = 22.0 * ps;
  if (abs(dp.x) > bound || abs(dp.y) > bound) return;

  float dir = pos.z;
  float stride = pos.w;
  float kit = meta.x;
  float spd = meta.y;
  float glow = meta.z;
  float hSeed = meta.w;

  // 1. Vòng định vị chiến thuật dưới chân (khi đang giữ bóng hoặc bứt tốc đuổi theo click)
  if (glow > 0.02) {
    float rDist = abs(length(dp) - 10.8 * ps);
    vec3 ringC = (kit < 0.5 || (kit > 1.5 && kit < 2.5))
      ? vec3(1.0, 0.45, 0.38)
      : vec3(0.38, 0.78, 1.0);
    col = mix(col, ringC, ss(1.6 * ps, 0.25 * ps, rDist) * min(0.78, glow * 0.68));
  }

  // 2. Bóng đổ trên mặt cỏ (ban đêm có 4 bóng chéo dưới ánh đèn pha SVĐ!)
  if (lighting > 1.5) {
    vec2 pc = 0.5 * (pitch.xy + pitch.zw);
    vec2 fromCenter = clamp((pos.xy - pc) / max(pitch.zw - pitch.xy, vec2(1.0)), -0.6, 0.6);
    vec2 o1 = vec2( 4.2,  4.2) * ps + fromCenter * 5.0 * ps;
    vec2 o2 = vec2(-4.2,  4.2) * ps + fromCenter * 5.0 * ps;
    vec2 o3 = vec2( 4.2, -3.6) * ps + fromCenter * 5.0 * ps;
    vec2 o4 = vec2(-4.2, -3.6) * ps + fromCenter * 5.0 * ps;
    float sh = ss(6.8 * ps, 1.2 * ps, length(dp - o1))
             + ss(6.8 * ps, 1.2 * ps, length(dp - o2))
             + ss(6.8 * ps, 1.2 * ps, length(dp - o3))
             + ss(6.8 * ps, 1.2 * ps, length(dp - o4));
    col *= 1.0 - min(0.45, sh * 0.15);
  } else {
    vec2 shOff = lighting > 0.5 ? vec2(6.5, 5.2) * ps : vec2(2.8, 3.6) * ps;
    float dSh = sdEllipse(dp - shOff, vec2(6.8 * ps, 5.8 * ps));
    col *= 1.0 - ss(2.5 * ps, -2.5 * ps, dSh) * (lighting > 0.5 ? 0.36 : 0.28);
  }

  // Tọa độ cục bộ của cầu thủ: +x hướng mặt chạy tới, y ngang hai vai
  vec2 q = (rot(-dir) * dp) / ps;

  // Màu trang phục theo đội & thủ môn
  vec3 jersey = vec3(0.898, 0.224, 0.208); // Đội Đỏ (#e53935)
  vec3 trimC  = vec3(0.98, 0.98, 1.0);
  vec3 sockC  = vec3(0.95, 0.25, 0.24);
  if (kit > 2.5) {
    // GK Đội Xanh: Xanh ngọc Neon
    jersey = vec3(0.05, 0.78, 0.64);
    trimC  = vec3(0.10, 0.22, 0.24);
    sockC  = vec3(0.08, 0.72, 0.58);
  } else if (kit > 1.5) {
    // GK Đội Đỏ: Vàng cam rực
    jersey = vec3(1.0, 0.72, 0.08);
    trimC  = vec3(0.16, 0.16, 0.18);
    sockC  = vec3(0.96, 0.68, 0.08);
  } else if (kit > 0.5) {
    // Đội Xanh Dương (#1e88e5)
    jersey = vec3(0.118, 0.533, 0.898);
    trimC  = vec3(0.98, 0.99, 1.0);
    sockC  = vec3(0.94, 0.96, 0.99);
  }

  // 3. Đôi chân & giày đinh sải bước theo tốc độ chạy
  float swing = sin(stride) * (1.2 + 5.2 * spd);
  vec2 footL = vec2( swing, -3.2);
  vec2 footR = vec2(-swing,  3.2);
  float dFootL = sdEllipse(q - footL, vec2(3.3, 1.85));
  float dFootR = sdEllipse(q - footR, vec2(3.3, 1.85));
  float dFeet = min(dFootL, dFootR);
  if (dFeet < 2.2) {
    col = mix(col, OUTLINE, ss(1.3, 0.2, dFeet));
    vec3 bootC = vec3(0.16, 0.18, 0.22);
    col = mix(col, mix(sockC, bootC, ss(-0.5, 1.8, max(q.x - footL.x, q.x - footR.x))), ss(0.5, -0.6, dFeet));
  }

  // 4. Hai cánh tay đánh nhịp ngược chiều chân
  float armSwing = -swing * 0.68;
  float dArmL = sdEllipse(q - vec2(armSwing, -7.2), vec2(2.6, 1.6));
  float dArmR = sdEllipse(q - vec2(-armSwing, 7.2), vec2(2.6, 1.6));
  float dArms = min(dArmL, dArmR);
  if (dArms < 2.0) {
    col = mix(col, OUTLINE, ss(1.2, 0.2, dArms));
    col = mix(col, vec3(0.96, 0.80, 0.67), ss(0.4, -0.6, dArms));
  }

  // 5. Thân & Hai vai áo đấu
  float dBody = sdBox(q - vec2(-0.3, 0.0), vec2(4.4, 7.2), 3.6);
  if (dBody < 2.2) {
    col = mix(col, OUTLINE, ss(1.35, 0.15, dBody));
    vec3 bodyCol = jersey;
    // Sọc cầu vai trắng / viền tay áo thể thao
    float stripe = ss(4.9, 5.5, abs(q.y)) * (1.0 - ss(6.4, 7.0, abs(q.y)));
    bodyCol = mix(bodyCol, trimC, stripe * 0.9);
    // Ánh sáng khối vai trên
    bodyCol = mix(bodyCol, min(vec3(1.0), bodyCol * 1.22 + 0.06), ss(2.5, -2.5, q.x - 1.2) * 0.45);
    col = mix(col, bodyCol, ss(0.45, -0.55, dBody));
  }

  // 6. Đầu & Mái tóc nhìn từ đỉnh (có viền trán/mũi phía +x để rõ hướng nhìn)
  vec2 hq = q - vec2(0.85, 0.0);
  float dHead = length(hq) - 4.15;
  if (dHead < 2.0) {
    col = mix(col, OUTLINE, ss(1.25, 0.15, dHead));
    vec3 skin = vec3(0.97, 0.81, 0.68);
    vec3 hair = hSeed < 0.35 ? vec3(0.13, 0.11, 0.10)
              : hSeed < 0.68 ? vec3(0.28, 0.18, 0.11)
              : hSeed < 0.88 ? vec3(0.46, 0.29, 0.15)
                             : vec3(0.82, 0.66, 0.30);
    // Phía trước mặt (+x) lộ nhẹ viền trán, phía sau phủ tóc
    float hairMask = ss(2.4, 0.6, hq.x);
    vec3 headCol = mix(skin, hair, hairMask);
    headCol += vec3(0.09) * ss(2.6, 0.0, length(hq - vec2(-0.8, -1.0)));
    col = mix(col, headCol, ss(0.45, -0.55, dHead));
  }
}

// Vẽ quả bóng đá 3D (bóng đổ tách rời khi nảy cao z, họa tiết ngũ giác xoay tròn)
void drawBall(inout vec3 col, vec2 p, vec4 ball, vec4 trail, float s, float lighting) {
  // Vệt gió đường chuyền / cú sút nhanh
  if (trail.z > 0.02) {
    vec2 a = trail.xy;
    vec2 b = ball.xy;
    vec2 ab = b - a;
    float lenAB = length(ab);
    if (lenAB > 3.0 * s) {
      vec2 pa = p - a;
      float h = clamp(dot(pa, ab) / max(dot(ab, ab), 1e-4), 0.0, 1.0);
      float dTr = length(pa - ab * h);
      float wTr = mix(1.0, 4.2, h) * s;
      float aTr = ss(wTr, 0.0, dTr) * h * h * trail.z * 0.42;
      col = mix(col, vec3(0.96, 0.99, 1.0), aTr);
    }
  }

  // Bụi cỏ / vôi trắng bốc nhẹ ngay điểm vừa sút bóng
  if (trail.w > 0.0 && trail.w < 0.55) {
    vec2 kp = p - trail.xy;
    if (abs(kp.x) < 24.0 * s && abs(kp.y) < 24.0 * s) {
      float kAge = trail.w / 0.55;
      float rPuff = (5.0 + kAge * 14.0) * s;
      float dPuff = abs(length(kp) - rPuff);
      col = mix(col, vec3(0.95, 0.99, 0.94), ss(2.6 * s, 0.2 * s, dPuff) * (1.0 - kAge) * 0.48);
    }
  }

  float z = max(0.0, ball.z);
  vec2 groundPos = ball.xy;
  vec2 drawPos = ball.xy - vec2(0.0, z * 0.55 * s);
  vec2 dpCheck = p - groundPos;
  if (abs(dpCheck.x) > 36.0 * s || abs(dpCheck.y) > 48.0 * s) return;

  // Bóng đổ dưới mặt cỏ (nhỏ và mờ dần khi bóng bay bổng)
  vec2 shOff = (lighting > 0.5 ? vec2(3.8, 3.4) : vec2(2.2, 2.6)) * s + vec2(z * 0.28, z * 0.18) * s;
  float shR = max(3.6, 7.0 - z * 0.04) * s;
  float dSh = length(p - (groundPos + shOff)) - shR;
  col *= 1.0 - ss(2.2 * s, -1.5 * s, dSh) * clamp(0.40 - z * 0.003, 0.10, 0.40);

  // Thân quả bóng tròn
  float rBall = (7.1 + min(z * 0.04, 2.5)) * s;
  vec2 bq = p - drawPos;
  float dB = length(bq) - rBall;
  if (dB < 1.8 * s) {
    col = mix(col, OUTLINE, ss(1.2 * s, 0.1 * s, dB));
    vec2 uv = bq / max(rBall, 1e-3);
    // Ánh sáng cầu 3D
    float shade = clamp(0.88 - 0.32 * dot(uv, uv) - 0.18 * (uv.x * 0.5 + uv.y * 0.7), 0.45, 1.05);
    vec3 bCol = vec3(0.99, 0.99, 0.98) * shade;

    // Họa tiết mảnh đen ngôi sao/ngũ giác lăn theo pha spin
    vec2 ruv = rot(ball.w) * uv;
    float pCenter = length(ruv - vec2(0.18 * sin(ball.w * 0.7), 0.15 * cos(ball.w * 0.7))) - 0.34;
    float pSpokes = 99.0;
    for (int k = 0; k < 5; k++) {
      float ang = float(k) * (2.0 * PI / 5.0) + ball.w * 0.4;
      vec2 edgePt = vec2(cos(ang), sin(ang)) * 0.88;
      pSpokes = min(pSpokes, length(ruv - edgePt) - 0.24);
    }
    float patchMask = ss(0.06, -0.06, min(pCenter, pSpokes));
    bCol = mix(bCol, vec3(0.14, 0.16, 0.20) * shade, patchMask * 0.92);
    // Điểm phản quang bóng
    bCol += vec3(0.22) * ss(0.38, 0.0, length(uv - vec2(-0.32, -0.32)));
    col = mix(col, bCol, ss(0.55 * s, -0.55 * s, dB));
  }
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y * uScale - gl_FragCoord.y) / uScale;
  float W = uRes.x, H = uRes.y;
  float s = min(W, H) / 900.0;
  float t = uTime;

  float x0 = uPitch.x, y0 = uPitch.y, x1 = uPitch.z, y1 = uPitch.w;
  float pw = x1 - x0, ph = y1 - y0;
  vec2 pitchCenter = 0.5 * vec2(x0 + x1, y0 + y1);
  vec2 pitchHalf = 0.5 * vec2(pw, ph);

  // Khoảng cách từ điểm hiện tại tới mép sân cỏ chính (< 0 là trong sân, > 0 là ngoài biên)
  float dPitch = sdBox(p - pitchCenter, pitchHalf, 0.0);
  float dApron = sdBox(p - pitchCenter, pitchHalf + vec2(32.0 * s, 26.0 * s), 10.0 * s);
  float dTrack = sdBox(p - pitchCenter, pitchHalf + vec2(84.0 * s, 54.0 * s), 48.0 * s);

  float nTex = texture(uNoise, fract(p / (220.0 * s))).r;
  float nFine = texture(uNoise, fract(p / (48.0 * s))).g;

  // ---------- 1. Khán đài viền ngoài & Đường chạy điền kinh ----------
  // Nền ngoài cùng tối dịu để icon Desktop ở sát mép màn hình cực kỳ dễ đọc
  vec3 outerCol = uLighting > 1.5
    ? vec3(0.068, 0.092, 0.132)
    : (uLighting > 0.5 ? vec3(0.165, 0.155, 0.178) : vec3(0.145, 0.195, 0.235));
  // Bậc thang khán đài + chấm khán giả cổ vũ tinh tế ở rìa ngoài
  if (dTrack > -1.0 * s) {
    float rowW = 7.5 * s;
    float tierStep = sin(max(0.0, dTrack) * (PI * 2.0 / max(rowW, 1.0)));
    outerCol *= 0.93 + 0.07 * tierStep;
    vec2 seatCell = floor(p / (5.5 * s));
    float hSeat = hash2(seatCell);
    if (dTrack > 5.0 * s && hSeat > 0.42) {
      vec3 crowdC = hSeat < 0.62 ? vec3(0.75, 0.24, 0.24)
                  : hSeat < 0.80 ? vec3(0.20, 0.52, 0.86)
                  : hSeat < 0.92 ? vec3(0.92, 0.78, 0.28)
                                 : vec3(0.88, 0.92, 0.96);
      float sway = 0.5 + 0.5 * sin(t * 2.4 + hSeat * 19.0);
      vec2 seatLocal = fract(p / (5.5 * s)) - 0.5;
      float seatDot = ss(0.38, 0.14, length(seatLocal));
      outerCol = mix(outerCol, crowdC * (uLighting > 1.5 ? 0.52 : 0.62), seatDot * (0.24 + 0.12 * sway));
    }
    // Viền tường chắn giữa khán đài và đường chạy piste
    float dWall = abs(dTrack - 1.5 * s);
    outerCol = mix(outerCol, vec3(0.78, 0.82, 0.86) * (uLighting > 1.5 ? 0.65 : 0.88), ss(2.2 * s, 0.5 * s, dWall) * 0.65);
  }
  vec3 col = outerCol;

  // Đường piste điền kinh màu đất nung ấm áp ôm quanh thảm cỏ
  if (dTrack < 3.0 * s && dApron > -2.0 * s) {
    vec3 trackC = uLighting > 1.5
      ? vec3(0.48, 0.22, 0.18)
      : (uLighting > 0.5 ? vec3(0.72, 0.34, 0.24) : vec3(0.76, 0.36, 0.27));
    trackC *= 0.96 + 0.08 * (nFine - 0.5);
    // Vạch kẻ làn chạy trắng mờ
    float laneDist = dApron;
    float laneLine = abs(fract(laneDist / (11.5 * s)) - 0.5) * (11.5 * s);
    trackC = mix(trackC, vec3(0.95, 0.88, 0.84), (1.0 - ss(0.4 * s, 1.2 * s, laneLine)) * 0.28);
    col = mix(col, trackC, ss(1.5 * s, -1.5 * s, dTrack));
  }

  // ---------- 2. Thảm cỏ biên & Mặt sân cỏ sọc chuẩn FIFA ----------
  if (dApron < 2.0 * s) {
    // Hai tông màu cỏ cắt xen kẽ (tươi tắn ban ngày, ấm vàng chiều tà, xanh ngọc dưới đèn đêm)
    vec3 grassA = vec3(0.220, 0.618, 0.262);
    vec3 grassB = vec3(0.276, 0.696, 0.304);
    if (uLighting > 1.5) {
      grassA = vec3(0.168, 0.565, 0.250);
      grassB = vec3(0.225, 0.655, 0.295);
    } else if (uLighting > 0.5) {
      grassA = vec3(0.265, 0.585, 0.225);
      grassB = vec3(0.345, 0.665, 0.255);
    }

    // 14 dải sọc cỏ cắt dọc sân mượt mà
    float numStripes = 14.0;
    float stripeCoord = clamp((p.x - x0) / max(pw, 1.0), 0.0, 1.0) * numStripes;
    float stripeIdx = mod(floor(stripeCoord), 2.0);
    float edgeDist = min(fract(stripeCoord), 1.0 - fract(stripeCoord)) * (pw / numStripes);
    float stripeBlend = mix(0.5, stripeIdx, ss(0.0, 1.6 * s, edgeDist));
    // Ngoài đường biên là cỏ đậm đồng nhất
    float inPitchMask = ss(1.0 * s, -1.0 * s, dPitch);
    vec3 turf = mix(grassA * 0.88, mix(grassA, grassB, stripeBlend * uStripes), inPitchMask);

    // Hạt cỏ li ti tự nhiên + gợn gió nhẹ
    float shimmer = sin(p.x * 0.018 + p.y * 0.012 - t * 1.4) * 0.012;
    turf *= 0.955 + 0.075 * nFine + 0.035 * (nTex - 0.5) + shimmer;

    col = mix(col, turf, ss(1.5 * s, -1.5 * s, dApron));

    // Khu kỹ thuật & Mái che băng ghế huấn luyện 2 đội ở biên trên
    if (p.y < y0 && p.y > y0 - 24.0 * s) {
      for (int b = 0; b < 2; b++) {
        float bx = pitchCenter.x + (b == 0 ? -0.15 : 0.15) * pw;
        vec2 bp = p - vec2(bx, y0 - 15.5 * s);
        float dBench = sdBox(bp, vec2(32.0 * s, 5.2 * s), 2.0 * s);
        if (dBench < 3.0 * s) {
          vec3 roofC = (b == 0) ? vec3(0.84, 0.24, 0.24) : vec3(0.16, 0.48, 0.86);
          roofC = mix(roofC, vec3(0.94, 0.96, 0.98), ss(1.5 * s, -2.5 * s, bp.y + 1.2 * s) * 0.55);
          col = mix(col, OUTLINE, ss(1.8 * s, 0.2 * s, dBench) * 0.55);
          col = mix(col, roofC, ss(0.7 * s, -0.7 * s, dBench));
        }
        // Vạch khu kỹ thuật trước băng ghế
        vec2 tpBox = p - vec2(bx, y0 - 6.5 * s);
        float dTech = abs(sdBox(tpBox, vec2(36.0 * s, 5.5 * s), 0.0));
        float dash = step(0.45, fract((p.x + p.y) / (7.0 * s)));
        col = mix(col, vec3(0.95, 0.98, 0.96), (1.0 - ss(0.4 * s, 1.2 * s, dTech)) * dash * 0.55);
      }
    }

    // Biển quảng cáo LED chạy dọc ngoài biên sân
    float adDist = abs(dApron + 2.5 * s);
    if (adDist < 3.2 * s) {
      // Chừa khe thoáng phía sau hai khung thành
      float behindGoal = step(abs(p.y - pitchCenter.y), ph * 0.14) * step(pw * 0.5, abs(p.x - pitchCenter.x));
      if (behindGoal < 0.5) {
        float seg = floor((p.x + p.y) / (46.0 * s));
        float hAd = fract(seg * 0.37 + 0.13);
        vec3 adCol = hAd < 0.33 ? vec3(0.12, 0.36, 0.78)
                   : hAd < 0.66 ? vec3(0.86, 0.22, 0.24)
                                : vec3(0.95, 0.74, 0.12);
        if (uLighting > 1.5) adCol = min(vec3(1.0), adCol * 1.25 + 0.08);
        col = mix(col, adCol, ss(2.2 * s, 0.7 * s, adDist) * 0.92);
      }
    }
  }

  // ---------- 3. Hệ thống vạch vôi trắng chuẩn FIFA (SDF chống răng cưa) ----------
  if (dPitch < 6.0 * s) {
    float lw = 1.45 * s;
    float dLine = abs(dPitch); // 4 đường biên dọc & biên ngang

    // Vạch giữa sân & Vòng tròn trung tâm
    dLine = min(dLine, abs(p.x - pitchCenter.x));
    float rCenter = ph * 0.165;
    dLine = min(dLine, abs(length(p - pitchCenter) - rCenter));

    // Đối xứng Trái / Phải cho Vòng cấm 16m50, Khu 5m50 & Vòng cung phạt đền (D-box)
    float dxSide = pitchHalf.x - abs(p.x - pitchCenter.x); // khoảng cách tới đường biên ngang gần nhất
    float dyMid = abs(p.y - pitchCenter.y);

    // Vòng cấm địa 16m50
    float penW = pw * 0.155;
    float penH = ph * 0.27; // nửa chiều cao
    if (dxSide <= penW + 4.0 * s && dyMid <= penH + 4.0 * s) {
      float dPenFront = (dyMid <= penH) ? abs(dxSide - penW) : 999.0;
      float dPenSide = (dxSide <= penW) ? abs(dyMid - penH) : 999.0;
      dLine = min(dLine, min(dPenFront, dPenSide));
    }

    // Khu vực 5m50 cầu môn
    float gaW = pw * 0.055;
    float gaH = ph * 0.14;
    if (dxSide <= gaW + 4.0 * s && dyMid <= gaH + 4.0 * s) {
      float dGaFront = (dyMid <= gaH) ? abs(dxSide - gaW) : 999.0;
      float dGaSide = (dxSide <= gaW) ? abs(dyMid - gaH) : 999.0;
      dLine = min(dLine, min(dGaFront, dGaSide));
    }

    // Chấm phạt đền 11m & Vòng cung D-box ngoài vạch 16m50
    float penSpotDx = pw * 0.108;
    vec2 nearSpot = vec2(p.x < pitchCenter.x ? x0 + penSpotDx : x1 - penSpotDx, pitchCenter.y);
    float rArc = ph * 0.145;
    if (dxSide > penW) {
      dLine = min(dLine, abs(length(p - nearSpot) - rArc));
    }

    // 4 Cung đá phạt góc
    vec2 nearCorner = vec2(p.x < pitchCenter.x ? x0 : x1, p.y < pitchCenter.y ? y0 : y1);
    if (dPitch <= 0.5 * s) {
      dLine = min(dLine, abs(length(p - nearCorner) - 14.0 * s));
    }

    // Chấm giao bóng giữa sân & 2 chấm phạt đền
    float dSpots = min(length(p - pitchCenter) - 3.2 * s, length(p - nearSpot) - 2.6 * s);

    float lineAlpha = max(1.0 - ss(lw * 0.55, lw * 1.45, dLine), ss(0.8 * s, -0.6 * s, dSpots));
    lineAlpha *= ss(2.2 * s, -0.5 * s, dPitch); // gói gọn trong biên sân
    col = mix(col, vec3(0.97, 0.99, 0.98), lineAlpha * 0.90);
  }

  // ---------- 4. Vòng mục tiêu khi người dùng Click chuột lên mặt cỏ ----------
  if (uTarget.w > 0.5 && uTarget.z < 1.8) {
    vec2 tp = p - uTarget.xy;
    if (abs(tp.x) < 68.0 * s && abs(tp.y) < 68.0 * s) {
      float age = uTarget.z;
      float fade = (1.0 - age / 1.8);
      fade *= fade;
      float r1 = (8.0 + age * 28.0) * s;
      float r2 = (4.0 + max(0.0, age - 0.18) * 24.0) * s;
      float ring1 = (1.0 - ss(0.5 * s, 2.2 * s, abs(length(tp) - r1))) * fade;
      float ring2 = (1.0 - ss(0.4 * s, 1.8 * s, abs(length(tp) - r2))) * fade * 0.75;
      float dotC = ss(3.8 * s, 1.2 * s, length(tp)) * fade;
      col = mix(col, vec3(1.0, 0.96, 0.55), clamp(ring1 + ring2 + dotC * 0.8, 0.0, 0.85));
    }
  }

  // ---------- 5. Tất cả cầu thủ chạy chỗ & tranh bóng trên sân ----------
  for (int i = 0; i < 22; i++) {
    if (float(i) >= uPlayerN) break;
    drawPlayer(col, p, uPlPos[i], uPlMeta[i], s, uLighting, uPitch);
  }

  // ---------- 6. Quả bóng đá 3D ----------
  drawBall(col, p, uBall, uBallTrail, s, uLighting);

  // ---------- 7. Hai khung thành & 4 Cờ góc sân (vẽ trên bóng để thấy bóng chui vào lưới!) ----------
  drawGoal(col, p, uPitch, -1.0, s, uGoalFx);
  drawGoal(col, p, uPitch,  1.0, s, uGoalFx);
  drawCornerFlag(col, p, vec2(x0, y0), -1.0, -1.0, s, t);
  drawCornerFlag(col, p, vec2(x1, y0),  1.0, -1.0, s, t);
  drawCornerFlag(col, p, vec2(x0, y1), -1.0,  1.0, s, t);
  drawCornerFlag(col, p, vec2(x1, y1),  1.0,  1.0, s, t);

  // ---------- 8. Hiệu ứng Pháo giấy ăn mừng bàn thắng (Goal!) ----------
  if (uGoalFx.w != 0.0 && uGoalFx.z < 2.2) {
    vec2 gp = p - uGoalFx.xy;
    if (abs(gp.x) < 150.0 * s && abs(gp.y) < 150.0 * s) {
      float ga = uGoalFx.z;
      float gFade = ss(2.2, 0.5, ga);
      // Quầng sáng vàng bùng lên tại khung thành lúc bóng đập lưới
      col += vec3(1.0, 0.88, 0.42) * exp(-length(gp) / (42.0 * s)) * exp(-ga * 3.5) * 0.55;
      for (int k = 0; k < 18; k++) {
        float fk = float(k);
        float ang = fk * 2.39996 + uGoalFx.w * 0.7;
        float spd = (46.0 + mod(fk * 19.0, 58.0)) * s;
        vec2 dirV = vec2(cos(ang), sin(ang));
        // Pháo giấy bay tỏa vào trong sân
        dirV.x = -uGoalFx.w * abs(dirV.x) * 0.85 + dirV.x * 0.35;
        vec2 cPos = uGoalFx.xy + dirV * spd * pow(ga, 0.68) + vec2(0.0, ga * ga * 16.0 * s);
        vec2 cq = rot(ga * (4.0 + mod(fk, 5.0)) + fk) * (p - cPos);
        if (abs(cq.x) < 5.0 * s && abs(cq.y) < 5.0 * s) {
          float dConf = sdBox(cq, vec2(3.4 * s, 1.6 * s), 0.4 * s);
          vec3 confC = mod(fk, 3.0) < 0.5 ? vec3(1.0, 0.82, 0.16)
                     : mod(fk, 3.0) < 1.5 ? vec3(0.96, 0.26, 0.28)
                                          : vec3(0.22, 0.68, 1.0);
          col = mix(col, confC, ss(0.7 * s, -0.5 * s, dConf) * gFade);
        }
      }
    }
  }

  // ---------- 9. Khí quyển ánh sáng & 4 Cột đèn pha góc Sân vận động ----------
  if (uLighting > 1.5) {
    // Đêm Champions League: 4 chùm đèn pha góc sân chiếu chéo rực sáng mặt cỏ
    vec2 towers[4] = vec2[4](
      vec2(x0 - 44.0 * s, y0 - 26.0 * s),
      vec2(x1 + 44.0 * s, y0 - 26.0 * s),
      vec2(x0 - 44.0 * s, y1 + 26.0 * s),
      vec2(x1 + 44.0 * s, y1 + 26.0 * s)
    );
    float floodSum = 0.0;
    for (int i = 0; i < 4; i++) {
      vec2 tw = towers[i];
      vec2 toCenter = normalize(pitchCenter - tw);
      vec2 vp = p - tw;
      float dist = length(vp);
      vec2 vd = vp / max(dist, 1.0);
      float cone = pow(max(0.0, dot(vd, toCenter)), 7.0);
      floodSum += cone * exp(-dist / (max(W, H) * 0.85));

      // Thân giàn đèn & cụm bóng LED rực sáng ở 4 góc
      if (dist < 26.0 * s) {
        float dHead = sdBox(rot(atan(toCenter.y, toCenter.x)) * vp, vec2(4.5 * s, 11.5 * s), 2.0 * s);
        col = mix(col, vec3(0.25, 0.30, 0.36), ss(1.5 * s, 0.0, dHead - 1.5 * s));
        col = mix(col, vec3(0.96, 0.98, 1.0), ss(1.0 * s, -0.8 * s, dHead));
      }
      col += vec3(0.85, 0.93, 1.0) * exp(-dist / (18.0 * s)) * 0.55;
    }
    // Mặt sân bừng sáng tương phản cao dưới ánh đèn pha, khán đài ngoài chìm trong đêm
    float inStadium = ss(16.0 * s, -18.0 * s, dApron);
    col *= mix(0.76, 1.04, inStadium);
    col += vec3(0.88, 0.95, 1.0) * floodSum * mix(0.22, 0.055, inStadium);
  } else if (uLighting > 0.5) {
    // Nắng chiều vàng ấm chiếu từ góc Tây Bắc
    float sunGrad = clamp(1.0 - (p.x / max(W, 1.0) * 0.55 + p.y / max(H, 1.0) * 0.45), 0.0, 1.0);
    col = mix(col, col * vec3(1.08, 0.98, 0.86) + vec3(0.05, 0.025, 0.0), 0.55);
    col += vec3(1.0, 0.82, 0.48) * pow(sunGrad, 2.2) * 0.11;
  } else {
    // Ban ngày nắng đẹp: viền ngoài hơi dịu nhẹ để tôn sân cỏ trung tâm
    float dayVig = ss(max(W, H) * 0.78, max(W, H) * 0.25, length(p - pitchCenter));
    col *= 0.93 + 0.08 * dayVig;
  }

  o = vec4(clamp(col, 0.0, 1.0), 1.0);
}
