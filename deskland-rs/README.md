# deskland-rs — Phiên bản Rust Native (Win32 + OpenGL 3.3)

Phiên bản viết lại hoàn toàn bằng **Rust thuần (1 tiến trình duy nhất, Win32 + OpenGL 3.3 qua WGL)** của Deskland, thay thế kiến trúc đa tiến trình C# + WebView2 thường trực để tối ưu triệt để RAM/CPU, bổ sung **tương tác chuột trực tiếp trên màn hình nền**, nâng tổng số chủ đề lên **14 chủ đề** và chuyển sang cơ chế **màn hình khóa thật 100% của Windows**.

---

## Điểm nổi bật của bản Rust Native

1. **Siêu nhẹ — 1 tiến trình duy nhất (`deskland-rs.exe` ~2 MB):**
   - Không nuôi trình duyệt ngầm khi chạy hình nền: chỉ **1 tiến trình** (~65–98 MB Private RAM tùy độ phân giải & số màn hình), giảm **~6–8 lần RAM** so với bản C# + WebView2 (~11 tiến trình, ~620–790 MB RAM).
   - Một ngữ cảnh OpenGL 3.3 dùng chung cho mọi màn hình; biên dịch shader & khởi tạo khung hình đầu tiên chỉ mất **~350–480 ms**.
2. **Đầy đủ 14 chủ đề thích ứng biểu tượng + Tương tác Click chuột riêng biệt:**
   - Tự động đọc vị trí biểu tượng màn hình nền (`SysListView32`), dựng trường khoảng cách (SDF) trên CPU và uốn lượn bờ biển, đồng cỏ, rạn cát, tán rừng, hàng rào sân vận động... ôm quanh các nhóm icon theo thời gian thực.
   - Mỗi chủ đề đều có phản hồi vật lý / hoạt ảnh riêng khi người dùng click chuột trái vào vùng trống trên màn hình nền.
3. **Bắt chuột thông minh & Adaptive FPS (`20 -> 50 FPS`):**
   - Sử dụng `WM_INPUT` (`RIDEV_INPUTSINK`) không chiếm quyền chuột của Windows, chỉ kích hoạt khi click gọn vào đúng vùng trống Desktop (`Progman` / `WorkerW` / `SysListView32`) và **tự động bỏ qua khi bấm vào biểu tượng (icon)** hoặc cửa sổ ứng dụng khác.
   - Bình thường giữ nhịp tiết kiệm **20 FPS**; ngay khi có tương tác hoặc hiệu ứng động (bóng lăn, phao câu giật, cánh hoa bay...), bộ điều phối tự động tăng tốc lên **50 FPS** mượt mà rồi hạ về **20 FPS** khi hiệu ứng lắng xuống.
4. **Cửa sổ Cài đặt Kép (WebView2 On-Demand + Win32 Native Fallback):**
   - Mở chính giao diện `prototype/settings.html` giàu trực quan qua WebView2 **chỉ khi cửa sổ Cài đặt đang mở**; đóng cửa sổ là giải phóng toàn bộ tiến trình `msedgewebview2.exe`.
   - Nếu máy không có WebView2 Runtime hoặc chạy độc lập một file `.exe` không kèm thư mục `prototype/`, ứng dụng tự động chuyển sang **cửa sổ Cài đặt Win32 thuần** (`src/settings.rs`) có đầy đủ chọn chủ đề, thanh trượt tham số và tùy chọn khóa màn hình.
5. **Bảo mật chuẩn hệ điều hành — Chỉ dùng khóa thật của Windows (`src/lock.rs`):**
   - Không hook bàn phím mức thấp, không chặn `Win+L`, không lưu/kiểm mã PIN tự chế.
   - Hỗ trợ khóa ngay (`LockWorkStation()`), trình bảo vệ màn hình cảnh động bảo mật (`Deskland.scr` với `ScreenSaverIsSecure=1`), phím tắt `Ctrl+Alt+L` và tự động đồng bộ cảnh hiện tại làm ảnh nền màn khóa Windows.
   - Tự động phát hiện và cho phép khôi phục lại `Win+L` nếu bản C# cũ từng ghi `DisableLockWorkstation=1`.
