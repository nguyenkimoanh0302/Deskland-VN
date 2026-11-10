# Phụ thuộc bên thứ ba / Third-party dependencies

## Microsoft Edge WebView2

Trình chủ Windows sử dụng Microsoft Edge WebView2 SDK và WebView2 Runtime. Kịch bản biên dịch tải thư viện phát triển từ NuGet, kho mã nguồn này không lưu trữ tệp nhị phân của SDK.

- Gói: `Microsoft.Web.WebView2`, phiên bản `1.0.4191.47`.
- Thông tin gói và giấy phép: https://www.nuget.org/packages/Microsoft.Web.WebView2/1.0.4191.47
- Tài liệu: https://learn.microsoft.com/microsoft-edge/webview2/

Các thành phần WebView2 và Windows / .NET Framework áp dụng điều khoản cấp phép riêng của từng thành phần và không thay đổi bởi giấy phép GPL-3.0 của dự án này.

The host uses the Microsoft Edge WebView2 SDK and Runtime. SDK binaries are downloaded by the build script and are not checked into this repository. Microsoft components retain their respective licenses.

## NSIS

Bộ cài đặt Windows được đóng gói bằng NSIS 3.13. Trình biên dịch di động chỉ được tải về khi đóng gói và không thuộc mã nguồn dự án. NSIS cùng các mô-đun nén áp dụng giấy phép riêng tại https://nsis.sourceforge.io/License. Bộ cài đặt đi kèm tệp ZIP chứa mã nguồn tương ứng.

The Windows installer is built with NSIS 3.13. The portable compiler is downloaded only at build time and is not checked in as project source code. NSIS and its compression modules retain their respective licenses at https://nsis.sourceforge.io/License. The installer includes a ZIP archive of the corresponding source code.
