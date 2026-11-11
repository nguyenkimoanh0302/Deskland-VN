## Thay đổi

<!-- Mô tả ngắn: chủ đề mới / sửa lỗi / dịch / tối ưu... -->

## Chủ đề mới (nếu có)

- id: `...`
- Tương tác khi click:
- Ảnh / GIF (ngày, đêm nếu có):
- "CPU vẽ … ms/khung" trong `deskland-rs.log`:

## Kiểm tra

- [ ] `node tools/check-themes.mjs` không có lỗi
- [ ] `cargo build --release --locked` (thư mục `deskland-rs`) thành công
- [ ] Đã chạy thử bản web (`node prototype/serve.mjs`) và bản Rust (`--theme <id> --test-click`)
- [ ] Biểu tượng luôn dễ nhìn; khung xem trước trong Cài đặt hiển thị đúng
- [ ] Tác phẩm nguyên bản, không dựng lại nội dung có bản quyền; đồng ý phát hành theo GPL-3.0
- [ ] Không truy cập mạng, không hook bàn phím / chuột, không xử lý mật khẩu
