# deskland-rs — spike Rust native

Bản Rust 1 tiến trình (Win32 + OpenGL 3.3 qua WGL) để đo hiệu năng so với bản C# + WebView2.
Mới port 2 chủ đề: **Bãi biển** (`beach`, mặc định của bản C#) và **Thảo nguyên** (`meadow`).

## Chạy

```bash
cargo build --release
target/release/deskland-rs.exe --theme beach      # hoặc meadow; --fps 20 (mặc định)
target/release/deskland-rs.exe --quit             # thoát (hoặc chuột phải icon khay)
target/release/deskland-rs.exe --snapshot <thư mục>  # lưu BMP khung hình từ GPU sau 4 giây
```

Cài đặt lưu ở `~/.deskland-rs/settings.json` (chủ đề, `seed` + thông số từng chủ đề, `lockSyncWin`, `lockIdleMins`, `lang`).
Lần đầu chưa có file này thì đọc của bản C# (`%LOCALAPPDATA%\DesklandPreview\settings.json`), không ghi đè file của bản C#.

**Cửa sổ Cài đặt** (bấm chuột trái icon khay, chuột phải > Cài đặt…, hoặc `deskland-rs.exe --settings`):
giống hệt bản gốc vì mở **chính trang `prototype/settings.html`** trong WebView2 (`src/web.rs`, ánh xạ `https://app.local/`
vào thư mục `prototype` tìm thấy cạnh exe hoặc đi ngược lên từ exe). Có lưới chủ đề, xem trước trực tiếp với vị trí biểu
tượng thật, thông số theo nhóm, đổi ngôn ngữ, khởi động cùng Windows. Giao thức thông điệp giống bản C#; bản Rust gửi
thêm `caps` trong `init` để trang:
- chỉ cho chọn chủ đề đã port (còn lại hiện "Sắp ra mắt"), ẩn thông số chưa port (cây dừa, cậu bé, mèo) và tắt chúng
  trong khung xem trước cho giống desktop, ẩn nút chia sẻ ảnh / video;
- thay nhóm Màn hình khóa bằng các tùy chọn **khóa thật của Windows** (Khóa với cảnh động, cảnh động khi rảnh, ảnh màn
  khóa). Không có PIN, không chặn Win+L, không tự kiểm mật khẩu. Bản C# không gửi `caps` nên trang của nó giữ nguyên.

WebView2 chỉ chạy khi cửa sổ đang mở (~6 tiến trình, ~290 MB riêng); đóng là tắt hẳn. Bản Rust giữ thêm ~9 MB sau lần mở
đầu (thư viện COM của WebView2). Không có WebView2 Runtime hoặc không thấy thư mục `prototype` thì dùng cửa sổ Win32
dự phòng (`src/settings.rs`, mở thẳng bằng `--settings-win32`). Kiểm thử: `--webview-debug` mở cổng DevTools 9222.
Nhật ký: `deskland-rs.log` cạnh file exe (in RAM + khung/giây mỗi 10 giây).

## Đã port
- Gắn vào lớp hình nền (24H2+: cửa sổ con phân lớp của Progman; Classic: WorkerW), giữ thứ tự lớp, đa màn hình, DPI.
- Đọc khung biểu tượng (`LVM_GETITEMRECT` qua `VirtualAllocEx`), tạm dừng khi có cửa sổ phóng to / toàn màn hình.
- Trường khoảng cách quanh biểu tượng, texture nhiễu, nội suy 2 trường; shader GLSL giữ nguyên văn
  (chỉ đổi `#version 300 es` → `#version 330 core`), render ở 72% độ phân giải rồi phóng lên như bản web.
- Beach: kayak (tránh nhau, tránh bờ), hải âu. Meadow: mây quanh biểu tượng, mây trôi tự do, bóng mây.
- Một ngữ cảnh OpenGL dùng chung cho mọi màn hình: shader + texture nhiễu chỉ nạp 1 lần.

## Hiển thị lên desktop (`--present gdi|gl`)
Mặc định **gdi**: OpenGL vẽ vào FBO trên cửa sổ ẩn → GPU phóng lên độ phân giải thật → đọc về PBO (không chặn)
→ `SetDIBitsToDevice` lên cửa sổ hình nền. Lý do: ở chế độ 24H2+ cửa sổ hình nền là cửa sổ con `WS_EX_LAYERED`,
và `SwapBuffers` của OpenGL **không** ra bề mặt của cửa sổ đó (đã kiểm bằng `PrintWindow`: chế độ `gl` đen,
chế độ `gdi` có hình). `--present gl` vẫn giữ để so sánh.

Hai điều bắt buộc để cửa sổ con phân lớp hiện lên màn hình (thiếu là desktop đen dù cửa sổ có hình):
- Bật `WS_EX_LAYERED` + `SetLayeredWindowAttributes` **sau** `SetParent` (`src/desktop.rs`).
- Nhúng `app.manifest` khai báo hỗ trợ Windows 8–11 (`build.rs`, như `host/app.manifest` của bản C#).

## Màn hình khóa - chỉ dùng khóa thật của Windows (`src/lock.rs`)
Không hook bàn phím, không chặn Win+L, không ghi `DisableLockWorkstation`, không tự kiểm hay lưu mật khẩu / PIN.
- **Khóa máy ngay** (menu khay) = `LockWorkStation()`. Win+L để nguyên cho Windows.
- **Cảnh động khi rảnh** = trình bảo vệ màn hình Windows: chép exe thành `~/.deskland-rs/Deskland.scr`, đặt
  `SCRNSAVE.EXE`, `ScreenSaveTimeOut` (= `lockIdleMins`), `ScreenSaverIsSecure=1`. Có thao tác là `.scr` thoát và
  **Windows** hiện màn đăng nhập. Chạy thử: `deskland-rs.exe --saver`.
- **Khóa với cảnh động** (menu khay hoặc **Ctrl+Alt+L**, đăng ký bằng `RegisterHotKey`, không hook) = nhờ chính
  Windows bật trình bảo vệ màn hình ngay (`WM_SYSCOMMAND` / `SC_SCREENSAVE`). Nếu chưa đặt Deskland làm trình bảo vệ
  màn hình có khóa thì hỏi trước. Sau 2,5 giây mà trình bảo vệ màn hình không chạy thì khóa luôn bằng `LockWorkStation()`.
  Win+L và nút Khóa của Windows vẫn ra màn khóa ảnh tĩnh: Windows không cho ứng dụng vẽ lên màn khóa thật.
- **Ngừng vẽ khi không ai thấy desktop**: phiên khóa (`WTSRegisterSessionNotification`), input desktop không phải
  `Default` (màn Winlogon / Ctrl+Alt+Del / UAC), hoặc trình bảo vệ màn hình đang chạy. Kiểm mỗi 500 ms; đo lúc máy
  khóa: CPU 0% (so với ~14% khi đang vẽ).
- **Ảnh màn khóa Windows** (`lockSyncWin`) = chụp cảnh màn hình chính → JPG → `LockScreen.SetImageFileAsync`.
- Nếu bản C# cũ để lại `DisableLockWorkstation=1`: hiện cảnh báo + mục "Bật lại Win+L của Windows" (chỉ làm khi bấm).

## Chưa port
Cây dừa (`beach-palms.js`), cậu bé + mèo (`meadow-boy.js`, `meadow-cat.js`), gợn nước khi bấm, 8 chủ đề còn lại,
đồng hồ / widget trên màn khóa (màn khóa thật của Windows không cho vẽ đè), chia sẻ ảnh/video, đa ngôn ngữ,
ảnh xem trước chủ đề trong cửa sổ Cài đặt.

## Kết quả đo (10/11/2026, Intel UHD 770, 2 màn 1920×1080, `tools/measure-ram.ps1`)

| | Tiến trình | Private RAM | Working Set | GPU Shared | CPU (% 1 nhân) |
|---|---:|---:|---:|---:|---:|
| C# + WebView2 — beach (không cây dừa) | 11 | ~620 MB* | ~1.210 MB | ~250 MB | ~15% |
| **Rust — beach** | **1** | **55,6 MB** | **136,6 MB** | **107 MB** | **~5,6%** |
| C# + WebView2 — meadow (không cậu bé/mèo) | 11 | ~650 MB* | ~1.240 MB | ~266 MB | ~20% |
| **Rust — meadow** | **1** | **63,1 MB** | **143,8 MB** | **114 MB** | **~9,6%** |

| **Rust — beach, `--present gdi` (mặc định, hiện được trên desktop 24H2)** | **1** | **97,7 MB** | **124,2 MB** | **97,9 MB** | **~14%** |

\* Giá trị ổn định (2 mẫu cuối); ngay sau khởi động bản C# lên ~790 MB.
Hai dòng Rust đầu đo ở chế độ `gl` cũ - nhẹ hơn nhưng không hiện trên cửa sổ con phân lớp của 24H2.
Khởi động bản Rust: khung hình đầu sau ~350–480 ms (gồm ~160–250 ms biên dịch shader của driver).
Tốc độ khung hình bản Rust: ~18 khung/giây (đặt 20, bằng mức `cap30` của bản web trên màn 60 Hz).
