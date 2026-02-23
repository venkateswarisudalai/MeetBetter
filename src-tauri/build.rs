fn main() {
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
            }
        }
    }

    tauri_build::build()
}
