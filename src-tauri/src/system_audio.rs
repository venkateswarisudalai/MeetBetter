// System audio capture for macOS
// Primary: ScreenCaptureKit (macOS 13+, zero config)
// Fallback: BlackHole / Loopback / Soundflower (any macOS, requires user setup)

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use anyhow::{anyhow, Result};

/// How system audio is being captured
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAudioCaptureMethod {
    /// Apple ScreenCaptureKit (macOS 13+, zero config)
    ScreenCaptureKit,
    /// Virtual audio device: BlackHole, Loopback, Soundflower
    BlackHole,
    /// No system audio — mono mic only with diarization
    None,
}

impl std::fmt::Display for SystemAudioCaptureMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScreenCaptureKit => write!(f, "ScreenCaptureKit"),
            Self::BlackHole => write!(f, "BlackHole"),
            Self::None => write!(f, "None"),
        }
    }
}

/// Audio source identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSource {
    Microphone,    // User's voice (stereo mode, channel 0)
    SystemAudio,   // Remote participants (stereo mode, channel 1)
    Diarized(u32), // Speaker from diarization (mono mode)
}

// ─── macOS implementation ───────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    /// Check if running macOS 13+ (Ventura) where ScreenCaptureKit audio is available
    pub fn is_macos_13_or_later() -> bool {
        use std::process::Command;
        if let Ok(output) = Command::new("sw_vers").arg("-productVersion").output() {
            if let Ok(version_str) = String::from_utf8(output.stdout) {
                let parts: Vec<u32> = version_str
                    .trim()
                    .split('.')
                    .filter_map(|p| p.parse().ok())
                    .collect();
                if let Some(&major) = parts.first() {
                    return major >= 13;
                }
            }
        }
        false
    }

    /// Check Screen Recording permission without triggering a prompt.
    /// Uses CGPreflightScreenCaptureAccess (macOS 10.15+).
    pub fn check_screencapturekit_permission() -> bool {
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGPreflightScreenCaptureAccess() -> bool;
        }
        unsafe { CGPreflightScreenCaptureAccess() }
    }

    /// Request Screen Recording permission (shows the system dialog once).
    /// Returns true if already authorized, false if prompt was shown.
    pub fn request_screencapturekit_permission() -> bool {
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGRequestScreenCaptureAccess() -> bool;
        }
        unsafe { CGRequestScreenCaptureAccess() }
    }

    /// Determine the best available system audio capture backend.
    /// Priority: ScreenCaptureKit → BlackHole → None
    pub fn get_system_audio_backend() -> SystemAudioCaptureMethod {
        // Try ScreenCaptureKit first (macOS 13+)
        if is_macos_13_or_later() {
            if check_screencapturekit_permission() {
                eprintln!("System audio: ScreenCaptureKit (permission granted)");
                return SystemAudioCaptureMethod::ScreenCaptureKit;
            }
            // Permission not yet granted — request it. If the user hasn't
            // responded yet, we fall through to BlackHole for this session.
            eprintln!("System audio: Requesting Screen Recording permission...");
            request_screencapturekit_permission();

            // Re-check after request (will be true if already authorized)
            if check_screencapturekit_permission() {
                eprintln!("System audio: ScreenCaptureKit (permission just granted)");
                return SystemAudioCaptureMethod::ScreenCaptureKit;
            }
            eprintln!("System audio: Screen Recording permission pending/denied");
        }

        // Fallback: look for virtual audio devices
        if get_blackhole_device().is_some() {
            eprintln!("System audio: BlackHole fallback");
            return SystemAudioCaptureMethod::BlackHole;
        }

        eprintln!("System audio: None (mono mic only)");
        SystemAudioCaptureMethod::None
    }

    /// Get the BlackHole / Loopback / Soundflower device if available
    pub fn get_blackhole_device() -> Option<cpal::Device> {
        use cpal::traits::{DeviceTrait, HostTrait};

        let host = cpal::default_host();
        let device_priority = [
            "BlackHole 2ch",
            "BlackHole",
            "Loopback Audio",
            "Soundflower (2ch)",
            "Soundflower",
        ];

        for priority_name in &device_priority {
            if let Ok(devices) = host.input_devices() {
                for device in devices {
                    if let Ok(name) = device.name() {
                        if name.contains(priority_name) {
                            eprintln!("Found virtual audio device: {}", name);
                            return Some(device);
                        }
                    }
                }
            }
        }
        None
    }

    /// Start system audio capture via ScreenCaptureKit.
    /// Fills `system_buffer` with i16 PCM samples at `sample_rate` Hz, mono.
    /// The caller's interleave loop reads from this buffer — same as BlackHole path.
    pub fn start_screencapturekit_capture(
        system_buffer: Arc<Mutex<Vec<i16>>>,
        sample_rate: u32,
        is_running: Arc<AtomicBool>,
    ) -> Result<()> {
        use screencapturekit::prelude::*;

        // Get shareable content (triggers permission prompt if needed)
        let content = SCShareableContent::get()
            .map_err(|e| anyhow!("ScreenCaptureKit: failed to get shareable content (Screen Recording permission denied?): {}", e))?;

        let display = content
            .displays()
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("ScreenCaptureKit: no displays found"))?;

        // Filter: capture all audio from all apps on the primary display
        let filter = SCContentFilter::create()
            .with_display(&display)
            .with_excluding_windows(&[])
            .build();

        // Minimal 2×2 video (required by API) + audio at matching sample rate, mono
        let config = SCStreamConfiguration::new()
            .with_width(2)
            .with_height(2)
            .with_captures_audio(true)
            .with_sample_rate(sample_rate as i32)
            .with_channel_count(1); // mono — we only need one channel for system audio

        // Handler that pushes audio samples into the shared buffer
        struct AudioHandler {
            system_buffer: Arc<Mutex<Vec<i16>>>,
            is_running: Arc<AtomicBool>,
        }

        impl SCStreamOutputTrait for AudioHandler {
            fn did_output_sample_buffer(
                &self,
                sample: CMSampleBuffer,
                output_type: SCStreamOutputType,
            ) {
                if !self.is_running.load(Ordering::Relaxed) {
                    return;
                }

                if output_type != SCStreamOutputType::Audio {
                    return; // ignore video frames
                }

                // Extract audio data from the CMSampleBuffer
                let audio_buffers = match sample.audio_buffer_list() {
                    Some(list) => list,
                    None => return,
                };

                // Process each audio buffer in the list
                for buffer in audio_buffers.iter() {
                    let raw_bytes = buffer.data();
                    if raw_bytes.is_empty() {
                        continue;
                    }

                    // Audio data is f32 PCM — convert to i16
                    let f32_samples: &[f32] = unsafe {
                        std::slice::from_raw_parts(
                            raw_bytes.as_ptr() as *const f32,
                            raw_bytes.len() / std::mem::size_of::<f32>(),
                        )
                    };

                    let i16_samples: Vec<i16> = f32_samples
                        .iter()
                        .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                        .collect();

                    if let Ok(mut buf) = self.system_buffer.lock() {
                        buf.extend(i16_samples);
                    }
                }
            }
        }

        let handler = AudioHandler {
            system_buffer: system_buffer.clone(),
            is_running: is_running.clone(),
        };

        let mut stream = SCStream::new(&filter, &config);
        // Register for Audio output type to receive audio sample callbacks
        stream.add_output_handler(handler, SCStreamOutputType::Audio);

        stream.start_capture()
            .map_err(|e| anyhow!("ScreenCaptureKit: failed to start capture: {}", e))?;

        eprintln!("ScreenCaptureKit system audio capture started");

        // Keep the stream alive on a background thread until is_running becomes false
        std::thread::spawn(move || {
            while is_running.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let _ = stream.stop_capture();
            eprintln!("ScreenCaptureKit system audio capture stopped");
        });

        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