6. **Tiết kiệm năng lượng tuyệt đối (`0% CPU` khi khuất hoặc khóa máy):**
   - Tự động tạm ngừng vẽ khi có cửa sổ phóng to / toàn màn hình che khuất màn hình nền, khi phiên Windows đang khóa (`WTSRegisterSessionNotification`), ở màn hình `Winlogon` / UAC, hoặc khi trình bảo vệ màn hình đang chạy.
7. **Biểu tượng ứng dụng nhúng sẵn & Menu Khay hệ thống tinh gọn:**
   - Nhúng trực tiếp icon đa độ phân giải (`16×16` đến `256×256`) và `VERSIONINFO` vào `deskland-rs.exe` qua `app.rc` + `build.rs` (hiển thị đồng bộ trên Explorer, Taskbar, Khay hệ thống và thanh tiêu đề Cài đặt).
   - Menu chuột phải ở khay hệ thống gọn gàng: chỉ hiển thị chủ đề đang chọn (`✓ Chủ đề: <Tên chủ đề>`) cùng các tác vụ nhanh, việc đổi chủ đề và tinh chỉnh chuyên sâu tập trung tại cửa sổ **Cài đặt…**.

---

## Danh sách 15 Chủ đề & Cơ chế Tương tác Chuột

| # | Mã (`--theme`) | Bí danh (Alias) | Tên hiển thị | Mô tả cảnh & Tương tác khi Click vùng trống Desktop |
|---:|---|---|---|---|
| 1 | `meadow` | — | **Thảo nguyên** | Đồng cỏ xanh và mây trắng uốn quanh icon, bóng mây trôi dưới nắng. **Click:** Tạo cụm mây puff bồng bềnh bay lên hòa nhập vào tầng mây trời. |
| 2 | `beach` | — | **Bãi biển** | Bờ cát vàng, sóng ngọc lam vỗ bờ quanh đảo icon, thuyền kayak và hải âu. **Click:** Tạo vòng sóng lan tỏa trên nước, gọi thuyền kayak hướng mũi tới và hải âu nghiêng cánh. |
| 3 | `beach-turquoise` | `maldives` | **Đảo Maldives** | Đảo cát trắng giữa đầm phá san hô ngọc bích, đàn cá nhiệt đới và rùa biển. **Click:** Thả mồi thu hút đàn cá & rùa biển bơi tới, tạo gợn nước và gọi hải âu. |
| 4 | `lake` | — | **Hồ nước** | Mặt hồ tĩnh lặng, lá sen/hoa súng ôm bờ icon và đàn cá Koi bơi lội. **Click:** Mini-game câu cá thư giãn — click mặt hồ quăng phao câu, chờ cá đớp mồi giật phao rồi click lần nữa để kéo cá nhảy lên mặt nước. |
| 5 | `sky` | — | **Bầu trời** | Biển mây bồng bềnh đa tầng chuyển sắc Ngày / Hoàng hôn / Đêm (hoặc tự động theo giờ thực). **Click:** Ban ngày & hoàng hôn tạo cụm mây mới và gọi đàn chim bay qua; ban đêm phóng **sao băng** xẹt sáng bầu trời. |
| 6 | `ocean` | — | **Đại dương** | Đảo mây 3D nổi giữa đại dương xanh thẫm nhìn từ trên cao, thuyền buồm & du thuyền rẽ sóng. **Click:** Tạo vòng sóng định vị (waypoint) dẫn hướng cho thuyền buồm & du thuyền tiến về điểm chọn. |
| 7 | `rainforest` | — | **Rừng mưa nhiệt đới** | Tán rừng mưa nhiều tầng bao quanh trảng cỏ rêu icon, mưa rơi và sương mù. **Click:** Tạo gợn nước mưa, thu hút đàn đom đóm phát sáng tụ về và thúc chim nhiệt đới vỗ cánh. |
| 8 | `mocchau` | `plum-blossoms`, `plum` | **Hoa mận** | Rừng hoa mận trắng Mộc Châu ôm đồi cỏ xuân trong sương sớm. **Click:** Rung cành làm bung chùm cánh hoa mận trắng bay lả tả theo gió và gọi chim rừng bay lượn. |
| 9 | `tet-sakura` | `tet_sakura` | **Hoa anh đào Tết** | Sân xuân giấy dó ấm áp bao quanh bởi rừng đào hồng thắm, đèn lồng đỏ và bao lì xì. **Click:** Làm đung đưa đèn lồng / lì xì gần đó, tung cánh hoa đào xoay trong gió và gọi chim én mùa xuân. |
| 10 | `cat-hill` | `cat_hill` | **Đồi mèo nắng ấm** | Đồi cỏ nắng ấm với những chú mèo mập đi dạo, ngồi liếm lông và nằm ngủ thở đều. **Click:** Bấm trúng mèo để **vuốt ve** (mèo nảy lên vui vẻ); bấm lên thảm cỏ để thả **bướm vàng** thu hút mèo chạy theo vờn. |
| 11 | `stadium` | `football`, `soccer`, `pitch` | **Sân bóng sôi động** *(Mới)* | Sân vận động bóng đá nhìn từ trên cao (Ban ngày / Hoàng hôn / Đèn pha ban đêm), hai đội Đỏ – Xanh chạy chỗ chuyền bóng quanh chướng ngại icon. **Click:** Bấm trên sân để chuyền/lăn bóng tới vị trí đó cho cầu thủ bứt tốc đuổi theo; bấm vào khung thành để sút ghi bàn kèm hiệu ứng pháo sáng ăn mừng. |
| 12 | `aquarium` | `fish`, `fishing`, `goldfish`, `fishbowl` | **Bể cá vàng** *(Mới)* | Góc nhìn ngang qua bể kính ngập nắng với rạn cát vàng ươm uốn ôm biểu tượng, rong rêu đung đưa, bọt khí và đàn cá vàng tung tăng. **Click:** Bấm vùng nước để thả dây câu cá vàng (canh lúc phao giật mạnh hiện dấu `!` rồi click để kéo cá lên); bấm vào rạn cát để tung làn bụi cát lấp lánh. |
| 13 | `commute` | `bus`, `evening-bus`, `commute-bus` | **Chuyến xe chiều** *(Mới)* | Phong cảnh hoạt hình ven sông theo giờ thực, cầu vòm, phố lên đèn và xe buýt vàng chạy qua đồng lau. **Click:** Bấm mặt đường gọi xe dừng đón, bấm lên trời gọi chim / sao băng, bấm đồng cỏ tạo luồng gió thổi rạp lau. |
| 14 | `summer-porch` | `porch`, `corgi`, `corgi-porch`, `veranda` | **Hiên nhà ngày hạ** *(Mới)* | Hiên gỗ mái ngói âm dương kiểu Ghibli, đàn Corgi ngủ trưa lăn mình trước quạt bàn cổ điển, đĩa dưa hấu đỏ, sách mở và luống vườn xanh ôm quanh icon. **Click:** Bấm Corgi để gãi bụng bắn tim hồng, bấm quạt bàn đổi nấc gió, gõ chuông gió đồng, cắn dưa hấu hoặc thả bướm / đom đóm trên sân. |
| 15 | `cats` | `kittens`, `hillside-kittens` | **Mèo con trên đồi** *(Mới)* | Đồi cỏ trời xanh mây trắng, cây lá đung đưa, lá bay, bướm vàng và đàn mèo mập chân ngắn (10 bộ lông) đi dạo, liếm lông, ngủ cuộn tròn / nằm nghiêng / nằm ngửa thở phập phồng. Vẽ bằng **tiny-skia trên CPU** (`src/themes/cats/`, mỗi file `.rs` ứng với một file `prototype/cats-*.js`) thay vì shader, để giống hệt bản Canvas 2D. **Click:** Bấm mèo để vuốt ve (nhảy cẫng, chùm tim, mắt cười); bấm cây / lùm cây để rung rụng lá; bấm cỏ thả **cuộn len** cho 3 chú mèo gần nhất chạy tới vờn, khều lăn đi rồi đuổi theo; bấm trời gọi bướm. |

