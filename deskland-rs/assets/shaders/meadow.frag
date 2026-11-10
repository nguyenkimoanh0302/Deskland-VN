#version 300 es
precision highp float;
uniform sampler2D uNoise, uFA, uFB;
uniform vec2 uRes, uGrid;
uniform float uScale, uSeed, uStep, uMix, uTime, uDebug, uEdge;
#ifdef MEADOW_ACTOR
uniform sampler2D uActor, uActorShadow;
uniform vec4 uActorRect;
#endif
#ifdef MEADOW_CAT
uniform sampler2D uCat;
uniform vec4 uCatRect;
uniform vec2 uCatShadow;
uniform float uCatTail;
#endif
uniform highp sampler2D uCirc;   // Các hình tròn của mây chia theo ô lưới
uniform vec2 uCGrid;             // Số lượng ô lưới
uniform float uCellG;            // Kích thước ô lưới (px)
out vec4 o;

// ---- Bảng màu ----
const vec3 G_LIGHT = vec3(0.76, 0.83, 0.52), G_MID = vec3(0.62, 0.75, 0.42);
const vec3 G_DEEP = vec3(0.52, 0.68, 0.36);
const vec3 INK = vec3(0.24, 0.38, 0.18);
const vec3 SHADOW = vec3(0.56, 0.66, 0.52);
const vec3 CLOUD_HI = vec3(1.0, 1.0, 0.99);
const vec3 CLOUD_BODY = vec3(0.90, 0.93, 0.96), CLOUD_SH = vec3(0.80, 0.85, 0.90);
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

// Khoảng cách tới mây (<0 là trong mây)
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

float segD(vec2 p, vec2 a, vec2 b) { vec2 pa = p - a, ba = b - a; return length(pa - ba * clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0)); }

// Nét vẽ cỏ
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

#ifdef MEADOW_CAT
float catTailMask(vec2 uv) {
  return smoothstep(.635,.665,uv.x)*smoothstep(.715,.75,uv.y);
}
vec4 catColor(vec2 uv) {
  vec2 pivot=vec2(.68,.73),delta=uv-pivot;
  float angle=-uCatTail*smoothstep(.025,.16,length(delta));
  float c=cos(angle),s=sin(angle);
  vec2 source=pivot+vec2(c*delta.x-s*delta.y,s*delta.x+c*delta.y);
  vec4 body=texture(uCat,uv),tail=texture(uCat,source);
  body.a*=1.-catTailMask(uv);tail.a*=catTailMask(source);
  float alpha=body.a+tail.a;
  return vec4((body.rgb*body.a+tail.rgb*tail.a)/max(alpha,.001),min(alpha,1.));
}
#endif

void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y * uScale - gl_FragCoord.y) / uScale;

  // ---------- Đồng cỏ ----------
  float w1 = N(p, 1900.0).b, w2 = N(p + 311.0, 650.0).g;
  vec3 col = mix(G_MID, G_LIGHT, ss(0.3, 0.75, w1 * 0.7 + w2 * 0.3));
  col = mix(col, G_DEEP, ss(0.45, 0.85, N(p + 977.0, 1300.0).r) * 0.4);
  col *= 1.0 + 0.05 * (0.5 - (p.x + p.y) / (uRes.x + uRes.y));
  col *= 1.0 + (hash2(floor(p * (0.8 * max(uScale, 1.0)))) - 0.5) * 0.035;
  col = mix(col, INK, grassInk(p) * 0.75);

#ifdef MEADOW_ACTOR
  vec2 actorUV=(p-uActorRect.xy)/uActorRect.zw;
  actorUV.y=1.0-actorUV.y;
  float inActor=step(0.0,actorUV.x)*step(0.0,actorUV.y)*step(actorUV.x,1.0)*step(actorUV.y,1.0);
  vec4 actor=texture(uActor,actorUV);
  float actorShadow=texture(uActorShadow,actorUV).r*inActor;
  col=mix(col,col*SHADOW,actorShadow*0.85);
  col=mix(col,actor.rgb,actor.a*inActor);
#endif

#ifdef MEADOW_CAT
  vec2 catUV=(p-uCatRect.xy)/uCatRect.zw;
  vec2 catShadowUV=(p-uCatShadow-uCatRect.xy)/uCatRect.zw;
  vec4 cat=catColor(catUV);
  float catShadow=catColor(catShadowUV).a;
  float inCat=step(0.,catUV.x)*step(0.,catUV.y)*step(catUV.x,1.)*step(catUV.y,1.);
  float inShadow=step(0.,catShadowUV.x)*step(0.,catShadowUV.y)*step(catShadowUV.x,1.)*step(catShadowUV.y,1.);
  col=mix(col,col*SHADOW,catShadow*inShadow*.78);
  col=mix(col,cat.rgb,cat.a*inCat);
#endif

  // ---------- Bóng mây đổ trên đồng cỏ ----------
  float sd = cloudDist(p - SHADOW_OFF);
  col = mix(col, col * SHADOW, ss(3.0, -3.0, sd) * 0.85);

  // ---------- Mây ----------
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
