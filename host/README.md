# Trình chủ Windows của Deskland (Deskland Windows Host)

Sử dụng C# / .NET Framework và WebView2 để gắn trang chủ đề WebGL2 nằm ngay dưới lớp biểu tượng trên màn hình nền Windows.

## Biên dịch và chạy

```powershell
powershell -ExecutionPolicy Bypass -File host\build.ps1
host\bin\Deskland.exe
```

Kịch bản biên dịch sử dụng trình biên dịch C# có sẵn của Windows, lần đầu tiên sẽ tải thư viện phát triển WebView2 cố định phiên bản từ NuGet. Thư viện phụ thuộc được lưu trong `host/lib/`, tệp thực thi xuất ra tại `host/bin/` (cả hai thư mục này không đẩy lên kho mã nguồn).

## Tham số dòng lệnh

| Tham số | Hành vi |
| --- | --- |
| Không tham số | Khởi chạy hình nền và mở Cài đặt; nếu đang chạy thì mở Cài đặt |
| `--autostart` | Khởi chạy ngầm, không bật cửa sổ Cài đặt |
| `--settings` | Mở cửa sổ Cài đặt |
| `--menu` | Mở menu khay hệ thống |
| `--exit` | Thoát chương trình và khôi phục hình nền hệ thống |
| `--exit-installed` | Dùng cho bộ cài đặt: chỉ thoát tiến trình đang chạy từ đúng đường dẫn hiện tại |

## Hoạt động và dữ liệu

- Danh sách chủ đề được nạp từ `prototype/themes.json` (gồm 9 chủ đề hoàn chỉnh ở trạng thái `ready`).
- Kiểm tra vị trí các biểu tượng màn hình nền mỗi 400ms; chỉ gửi thông báo cho trang chủ đề khi vị trí thay đổi.
- Tự động tạm dừng khi có cửa sổ phóng to (maximized) hoặc toàn màn hình (full-screen); có thể tạm dừng/tiếp tục thủ công từ khay hệ thống.
- Trên Windows 11 24H2 trở lên sử dụng cửa sổ con phân lớp dưới `Progman`; trên các phiên bản cũ hơn sử dụng `WorkerW`.
- Hỗ trợ hiển thị trên đa màn hình (`Screen.AllScreens`).
- Cài đặt được lưu tại `%LOCALAPPDATA%\DesklandPreview\settings.json`, nhật ký tại `%LOCALAPPDATA%\DesklandPreview\host.log`.
- Tính năng chia sẻ chỉ dùng tọa độ khung chữ nhật của biểu tượng để vẽ biểu tượng mẫu và khối màu, không chụp ảnh màn hình thật.

## Giao tiếp với trang Web

- Trang Cài đặt (`settings.html`) gửi các thông điệp `hello`, `set`, `reset`, `theme`, `lang`, `autostart`, và các thông điệp chia sẻ; trình chủ trả về dữ liệu khởi tạo, danh sách hình chữ nhật biểu tượng và kết quả xuất tệp.
- Sau khi trang chủ đề gửi `ready`, trình chủ truyền cài đặt và vị trí biểu tượng, sau đó cập nhật khi tham số, biểu tượng hoặc trạng thái tạm dừng thay đổi.