> **Bật / tắt tương tác chuột:** Có thể bật hoặc tắt bất kỳ lúc nào ngay tại menu chuột phải ở khay hệ thống (**Tương tác chuột trên màn hình nền**) hoặc trong cửa sổ **Cài đặt**.

---

## Biên dịch & Chạy

Yêu cầu: **Rust toolchain (`rustup`, MSVC target `x86_64-pc-windows-msvc`)** và **Windows 10/11 SDK** (có sẵn khi cài Visual Studio Build Tools C++). Không cần kết nối mạng nếu cache Cargo đã sẵn sàng; trình biên dịch tự tìm `rc.exe` của Windows SDK để nhúng icon và manifest.

**Máy không cài được MSVC Build Tools** (cần quyền quản trị): build bằng bộ công cụ GNU của Rust. `build.rs` nhúng
`app.manifest` bằng crate `embed-manifest` và biên dịch `app.rc` (icon, VERSIONINFO) bằng `windres`; cần MinGW trong
`PATH` (ví dụ `winget install BrechtSanders.WinLibs.POSIX.UCRT --scope user`), rồi chép `WebView2Loader.dll`
(trong `target/release/build/webview2-com-sys-*/out/x64/`) ra cạnh exe để mở được cửa sổ Cài đặt. Cảnh báo
`.rsrc merge failure: multiple non-default manifests` là vô hại: manifest của dự án vẫn là tài nguyên số 1.

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu
cargo +stable-x86_64-pc-windows-gnu build --release
```

```powershell
cd deskland-rs
cargo build --release

