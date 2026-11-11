# Deskland Preview (Bản mở rộng Tiếng Việt / Vietnamese Extended Edition)

**Hình nền động thích ứng cho Windows — biến các biểu tượng trên màn hình nền thành một phần của phong cảnh.**

Deskland biến không gian quanh các biểu tượng màn hình nền thành một khung cảnh sống động. Khi bạn di chuyển, thêm hoặc xóa biểu tượng, bờ biển, đồng cỏ, đám mây, rạn cát, tán cây và mặt nước sẽ tự động uốn lượn bao quanh theo thời gian thực ngay trên máy — không dùng AI, không cần tài khoản hay API key.

> **Nguồn gốc & Giấy phép (Attribution & License):**
> Dự án này là bản phái sinh (fork/modified version) dựa trên dự án mã nguồn mở gốc **[Deskland](https://github.com/BLEZZ0102/Deskland)** (`Copyright (C) 2026 Deskland authors`, trang chủ: [deskland.app](https://deskland.app)), được phát hành và tiếp tục phân phối theo giấy phép **[GNU General Public License v3.0 (GPL-3.0)](LICENSE)**.

**Tiếng Việt** · [English](#english-overview) · [Bản Rust Native (`deskland-rs`)](deskland-rs/README.md) · [Dự án gốc (Upstream)](https://github.com/BLEZZ0102/Deskland) · [Cách biên dịch](#biên-dịch-từ-mã-nguồn) · [Giấy phép](#bản-quyền-và-giấy-phép)

## Tải về & chạy ngay (không cần cài đặt gì)

1. Vào trang **[Releases](https://github.com/nhhoang188/Deskland-VN/releases/latest)**, tải `Deskland-VN-<phiên bản>-win-x64.zip`.
2. Giải nén vào một thư mục cố định (ví dụ `Documents\Deskland-VN`). Giữ `Deskland-VN.exe` nằm cạnh thư mục `prototype` để có trang Cài đặt đầy đủ (lưới chủ đề, xem trước trực tiếp).
3. Chạy `Deskland-VN.exe`, rồi bấm chuột trái vào biểu tượng ở khay hệ thống để chọn chủ đề.

Lưu ý:
- **Không cần** cài Rust, .NET hay Visual C++ Redistributable. Chỉ cần Windows 10/11 64-bit.
- Tệp chưa có chữ ký số nên lần đầu Windows SmartScreen có thể báo *"Windows protected your PC"*: bấm **More info → Run anyway**. Bản phát hành được build tự động trên GitHub Actions từ đúng mã nguồn của tag (kèm mã SHA-256 để đối chiếu).
- Ứng dụng chỉ ghi vào tài khoản của bạn: cài đặt ở `~/.deskland-rs`, mục *Khởi động cùng Windows* (khi bạn bật), trình bảo vệ màn hình và ảnh màn khóa (khi bạn bật trong Cài đặt). Một số phần mềm diệt virus có thể hỏi lại vì các hành vi này.
- *Khởi động cùng Windows* ghi đường dẫn đầy đủ của exe. Nếu chuyển thư mục đi nơi khác, hãy tắt rồi bật lại mục này.
- Máy Windows 10 chưa có WebView2 Runtime vẫn chạy bình thường, chỉ là Cài đặt mở bằng cửa sổ Win32 đơn giản.

**Muốn tạo chủ đề của riêng bạn?** Xem **[CONTRIBUTING.md](CONTRIBUTING.md)**: một shader dùng chung cho bản web và bản Rust, có mẫu sẵn và script kiểm tra.

![Đồng cỏ — Meadow](docs/images/meadow.webp)

![Bãi biển — Beach](docs/images/beach.webp)

![Maldives](docs/images/maldives.webp)

---

## Mới: Phiên bản Rust Native (`deskland-rs`) — Siêu nhẹ, Tương tác Chuột & 12 Chủ đề

Bên cạnh trình chủ C# + WebView2 ban đầu (`host/`), kho mã nguồn nay bổ sung **phiên bản viết lại hoàn toàn bằng Rust thuần + OpenGL 3.3 (`deskland-rs/`)** với những cải tiến vượt trội cho sử dụng hàng ngày trên Windows 10 & Windows 11 (kể cả **24H2+**):

- **1 tiến trình duy nhất — Giảm ~6–8 lần RAM:** Gộp toàn bộ bộ dựng hình OpenGL 3.3 vào một tệp `deskland-rs.exe` (~2 MB), chỉ tiêu thụ **~65–98 MB RAM** (1 tiến trình) thay vì ~620–790 MB RAM (11 tiến trình WebView2 chạy ngầm liên tục). Khởi động lên hình trong **~0,4 giây**.
- **Tương tác Click chuột trực tiếp trên màn hình nền & Adaptive FPS (`20 -> 50 FPS`):** Click chuột trái vào khoảng trống trên Desktop để tương tác sống động với từng cảnh (thả mồi gọi cá, quăng cần câu cá vàng/cá hồ, vuốt ve mèo hoặc thả bướm, chuyền bóng & sút ghi bàn trên sân vận động, rung cành hoa mận/hoa đào, gọi sao băng ban đêm...). Tự động bỏ qua vùng biểu tượng (icon), nâng lên **50 FPS** mượt mà khi có tương tác và hạ về **20 FPS** tiết kiệm khi nghỉ (có thể bật/tắt ngay trên khay hệ thống hoặc Cài đặt).
- **Mở rộng lên 12 chủ đề hoàn chỉnh:** Bổ sung thêm 2 chủ đề mới giàu tính tương tác là **Sân bóng sôi động (`stadium`)** và **Bể cá vàng (`aquarium`)** bên cạnh 10 chủ đề hiện có.
- **Màn hình khóa thật 100% của Windows & `0% CPU` khi nghỉ:** Chuyển sang sử dụng cơ chế khóa chuẩn hệ điều hành (`LockWorkStation()`, trình bảo vệ màn hình cảnh động `.scr` có `ScreenSaverIsSecure=1`, phím tắt `Ctrl+Alt+L`, đồng bộ ảnh nền màn khóa Windows, giữ nguyên `Win+L` gốc của hệ thống) và tự động ngừng vẽ về **`0% CPU`** khi khóa máy hoặc mở ứng dụng toàn màn hình.
- **Cài đặt On-Demand & Khay hệ thống tinh gọn:** Giao diện Cài đặt `prototype/settings.html` chỉ nạp WebView2 khi mở (đóng là giải phóng sạch RAM, có sẵn giao diện Win32 thuần dự phòng), nhúng sẵn biểu tượng `.exe` đa kích thước sắc nét và menu chuột phải khay hệ thống gọn nhẹ chỉ hiển thị chủ đề đang chọn.

👉 Xem tài liệu chi tiết về kiến trúc, bảng tương tác của cả 12 chủ đề và tham số dòng lệnh tại **[deskland-rs/README.md](deskland-rs/README.md)**.

---

## Các điểm cải tiến trong bản mở rộng này (Modified on 2026-11-11)

Theo Mục 5a của giấy phép GNU GPL v3.0, bản sửa đổi này bao gồm các thay đổi sau so với bản gốc:

1. **12 chủ đề hoàn chỉnh (loại bỏ toàn bộ các thẻ chủ đề khóa/phải mua):**
   - **Thảo nguyên (`meadow`):** Mây trôi và bóng mây trên đồng cỏ uốn quanh biểu tượng; click tạo cụm mây puff bay lên bầu trời.
   - **Bãi biển (`beach` / `flat.html`):** Sóng biển ngọc lam, thuyền kayak và hải âu; click tạo gợn nước và gọi kayak/hải âu.
   - **Đảo Maldives (`beach-turquoise` / `maldives.html`):** Biển ngọc bích trong vắt, rạn san hô, đàn cá nhiệt đới và rùa biển; click thả mồi thu hút cá & rùa.
   - **Hồ nước (`lake`):** Nước hồ xanh tĩnh lặng, lá súng, hoa súng nở và cá Koi; click mặt hồ để quăng phao câu cá thư giãn.
   - **Bầu trời (`sky`):** Biển mây bồng bềnh theo chu kỳ Ngày / Hoàng hôn / Đêm; click tạo mây & gọi chim ban ngày hoặc phóng sao băng ban đêm.
   - **Đại dương (`ocean`):** Đảo mây giữa đại dương xanh thẫm, thuyền buồm trắng & du thuyền; click mặt biển tạo sóng dẫn đường cho tàu thuyền.
   - **Rừng mưa nhiệt đới (`rainforest`):** Tán rừng nhiệt đới đa tầng, mưa rơi, đom đóm và chim rừng; click tụ đàn đom đóm phát sáng & gợn nước mưa.
   - **Hoa mận (`mocchau`):** Đồi cỏ xuân và rừng hoa mận trắng Mộc Châu trong sương; click rung cành bung cánh hoa trắng bay theo gió.
   - **Hoa anh đào Tết (`tet-sakura`):** Sân xuân giấy dó với hoa đào hồng thắm, đèn lồng đỏ, bao lì xì và én xuân; click đung đưa đèn lồng & tung cánh đào.
   - **Đồi mèo nắng ấm (`cat-hill`):** Đồi cỏ xanh nắng ấm cùng đàn mèo mập đi dạo, liếm lông và nằm ngủ; click vuốt ve mèo hoặc thả bướm vàng trên cỏ.
   - **Sân bóng sôi động (`stadium` — Mới):** Sân vận động bóng đá nhìn từ trên cao (Ngày / Hoàng hôn / Đèn pha đêm) với hai đội Đỏ – Xanh phối hợp chuyền bóng; click trên sân để chuyền bóng cho cầu thủ đuổi theo hoặc click cầu môn để sút ghi bàn ăn mừng.
   - **Bể cá vàng (`aquarium` — Mới):** Góc nhìn ngang qua bể kính ngập nắng với rạn cát vàng ôm biểu tượng, rong rêu, bọt khí và đàn cá vàng; click vùng nước thả câu cá vàng hoặc click rạn cát tung bụi cát.
   - **Chuyến xe chiều (`commute` — Mới):** Phong cảnh hoạt hình ven sông, trời đổi màu theo giờ trên máy (ngày / hoàng hôn / đêm), phố lên đèn, cầu vòm, xe buýt vàng chạy qua đồng lau; click vào đường để gọi xe dừng, lên trời để có đàn chim hoặc sao băng, vào cỏ để gió thổi rạp lau.
2. **Phiên bản Rust Native (`deskland-rs`):** Bổ sung bản thực thi gốc Rust + OpenGL 3.3 siêu nhẹ (1 tiến trình, giảm ~6–8 lần RAM, hỗ trợ tương tác chuột Desktop, Adaptive FPS `20 -> 50 FPS`, biểu tượng `.exe` riêng, menu khay tinh gọn và khóa thật của Windows).
3. **Hỗ trợ đa màn hình (Multi-Monitor & Per-Monitor DPI):** Tự động nhận diện và hiển thị hình nền sắc nét trên tất cả các màn hình đang kết nối.
4. **Tối ưu hiệu năng GPU:** Giảm tải độ phân giải nội suy (`scale = 0.72`), kết xuất PBO bất đồng bộ và tự động ngừng vẽ khi màn hình nền bị che khuất hoặc khóa máy.
5. **Việt hóa toàn diện:** Hỗ trợ đầy đủ **Tiếng Việt (`vi`)** và **Tiếng Anh (`en`)** trên cả giao diện Cài đặt, khay hệ thống lẫn chú giải chủ đề.
6. **Tùy chọn Màn hình khóa linh hoạt:**
   - **Trên bản Rust (`deskland-rs`):** Sử dụng 100% khóa thật an toàn của Windows (`LockWorkStation()`, trình bảo vệ màn hình cảnh động `.scr` yêu cầu đăng nhập Windows khi quay lại, phím tắt `Ctrl+Alt+L` và đồng bộ ảnh màn khóa Windows 11).
   - **Trên bản C# (`host/`):** Hỗ trợ màn hình khóa động WebGL2 kèm Widget Đồng hồ Glassmorphism và đồng bộ ảnh nền màn khóa hệ thống.

---

## Yêu cầu hệ thống

- **Hệ điều hành:** Windows 10 hoặc Windows 11 (64-bit, tương thích đầy đủ Windows 11 24H2+).
- **Microsoft Edge WebView2 Runtime:** Có sẵn trên Windows 11 (bản Rust vẫn chạy tốt và có sẵn giao diện Cài đặt Win32 dự phòng ngay cả khi máy không có WebView2).
- **Để tự biên dịch từ mã nguồn:**
  - **Bản Rust (`deskland-rs` — khuyên dùng):** Rust (`cargo` / `rustup` MSVC) + Windows 10/11 SDK (Build Tools for Visual Studio). Exe build ra đã liên kết tĩnh thư viện C (`deskland-rs/.cargo/config.toml`), chép sang máy khác chạy được ngay.
  - **Bản C# (`host/`):** PowerShell và `.NET Framework csc.exe` có sẵn trên Windows (cần Internet ở lần build đầu để tải gói NuGet WebView2).
  - **Xem trước trình duyệt (tùy chọn):** Node.js.

## Biên dịch từ mã nguồn

### Cách 1: Bản Rust Native (`deskland-rs` — Khuyên dùng, nhẹ & đầy đủ 12 chủ đề tương tác)

```powershell
cd deskland-rs
cargo build --release
.\target\release\deskland-rs.exe
```

- Nhấp chuột trái vào biểu tượng Deskland trên khay hệ thống (System Tray) hoặc chạy `.\target\release\deskland-rs.exe --settings` để mở **Cài đặt**.
- Thoát chương trình từ menu chuột phải khay hệ thống hoặc chạy `.\target\release\deskland-rs.exe --quit`.
- Dữ liệu cài đặt lưu tại `~/.deskland-rs/settings.json`.

### Cách 2: Bản C# + WebView2 (`host/`)

Mở PowerShell tại thư mục gốc của dự án:

```powershell
powershell -ExecutionPolicy Bypass -File host\build.ps1
host\bin\Deskland.exe
```

Để thoát bản C# và khôi phục hình nền ban đầu, chọn **Thoát** ở khay hệ thống hoặc chạy `host\bin\Deskland.exe --exit`. Cấu hình bản C# lưu tại `%LOCALAPPDATA%\DesklandPreview`.

## Xem trước trên trình duyệt

```powershell
node prototype\serve.mjs
```

Mở **http://localhost:5178/settings.html** trên trình duyệt. Chế độ trình duyệt sử dụng các biểu tượng mẫu; tính năng thích ứng theo biểu tượng thật và tương tác chuột trên màn hình nền cần chạy qua `deskland-rs.exe` (hoặc `Deskland.exe`).

## Cấu trúc thư mục

| Đường dẫn | Nội dung |
| --- | --- |
| `deskland-rs/` | **Mã nguồn bản Rust Native** (Win32 + OpenGL 3.3, 12 chủ đề tương tác, Cài đặt WebView2/Win32, khóa Windows, icon `.ico`) |
| `host/` | Mã nguồn C# của trình chủ Windows (`DesktopHost.cs`, `Themes.cs`), kịch bản biên dịch và manifest |
| `prototype/meadow.html` | Chủ đề Thảo nguyên |
| `prototype/flat.html` | Chủ đề Bãi biển |
| `prototype/maldives.html` | Chủ đề Đảo Maldives |
| `prototype/lake.html` | Chủ đề Hồ nước |
| `prototype/sky.html` | Chủ đề Bầu trời |
| `prototype/ocean.html` | Chủ đề Đại dương |
| `prototype/rainforest.html` | Chủ đề Rừng mưa nhiệt đới |
| `prototype/mocchau.html` | Chủ đề Hoa mận Mộc Châu |
| `prototype/tet-sakura.html` | Chủ đề Hoa anh đào Tết |
| `prototype/cat-hill.html` | Chủ đề Đồi mèo nắng ấm |
| `prototype/stadium.html` | Chủ đề Sân bóng sôi động *(Mới)* |
| `prototype/aquarium.html` & `aquarium.frag` | Chủ đề Bể cá vàng & shader dùng chung *(Mới)* |
| `prototype/settings.html` | Giao diện Cài đặt chung (dùng cho cả bản Rust, bản C# và xem trước trình duyệt) |
| `prototype/themes.json` | Cấu hình tham số và nhóm tùy chỉnh của cả 12 chủ đề |
| `prototype/i18n/` | Các tệp ngôn ngữ Tiếng Việt (`vi.json`) và Tiếng Anh (`en.json`) |
| `prototype/share.js` | Công cụ xuất ảnh tĩnh và ảnh động bảo vệ quyền riêng tư (bản Web/C#) |
| `tools/` | Công cụ đo hiệu năng (`measure-ram.ps1`) và đóng gói bộ cài đặt NSIS |

## Bản quyền và giấy phép

- **Tác giả gốc (Upstream):** Copyright (C) 2026 Deskland authors — [https://github.com/BLEZZ0102/Deskland](https://github.com/BLEZZ0102/Deskland) ([deskland.app](https://deskland.app)).
- **Giấy phép:** Mã nguồn và tài nguyên chủ đề đi kèm được phát hành theo giấy phép **[GNU GPL v3.0](LICENSE)**. Xem thêm [NOTICE](NOTICE) và [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
- **Nhãn hiệu:** Tên gọi và biểu tượng Deskland không được cấp phép như nhãn hiệu thương mại; bản phân phối sửa đổi này là dự án cộng đồng độc lập, sử dụng biểu tượng ứng dụng thiết kế riêng và không phải bản phát hành chính thức của tác giả gốc. Xem chi tiết tại [TRADEMARKS.md](TRADEMARKS.md).

---

## English Overview

**Adaptive Windows wallpaper that makes your desktop icons part of the scenery — now featuring a lightweight Rust + OpenGL 3.3 Native Edition (`deskland-rs`).**

This repository is a community-modified Vietnamese & English edition of **[Deskland](https://github.com/BLEZZ0102/Deskland)** (`Copyright (C) 2026 Deskland authors`), distributed under the **[GNU General Public License v3.0 (GPL-3.0)](LICENSE)**.

### Summary of Modifications (GPL-3.0 Section 5a Notice)

- **New Rust Native Edition (`deskland-rs/`):**
  - Single ~2 MB native executable (`Win32 + OpenGL 3.3` via WGL) using **~65–98 MB Private RAM** in 1 process (6–8× less memory than the 11-process C# + WebView2 host) with ~0.4s startup and full Windows 11 24H2+ layered `Progman` compatibility.
  - **Interactive Desktop Clicks + Adaptive FPS (`20 -> 50 FPS`):** Non-intrusive Raw Input mouse click interactions on empty desktop space (ignoring desktop icons and app windows) with per-theme physics & animations, automatically boosting frame rate to 50 FPS during interactions and returning to 20 FPS when idle.
  - **100% Native Windows Lock Screen Security & `0% CPU` Idle Pause:** Uses real `LockWorkStation()`, secure `.scr` live screensaver (`ScreenSaverIsSecure=1`), `Ctrl+Alt+L` hotkey, and Windows Lock Screen wallpaper sync without keyboard hooks or `Win+L` interception; pauses rendering to `0% CPU` when locked or occluded.
  - **On-Demand Settings UI & Polished Tray UX:** Loads `prototype/settings.html` in WebView2 only while Settings is open (with a built-in native Win32 fallback window), ships a custom multi-size squircle `.exe` icon, and features a clean, compact System Tray menu showing the active theme.
- **12 complete open-source themes (removed all locked/paid preview cards):**
  - Original **Meadow** (`meadow`) & **Beach** (`beach`), plus **Maldives** (`beach-turquoise`), **Lake** (`lake`), **Sky** (`sky`), **Ocean** (`ocean`), **Rainforest** (`rainforest`), **Moc Chau Plum Blossoms** (`mocchau`), **Tet Peach Blossoms** (`tet-sakura`), **Sunny Cat Hill** (`cat-hill`), and new interactive themes: **Lively Football Stadium** (`stadium`), **Sunlit Goldfish Bowl** (`aquarium`) and **Evening Bus** (`commute`).
- **Multi-monitor & Per-Monitor DPI V2 support** across all connected displays.
- **Vietnamese (`vi`) & English (`en`) UI localization.**

### Download & Run

Download `Deskland-VN-<version>-win-x64.zip` from **[Releases](https://github.com/nhhoang188/Deskland-VN/releases/latest)**, extract it to a permanent folder and run `Deskland-VN.exe` (keep it next to the `prototype` folder for the full Settings page). No Rust, .NET or Visual C++ Redistributable required. The file is not code-signed, so SmartScreen may warn on first run (**More info → Run anyway**); releases are built by GitHub Actions from the tagged source and ship with a SHA-256 checksum.

Want to make your own theme? See **[CONTRIBUTING.md](CONTRIBUTING.md)**.

### Build & Run

**Option 1 — Rust Native Edition (Recommended):**

```powershell
cd deskland-rs
cargo build --release
.\target\release\deskland-rs.exe
```

**Option 2 — C# + WebView2 Host:**

```powershell
powershell -ExecutionPolicy Bypass -File host\build.ps1
host\bin\Deskland.exe
```

### Credits & License

- Original work: Copyright (C) 2026 Deskland authors ([https://github.com/BLEZZ0102/Deskland](https://github.com/BLEZZ0102/Deskland)).
- Licensed under [GPL-3.0](LICENSE). See [NOTICE](NOTICE), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), and [TRADEMARKS.md](TRADEMARKS.md).