mod macos {
    use super::*;

    pub fn get_system_audio_backend() -> SystemAudioCaptureMethod {
        SystemAudioCaptureMethod::None
    }

    pub fn get_blackhole_device() -> Option<cpal::Device> {
        None
    }

    pub fn check_screencapturekit_permission() -> bool {
        false
    }

    pub fn start_screencapturekit_capture(
        _system_buffer: Arc<Mutex<Vec<i16>>>,
        _sample_rate: u32,
        _is_running: Arc<AtomicBool>,
    ) -> Result<()> {
        Err(anyhow!("ScreenCaptureKit is only available on macOS"))
    }
}

pub use macos::*;

// ─── Compat: keep get_system_audio_device() working for any code that still uses it ───

/// Legacy function: returns a cpal device for BlackHole/Loopback/Soundflower.
/// Prefer get_system_audio_backend() for new code.
pub fn get_system_audio_device() -> Option<cpal::Device> {
    get_blackhole_device()
}

/// Returns the name of the detected virtual audio device, or None
pub fn get_system_audio_device_name() -> Option<String> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let device_priority = [
        "BlackHole 2ch",
        "BlackHole",
        "Loopback Audio",
        "Soundflower (2ch)",
        "Soundflower",
    ];

    for priority_name in &device_priority {
        if let Ok(devices) = host.input_devices() {
            for device in devices {
                if let Ok(name) = device.name() {
                    if name.contains(priority_name) {
                        return Some(name);
                    }
                }
            }
        }
    }
    None
}

/// List available audio devices for debugging
pub fn list_audio_devices() -> Vec<String> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let mut devices = Vec::new();
    let host = cpal::default_host();

    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                devices.push(format!("Input: {}", name));
            }
        }
    }

    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            if let Ok(name) = device.name() {
                devices.push(format!("Output: {}", name));
            }
        }
    }

    devices
}
