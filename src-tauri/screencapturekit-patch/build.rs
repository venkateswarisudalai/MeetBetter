use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rustc-link-lib=framework=ScreenCaptureKit");

    // Build the Swift bridge
    let swift_dir = "swift-bridge";
    let out_dir = env::var("OUT_DIR").unwrap();
    let swift_build_dir = format!("{out_dir}/swift-build");

    println!("cargo:rerun-if-changed={swift_dir}");

    // Run swiftlint if available (non-strict mode, don't fail build)
    if let Ok(output) = Command::new("swiftlint")
        .args(["lint"])
        .current_dir(swift_dir)
        .output()
    {
        if !output.status.success() {
            eprintln!(
                "SwiftLint warnings:\n{}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
    }

    // Build Swift package with build directory in OUT_DIR
    // Determine the Swift triple from the Cargo TARGET to support cross-compilation
    let target = env::var("TARGET").unwrap_or_default();
    let swift_triple = if target.contains("x86_64") {
        Some("x86_64-apple-macosx12.3")
    } else if target.contains("aarch64") {
        Some("arm64-apple-macosx12.3")
    } else {
        None
    };

    let mut swift_args = vec![
        "build".to_string(),
        "-c".to_string(),
        "release".to_string(),
        "--package-path".to_string(),
        swift_dir.to_string(),
        "--scratch-path".to_string(),
        swift_build_dir.clone(),
    ];

    if let Some(triple) = swift_triple {
        swift_args.push("--triple".to_string());
        swift_args.push(triple.to_string());
    }

    let output = Command::new("swift")
        .args(&swift_args)
        .output()
        .expect("Failed to build Swift bridge");

    // Swift build outputs warnings to stderr even on success, check exit code only
    if !output.status.success() {
        eprintln!(
            "Swift build STDOUT:\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
        eprintln!(
            "Swift build STDERR:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        panic!(
            "Swift build failed with exit code: {:?}",
            output.status.code()
        );
    }

    // Link the Swift library from OUT_DIR
    println!("cargo:rustc-link-search=native={swift_build_dir}/release");
    println!("cargo:rustc-link-lib=static=ScreenCaptureKitBridge");

    // Link required frameworks
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=CoreGraphics");
    println!("cargo:rustc-link-lib=framework=CoreMedia");
    println!("cargo:rustc-link-lib=framework=IOSurface");

    // Add rpath for Swift runtime libraries
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");

    // Add rpath for Xcode Swift runtime (needed for Swift Concurrency)
    if let Ok(output) = Command::new("xcode-select").arg("-p").output() {
        if output.status.success() {
            let xcode_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let swift_lib_path = format!(
                "{xcode_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift-5.5/macosx"
            );
            println!("cargo:rustc-link-arg=-Wl,-rpath,{swift_lib_path}");
            // Also add the newer swift path
            let swift_lib_path_new =
                format!("{xcode_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx");
            println!("cargo:rustc-link-arg=-Wl,-rpath,{swift_lib_path_new}");
        }
    }
}
