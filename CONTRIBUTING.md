# Đóng góp cho Deskland-VN

Cảm ơn bạn muốn đóng góp! Cách đóng góp được chào đón nhất là **tạo chủ đề (theme) mới**. Ngoài ra bạn có thể sửa lỗi, dịch, tối ưu hiệu năng hay viết tài liệu.

> **English summary:** Themes are one GLSL fragment shader shared by a web page (`prototype/<id>.html`, used by the settings preview and the C# host) and a Rust port (`deskland-rs/src/themes/<id>.rs`). Register the theme in `prototype/themes.json`, `prototype/i18n/{vi,en}.json` and `deskland-rs/src/main.rs`, add a 1024×576 thumbnail, run `node tools/check-themes.mjs` and `cargo build --release`, then open a PR. Original artwork only, GPL-3.0, no network access, no keyboard hooks or fake lock screens.

## Mục lục
1. [Chuẩn bị](#1-chuẩn-bị)
2. [Cấu trúc một chủ đề](#2-cấu-trúc-một-chủ-đề)
3. [Tạo chủ đề mới từng bước](#3-tạo-chủ-đề-mới-từng-bước)
4. [Quy tắc bắt buộc](#4-quy-tắc-bắt-buộc)
5. [Kiểm tra trước khi gửi PR](#5-kiểm-tra-trước-khi-gửi-pr)

## 1. Chuẩn bị

- Windows 10/11 x64.
- [Rust](https://rustup.rs) (bản stable, toolchain `x86_64-pc-windows-msvc`) và **Windows SDK** (có sẵn khi cài "Desktop development with C++" của Visual Studio Build Tools; cần `rc.exe` để nhúng biểu tượng).
- [Node.js](https://nodejs.org) 18+ để chạy máy chủ thử và script kiểm tra.

```bash
cd deskland-rs && cargo build --release
```

```bash
node prototype/serve.mjs
```

Máy chủ thử chạy tại `http://localhost:5178/<id>.html`. Mở thẳng trang chủ đề trong trình duyệt sẽ có biểu tượng mẫu để bạn bấm thử tương tác. Trang Cài đặt nằm ở `http://localhost:5178/settings.html`.

## 2. Cấu trúc một chủ đề

Mỗi chủ đề có **một shader GLSL duy nhất** dùng chung cho hai bản. Nhờ vậy bản web (dùng cho khung xem trước trong Cài đặt và bản C#) và bản Rust không bao giờ lệch nhau.

| Tệp | Vai trò |
|---|---|
| `prototype/<id>.frag` | Fragment shader `#version 300 es`. Toạ độ `p` tính theo px CSS, gốc góc trên-trái. |
| `prototype/<id>.html` | Trang web: tải `<id>.frag`, mô phỏng CPU (nhân vật, hạt, tương tác), giao thức với host. |
| `deskland-rs/src/themes/<id_gach_duoi>.rs` | Bản Rust: cùng mô phỏng, `include_str!("../../../prototype/<id>.frag")`. |
| `prototype/themes.json` | Tên, mô tả, trang, ảnh thu nhỏ, nhóm thông số (thanh trượt / nút chọn). |
| `prototype/i18n/vi.json`, `en.json` | Bản dịch tên, mô tả, nhãn thông số (`themes.<id>`). |
| `prototype/thumbs/<id>.jpg` | Ảnh thu nhỏ 1024×576. |

Mẫu tham khảo tốt nhất: **`aquarium`** (bể cá vàng, câu cá bằng click) và **`commute`** (chuyến xe chiều, trời theo giờ máy).

### Uniform chung (mọi chủ đề đều nhận)

`uNoise` (texture nhiễu lặp 256×256), `uFA`/`uFB` + `uMix` (trường khoảng cách quanh biểu tượng, nội suy khi biểu tượng di chuyển), `uRes` (kích thước màn hình px CSS), `uGrid`, `uStep`, `uScale`, `uSeed`, `uTime`, `uDebug`.

`s = mix(fieldS(uFA, p), fieldS(uFB, p), uMix)` là **khoảng cách có dấu tới vùng biểu tượng**: `s < 0` là bên trong vùng biểu tượng. Chủ đề phải "ôm" biểu tượng bằng giá trị này (đảo cát, đồi cỏ, mây...) để biểu tượng luôn dễ nhìn. Hai thông số `sigma` (độ rộng) và `warp` (độ uốn) điều khiển trường này.

### Giao thức với host (bản web)

Trang chạy ở 3 chế độ: mở thẳng (thử nghiệm), `?preview=1` (khung xem trước trong Cài đặt, nhận `postMessage` từ trang cha) và `?wallpaper=1` (bản C#, nhận thông điệp WebView2). Các thông điệp:

- `{t:'icons', vis, r:[[l,t,r,b],...]}`: vị trí biểu tượng (px vật lý, chia `devicePixelRatio`).
- `{t:'settings', v:{...}}`: đổi thông số (kể cả `seed`).
- `{t:'reset', v}`: về mặc định.
- `{t:'pause', v:true|false}`: tạm dừng. **Khi đang tạm dừng vẫn phải vẽ lại một khung** nếu biểu tượng hay thông số vừa đổi (xem biến `needDraw` trong `aquarium.html`), nếu không khung xem trước sẽ trống.

Trang gửi `{t:'ready'}` khi đã sẵn sàng.

### Bản Rust

Hiện thực trait `Theme` trong `deskland-rs/src/engine.rs`:

- `set_icons`: biểu tượng đổi.
- `frame(dt)`: cập nhật mô phỏng rồi vẽ.
- `settle`: vẽ một khung đứng yên.
- `handle_click(x, y)`: click vào vùng trống của desktop, toạ độ px CSS.
- `is_interacting`: trả `true` khi đang có hiệu ứng, để vòng lặp tăng lên 50 khung/giây.
- `destroy`, `field_ms`.

Dùng `Base` (trường khoảng cách, `cur_s(x, y)`, `bind_common`), `Rng`, `Noise` có sẵn. Logic CPU nên dịch 1:1 từ bản web.

## 3. Tạo chủ đề mới từng bước

1. **Chọn `id`**: chữ thường và dấu gạch ngang, ví dụ `rice-field`. Tên tệp Rust dùng gạch dưới: `rice_field.rs`.
2. **Viết shader** `prototype/<id>.frag`. Bắt đầu từ `aquarium.frag`, giữ nguyên khối uniform chung và các hàm `hash2`, `ss`, `N`, `fieldS`.
3. **Viết trang web** `prototype/<id>.html`. Chép từ `aquarium.html`: giữ nguyên khối hạ tầng (nhiễu, trường khoảng cách, WebGL, `commit`, giao thức host), chỉ thay phần cảnh: `DEFAULTS`, mô phỏng, `click()`, `draw()`.
4. **Thử trên trình duyệt**: mở `http://localhost:5178/<id>.html`, bấm thử mọi vùng (nước / cát / trời...), đổi cỡ cửa sổ.
5. **Đăng ký** trong `prototype/themes.json` (thêm vào cuối mảng `themes`) và `prototype/i18n/vi.json` + `en.json` (mục `themes.<id>`). Thông số dạng số dùng `min`, `max`, `step`, `default` (thêm `unit` hoặc `format: "percent" | "times"`). Thông số dạng nút chọn dùng `options` + `default` là chỉ số.
6. **Port sang Rust** `deskland-rs/src/themes/<id_gach_duoi>.rs`, rồi đăng ký trong `deskland-rs/src/main.rs`:
   - `mod themes { pub mod <id_gach_duoi>; }`
   - mảng `THEMES`: `("<id>", "Tên tiếng Việt", <số lệnh kế tiếp>)`
   - `canonical_theme`: bí danh (không bắt buộc)
   - mảng `"themes"` trong `web_init` (để trang Cài đặt cho chọn)
   - `make_theme` (tạo đối tượng) và bảng chọn shader (`FRAG`)

   Thêm `ParamDef` trong `deskland-rs/src/settings.rs` cho cửa sổ Cài đặt Win32 dự phòng.
7. **Ảnh thu nhỏ** 1024×576 JPG (chất lượng ~88) tại `prototype/thumbs/<id>.jpg`. Chụp từ chính chủ đề đang chạy.
8. **Chạy thử bản Rust**:

   ```bash
   deskland-rs\target\release\deskland-rs.exe --theme <id> --test-click
   ```

   Xem nhật ký `deskland-rs.log` cạnh exe: thời gian biên dịch shader, "CPU vẽ … ms/khung", lỗi nếu có.

## 4. Quy tắc bắt buộc

- **Tác phẩm nguyên bản.** Không dựng lại cảnh, nhân vật, logo, chữ hay phong cách đặc trưng của phim, game, tranh, video có bản quyền, kể cả khi đổi tên. Lấy cảm hứng về không khí, thể loại thì được. Hình ảnh, âm thanh do bạn tự làm hoặc có giấy phép tương thích GPL-3.0; ghi nguồn trong `THIRD_PARTY_NOTICES.md`.
- **Giấy phép**: mọi đóng góp phát hành theo **GPL-3.0** như phần còn lại của dự án.
- **Không truy cập mạng**, không thu thập dữ liệu, không tải mã từ bên ngoài lúc chạy.
- **An toàn**: không hook bàn phím, chuột; không chặn Win+L; không giả lập màn hình khóa; không yêu cầu hay xử lý mật khẩu; không sửa cài đặt hệ thống ngoài phạm vi tính năng đã có.
- **Ôm biểu tượng**: biểu tượng phải luôn dễ nhìn (dùng trường `s`), cảnh không được che biểu tượng.
- **Hiệu năng**: ở 20 khung/giây, "CPU vẽ" trong `deskland-rs.log` nên dưới ~6 ms/khung trên GPU tích hợp. Vòng lặp trong shader phải có giới hạn cố định và thoát sớm (kiểm tra khoảng cách trước khi vẽ chi tiết).
- **Tương tác**: click phải có phản hồi rõ ràng và tự kết thúc. Đừng để hiệu ứng chạy mãi khiến `is_interacting` luôn bật (tốn pin).

## 5. Kiểm tra trước khi gửi PR

```bash
node tools/check-themes.mjs
```

```bash
cd deskland-rs && cargo build --release --locked
```

CI trên GitHub chạy lại đúng hai bước này cho mọi PR và đính kèm `deskland-rs.exe` để người review tải về thử.

Trong mô tả PR, hãy kèm: ảnh hoặc GIF ngắn của chủ đề (ngày / đêm nếu có), mô tả tương tác click, số "CPU vẽ ms/khung" trong nhật ký, và xác nhận tác phẩm là nguyên bản.

## Phát hành (dành cho người bảo trì)

Gắn tag phiên bản rồi đẩy lên. Workflow `Release` tự build trên GitHub và đăng `Deskland-VN-<tag>-win-x64.zip` (kèm SHA-256) lên trang Releases:

```bash
git tag v0.2.0 && git push origin v0.2.0
```
