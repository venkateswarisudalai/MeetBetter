//! Screen Share Exclusion Module
//!
//! This module provides functionality to hide the app window during screen sharing.
//! - macOS: Uses NSWindow.sharingType = .none (macOS 12.0+)
//! - Windows: Uses SetWindowDisplayAffinity API
//! - Linux: Not supported natively

#[allow(unused_imports)]
use tauri::Window;

/// Sets whether the window should be excluded from screen capture/sharing
///
/// # Arguments
/// * `window` - The Tauri window to configure
/// * `exclude` - true to hide from screen share, false to show
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` with error message on failure
#[cfg(target_os = "macos")]
pub fn set_screen_share_exclusion(window: &Window, exclude: bool) -> Result<(), String> {
    use objc::runtime::YES;
    use objc::{msg_send, sel, sel_impl};

    // Get the native NSWindow handle
    let ns_window = window
        .ns_window()
        .map_err(|e| format!("Failed to get NSWindow: {}", e))?;

    unsafe {
        // NSWindowSharingType values:
        // 0 = NSWindowSharingNone - Window is not visible in screen capture
        // 1 = NSWindowSharingReadOnly - Window is visible in screen capture (default)
        let sharing_type: i64 = if exclude { 0 } else { 1 };

        // Call setSharingType: on the NSWindow
        let _: () = msg_send![ns_window as cocoa::base::id, setSharingType: sharing_type];
    }

    eprintln!("Screen share exclusion set to: {}", exclude);
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn set_screen_share_exclusion(window: &Window, exclude: bool) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    };

    // Get the native HWND handle
    let hwnd = window
        .hwnd()
        .map_err(|e| format!("Failed to get HWND: {}", e))?;

    let affinity = if exclude {
        WDA_EXCLUDEFROMCAPTURE // Exclude from screen capture (Windows 10 2004+)
    } else {
        WDA_NONE // Normal behavior
    };

    unsafe {
        SetWindowDisplayAffinity(HWND(hwnd.0 as *mut std::ffi::c_void), affinity)
            .map_err(|e| format!("SetWindowDisplayAffinity failed: {}", e))?;
    }

    eprintln!("Screen share exclusion set to: {}", exclude);
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn set_screen_share_exclusion(_window: &Window, exclude: bool) -> Result<(), String> {
    // Linux doesn't have native support for this feature
    // Some Wayland compositors might support it in the future
    if exclude {
        eprintln!("Warning: Screen share exclusion is not supported on Linux");
        Err("Screen share exclusion is not supported on Linux".to_string())
    } else {
        Ok(())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn set_screen_share_exclusion(_window: &Window, _exclude: bool) -> Result<(), String> {
    Err("Screen share exclusion is not supported on this platform".to_string())
}

/// Check if screen share exclusion is supported on this platform
pub fn is_supported() -> bool {
    cfg!(any(target_os = "macos", target_os = "windows"))
}

/// Get platform-specific information about screen share exclusion
pub fn get_platform_info() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        return "macOS: Supported (requires macOS 12.0+)";
    }

    #[cfg(target_os = "windows")]
    {
        return "Windows: Supported (requires Windows 10 version 2004+)";
    }

    #[cfg(target_os = "linux")]
    {
        return "Linux: Not supported";
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        return "Unknown platform: Not supported";
    }
}
