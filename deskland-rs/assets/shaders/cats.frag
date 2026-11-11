#version 300 es
// Chủ đề Mèo con trên đồi: cảnh được vẽ bằng tiny-skia trên CPU (src/themes/cats), shader chỉ chép từng điểm ảnh
// của texture ra framebuffer (hàng 0 của ảnh là mép trên màn hình).
precision highp float;
uniform sampler2D uTex;
uniform vec2 uBuf;
out vec4 o;
void main() {
  ivec2 px = ivec2(gl_FragCoord.xy);
  o = vec4(texelFetch(uTex, ivec2(px.x, int(uBuf.y) - 1 - px.y), 0).rgb, 1.0);
}
