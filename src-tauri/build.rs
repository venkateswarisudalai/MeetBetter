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
    let mut from_file = Vec::new();
    let Ok(contents) = std::fs::read_to_string(path) else {
        default_missing_build_env(&from_file);
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
        from_file.push(key.to_string());
        println!("cargo:rustc-env={}={}", key, value);
    }
    default_missing_build_env(&from_file);
}

/// Keys calendar.rs and supabase.rs read with `env!()`. Without them the build still succeeds
/// (CI, fresh clones); Google Calendar and cloud sync just stay unavailable in that build.
const BUILD_KEYS: [&str; 4] = ["GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET", "SUPABASE_URL", "SUPABASE_ANON_KEY"];

fn default_missing_build_env(from_file: &[String]) {
    for key in BUILD_KEYS {
        println!("cargo:rerun-if-env-changed={}", key);
        if std::env::var_os(key).is_none() && !from_file.iter().any(|k| k == key) {
            println!("cargo:warning={} not set (.env.build or env): Google Calendar / cloud sync disabled in this build", key);
            println!("cargo:rustc-env={}=", key);
        }
    }
}
