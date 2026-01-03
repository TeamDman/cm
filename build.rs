use std::process::Command;

fn main() {
    // Try to get a short git revision; on failure, set to "unknown".
    let rev = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(o.stdout) } else { None })
        .and_then(|v| String::from_utf8(v).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_REVISION={}", rev);
}
