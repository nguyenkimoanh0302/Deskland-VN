// Nhúng app.manifest vào exe (trình liên kết MSVC). Không có manifest thì cửa sổ con phân lớp
// của chế độ 24H2+ không hiện lên màn hình (desktop đen).
fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") && target.contains("msvc") {
        let path = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("app.manifest");
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", path.display());
    }
}
