fn main() {
    load_build_env();

    #[cfg(target_os = "macos")]
    {
        // Weak-link ScreenCaptureKit so the binary still loads on macOS < 13
        println!("cargo:rustc-link-arg=-Wl,-weak_framework,ScreenCaptureKit");

        // The screencapturekit crate uses Swift and needs the Swift Concurrency runtime.
        // When using CommandLineTools (not full Xcode), the runtime lives in a
        // non-standard path that the crate's own build.rs doesn't cover.
        if let Ok(output) = std::process::Command::new("xcode-select")
            .arg("-p")
            .output()
        {
            if output.status.success() {
                let dev_path = String::from_utf8_lossy(&output.stdout).trim().to_string();

                // CommandLineTools: /Library/Developer/CommandLineTools/usr/lib/swift-5.5/macosx
                let clt_swift_path = format!("{}/usr/lib/swift-5.5/macosx", dev_path);
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", clt_swift_path);

                // Also try the non-versioned path
                let clt_swift_path2 = format!("{}/usr/lib/swift/macosx", dev_path);
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", clt_swift_path2);

                // Xcode Toolchain path (when using full Xcode, the runtime is here)
                let toolchain_swift_path = format!("{}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift-5.5/macosx", dev_path);
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", toolchain_swift_path);

                let toolchain_swift_path2 = format!("{}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx", dev_path);
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", toolchain_swift_path2);
            }
        }
    }

    tauri_build::build()
}

// Forwards keys from ../.env.build into compile-time env vars so `env!()` calls
// (calendar.rs, supabase.rs) resolve without the developer having to source the
// file in their shell. Caller-provided env still wins.
fn load_build_env() {
    let path = std::path::Path::new("../.env.build");
    println!("cargo:rerun-if-changed=../.env.build");
    let Ok(contents) = std::fs::read_to_string(path) else {
        return;
    };
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if std::env::var_os(key).is_some() {
            continue;
        }
        println!("cargo:rustc-env={}={}", key, value);
    }
}
