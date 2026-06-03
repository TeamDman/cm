use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=resources");

    #[cfg(windows)]
    // Reactor's self-contained setup owns the embedded app manifest. It injects
    // the linker arguments needed for WinUI/bootstrap support, so we avoid
    // duplicating manifest ownership in app.rc.
    windows_reactor_setup::as_self_contained();

    // app.rc currently contributes only non-manifest resources such as the icon.
    // Keeping this optional prevents duplicate MANIFEST resources now that
    // windows_reactor_setup embeds the required manifest for Windows builds.
    embed_resource::compile("resources/app.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("failed to embed resources");

    // Try to get a short git revision; on failure, set to "unknown".
    let rev = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(o.stdout)
            } else {
                None
            }
        })
        .and_then(|v| String::from_utf8(v).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string());

    println!("cargo:rustc-env=GIT_REVISION={rev}");
}