# Chạy hình nền (mặc định đọc chủ đề lần trước hoặc beach, 20 FPS cơ sở)
.\target\release\deskland-rs.exe

# Hoặc chỉ định chủ đề / tốc độ khung hình ngay từ dòng lệnh
.\target\release\deskland-rs.exe --theme stadium --fps 24

# Mở cửa sổ Cài đặt của tiến trình đang chạy (hoặc khởi động kèm Cài đặt)
.\target\release\deskland-rs.exe --settings

# Mở thẳng cửa sổ Cài đặt Win32 thuần (bỏ qua WebView2)
.\target\release\deskland-rs.exe --settings-win32

# Thoát tiến trình đang chạy nền
.\target\release\deskland-rs.exe --quit
```

### Đầy đủ tham số dòng lệnh (CLI)

| Tham số | Chức năng |
|---|---|
| `--theme <id>` | Khởi chạy với chủ đề chỉ định (hỗ trợ cả mã chính thức lẫn alias trong bảng trên). |
| `--fps <số>` | Đặt tốc độ khung hình cơ sở khi nghỉ (mặc định `20`; tự nâng lên `50` khi đang tương tác). |
| `--settings` | Mở cửa sổ Cài đặt WebView2 (tự lùi về Win32 nếu thiếu WebView2/prototype). |
| `--settings-win32` | Buộc mở giao diện Cài đặt Win32 cổ điển siêu nhẹ. |
| `--saver` (hoặc `/s`) | Chạy thử ngay chế độ trình bảo vệ màn hình toàn màn hình. |
| `--quit` | Gửi tín hiệu đóng an toàn tới tiến trình `deskland-rs.exe` đang chạy và dọn icon khay. |
| `--snapshot <thư_mục>` | Xuất ảnh chụp khung hình GPU (`.bmp`) ra thư mục chỉ định sau 4 giây. |
| `--exit-after-snapshot` | Tự động thoát ngay sau khi chụp xong `--snapshot` (dùng cho kiểm thử tự động). |
| `--test-click` | Tự động mô phỏng thao tác click trên cảnh để kiểm thử hiệu ứng tương tác. |
| `--headless` | Dựng cảnh ẩn ngoài màn hình (không gắn vào `Progman`/`WorkerW`, không tạo icon khay). |
| `--present gdi\|gl` | Chọn phương thức đẩy hình lên cửa sổ nền (`gdi` mặc định tương thích Windows 11 24H2+; `gl` dùng `SwapBuffers`). |
| `--webview-debug` | Mở cổng Remote Debugging `9222` cho cửa sổ cài đặt WebView2. |

- **Tệp cấu hình:** Lưu độc lập tại `~/.deskland-rs/settings.json` (lần chạy đầu tiên nếu chưa có sẽ tự kế thừa cấu hình từ `%LOCALAPPDATA%\DesklandPreview\settings.json` của bản C# mà không ghi đè dữ liệu cũ).
- **Nhật ký hoạt động:** Ghi tại `deskland-rs.log` cạnh file `.exe` (hoặc `~/.deskland-rs/deskland-saver.log` ở chế độ `.scr`), tự in thống kê RAM và tốc độ khung hình mỗi 10 giây.

---

## Kiến trúc Hiển thị & Tương thích Windows 11 24H2+

- **Tự động nhận diện mô hình Desktop:** Hỗ trợ cả kiến trúc **Raised Desktop (`Progman` + cửa sổ con phân lớp `WS_EX_LAYERED`)** trên Windows 11 24H2+ lẫn kiến trúc **`WorkerW` cổ điển** trên Windows 10 / 11 23H2, hỗ trợ đa màn hình khác DPI (`Per-Monitor DPI Aware V2`) và tự dựng lại khi cắm/rút màn hình (`WM_DISPLAYCHANGE`).
- **Đường ống kết xuất (`--present gdi` mặc định):**
  1. OpenGL 3.3 vẽ shader vào FBO ở độ phân giải tối ưu **72%** (giống bản gốc để giảm tải fill-rate GPU tích hợp).
  2. `glBlitFramebuffer(..., GL_LINEAR)` phóng mượt trên GPU lên kích thước thật của từng màn hình.
  3. Đọc bất đồng bộ qua **PBO (Pixel Buffer Object)** không gây nghẽn pipeline GPU → đẩy thẳng lên cửa sổ hình nền bằng `SetDIBitsToDevice`.
  4. Giải quyết triệt để lỗi màn hình đen của `SwapBuffers` trên cửa sổ con `WS_EX_LAYERED` của Windows 11 24H2+.

---

## Hiệu năng Đo Thực Tế

1. **So sánh trực tiếp với bản C# + WebView2** *(Intel UHD 770, 2 màn hình `1920×1080`)*:

| Phiên bản / Cấu hình | Số tiến trình | Private RAM | Working Set | GPU Shared | CPU (1 nhân) |
|---|---:|---:|---:|---:|---:|
| Bản C# + WebView2 (`beach`) | 11 | ~620 MB | ~1.210 MB | ~250 MB | ~15% |
| Bản C# + WebView2 (`meadow`) | 11 | ~650 MB | ~1.240 MB | ~266 MB | ~20% |
| **Bản Rust (`deskland-rs`, `--present gdi` 2 màn Full HD)** | **1** | **~97,7 MB** | **~124,2 MB** | **~97,9 MB** | **~14%** |

2. **Đo thực tế trên 1 màn hình Full HD với chủ đề phức tạp (`stadium` — 12 cầu thủ AI + khán đài + bóng)** *(chạy nền hàng ngày)*:
   - **Private RAM:** `~64,9 MB` (`Working Set ~95,4 MB`).
   - **CPU trung bình khi nghỉ (20 FPS):** `~0,7%` tổng toàn hệ thống (`~1,22 ms` thời gian CPU mỗi khung hình).
   - **Khi khóa máy hoặc mở ứng dụng toàn màn hình:** **`0,0% CPU`**, **`0 FPS`**.

---

## Khác biệt chủ đích so với bản C# / Web

- Không port các lớp ảnh PNG/Canvas phụ của 2 chủ đề đầu (`beach-palms.js`, `meadow-boy.js`, `meadow-cat.js`) và tính năng xuất GIF/PNG chia sẻ mạng xã hội (`share.js`); giao diện Cài đặt tự động ẩn các tùy chọn này qua cờ `caps` để đồng nhất với cảnh thực tế trên Desktop.
- Không vẽ đè đồng hồ Glassmorphism giả lập lên màn hình khóa và không chặn `Win+L`, đảm bảo an toàn bảo mật tuyệt đối thông qua màn hình đăng nhập thật của Windows (`Winlogon` / Windows Hello / PIN hệ thống).
