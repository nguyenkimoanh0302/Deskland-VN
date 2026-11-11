// Nhúng app.manifest + tài nguyên app.rc (app.ico & VERSIONINFO) vào exe (trình liên kết MSVC).
// Không có manifest thì cửa sổ con phân lớp của chế độ 24H2+ không hiện lên màn hình (desktop đen).
use std::path::{Path, PathBuf};
use std::process::Command;

fn find_rc_exe() -> Option<PathBuf> {
    if Command::new("rc.exe").arg("/?").output().is_ok() {
        return Some(PathBuf::from("rc.exe"));
    }
    let pf86 = std::env::var_os("ProgramFiles(x86)").unwrap_or_else(|| r"C:\Program Files (x86)".into());
    let kits_bin = Path::new(&pf86).join("Windows Kits").join("10").join("bin");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(&kits_bin)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    // Ưu tiên phiên bản Windows SDK mới nhất
    versions.sort_by(|a, b| b.cmp(a));
    for ver in versions {
        for arch in ["x64", "arm64", "x86"] {
            let cand = ver.join(arch).join("rc.exe");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=assets/app.ico");

    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") && target.contains("msvc") {
        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let manifest_path = manifest_dir.join("app.manifest");
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", manifest_path.display());

        let rc_exe = find_rc_exe().expect("Không tìm thấy rc.exe của Windows SDK để nhúng assets/app.ico");
        let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let res_out = out_dir.join("app.res");
        let rc_in = manifest_dir.join("app.rc");
        let status = Command::new(&rc_exe)
            .current_dir(&manifest_dir)
            .arg("/nologo")
            .arg("/fo")
            .arg(&res_out)
            .arg(&rc_in)
            .status()
            .expect("Không chạy được rc.exe để biên dịch app.rc");
        assert!(status.success(), "rc.exe biên dịch app.rc thất bại (mã thoát: {:?})", status.code());
        println!("cargo:rustc-link-arg-bins={}", res_out.display());
    }
}
