// System audio capture for macOS using ScreenCaptureKit
// This allows capturing audio from other applications (Zoom, Meet, etc.)

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use anyhow::{anyhow, Result};

#[cfg(target_os = "macos")]
mod macos {
    /// Get the system audio loopback device if available
    pub fn get_system_audio_device() -> Option<cpal::Device> {
        use cpal::traits::{DeviceTrait, HostTrait};

        let host = cpal::default_host();

        // Priority order for system audio capture devices
        let device_priority = [
            "BlackHole 2ch",
            "BlackHole",
            "Loopback Audio",
            "Soundflower (2ch)",
            "Soundflower",
        ];

        // First try to find devices by priority
        for priority_name in &device_priority {
            for device in host.input_devices().ok()? {
                if let Ok(name) = device.name() {
                    if name.contains(priority_name) {
                        eprintln!("Found system audio device: {}", name);
                        return Some(device);
                    }
                }
            }
        }

        None
    }
}

#[cfg(not(target_os = "macos"))]
mod macos {
    use super::*;

    pub fn check_virtual_audio_device() -> Option<String> {
        None
    }

    pub fn get_system_audio_device() -> Option<cpal::Device> {
        None
    }
}

pub use macos::*;

/// Audio source identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSource {
    Microphone,    // User's voice
    SystemAudio,   // Remote participants (from Zoom/Meet/etc)
}

/// Combined audio message with source information
#[derive(Debug, Clone)]
pub struct SourcedAudioChunk {
    pub data: Vec<u8>,
    pub source: AudioSource,
}

/// Dual audio capturer that captures both microphone and system audio
pub struct DualAudioCapturer {
    is_running: Arc<AtomicBool>,
    has_system_audio: bool,
}

impl DualAudioCapturer {
    pub fn new() -> Self {
        let has_system_audio = get_system_audio_device().is_some();
        if has_system_audio {
            eprintln!("System audio capture available");
        } else {
            eprintln!("No system audio device found. Install BlackHole for full speaker separation.");
            eprintln!("  Download: https://github.com/ExistentialAudio/BlackHole");
        }

        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            has_system_audio,
        }
    }

    pub fn has_system_audio(&self) -> bool {
        self.has_system_audio
    }

    /// Start capturing audio from both microphone and system audio (if available)
    /// Audio is sent as interleaved stereo: left channel = mic, right channel = system
    pub fn start(
        &self,
        audio_tx: mpsc::Sender<Vec<u8>>,
        sample_rate: u32,
    ) -> Result<()> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        if self.is_running.load(Ordering::SeqCst) {
            return Err(anyhow!("Already running"));
        }

        self.is_running.store(true, Ordering::SeqCst);

        let host = cpal::default_host();

        // Get microphone device
        let mic_device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No microphone found"))?;

        eprintln!("Microphone: {:?}", mic_device.name());

        // Get system audio device (if available)
        let system_device = get_system_audio_device();

        if let Some(ref dev) = system_device {
            eprintln!("System audio: {:?}", dev.name());
        }

        let is_running = self.is_running.clone();
        let has_system = system_device.is_some();

        std::thread::spawn(move || {
            // Shared buffers for mic and system audio
            let mic_buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
            let system_buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));

            // Buffer size for ~100ms of audio
            let buffer_samples = (sample_rate as usize / 10) as usize;

            // Build microphone stream
            let mic_buffer_clone = mic_buffer.clone();
            let mic_config = cpal::StreamConfig {
                channels: 1,
                sample_rate: cpal::SampleRate(sample_rate),
                buffer_size: cpal::BufferSize::Default,
            };

            let mic_stream = mic_device.build_input_stream(
                &mic_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let samples: Vec<i16> = data
                        .iter()
                        .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                        .collect();

                    if let Ok(mut buf) = mic_buffer_clone.lock() {
                        buf.extend(samples);
                    }
                },
                |err| eprintln!("Mic stream error: {}", err),
                None,
            );

            // Build system audio stream (if available)
            let system_stream = if let Some(sys_dev) = system_device {
                let system_buffer_clone = system_buffer.clone();
                let sys_config = cpal::StreamConfig {
                    channels: 1, // We'll take just one channel
                    sample_rate: cpal::SampleRate(sample_rate),
                    buffer_size: cpal::BufferSize::Default,
                };

                match sys_dev.build_input_stream(
                    &sys_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let samples: Vec<i16> = data
                            .iter()
                            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                            .collect();

                        if let Ok(mut buf) = system_buffer_clone.lock() {
                            buf.extend(samples);
                        }
                    },
                    |err| eprintln!("System audio stream error: {}", err),
                    None,
                ) {
                    Ok(stream) => Some(stream),
                    Err(e) => {
                        eprintln!("Failed to build system audio stream: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            // Start streams
            if let Ok(ref stream) = mic_stream {
                if let Err(e) = stream.play() {
                    eprintln!("Failed to start mic stream: {}", e);
                    return;
                }
                eprintln!("Microphone capture started");
            }

            if let Some(ref stream) = system_stream {
                if let Err(e) = stream.play() {
                    eprintln!("Failed to start system audio stream: {}", e);
                }
                eprintln!("System audio capture started");
            }

            // Main loop: mix audio into stereo and send
            while is_running.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(50));

                let mic_samples: Vec<i16>;
                let system_samples: Vec<i16>;

                // Get mic samples
                {
                    let mut buf = mic_buffer.lock().unwrap();
                    if buf.len() >= buffer_samples {
                        mic_samples = buf.drain(..buffer_samples).collect();
                    } else {
                        continue;
                    }
                }

                // Get system samples (or silence if not available)
                if has_system {
                    let mut buf = system_buffer.lock().unwrap();
                    if buf.len() >= buffer_samples {
                        system_samples = buf.drain(..buffer_samples).collect();
                    } else {
                        // Pad with silence if system audio is behind
                        system_samples = vec![0i16; buffer_samples];
                    }
                } else {
                    // No system audio device - just silence
                    system_samples = vec![0i16; buffer_samples];
                }

                // Interleave as stereo: [mic_0, sys_0, mic_1, sys_1, ...]
                // Left channel (0) = Microphone = You
                // Right channel (1) = System Audio = Participants
                let mut stereo_bytes: Vec<u8> = Vec::with_capacity(buffer_samples * 4);
                for i in 0..buffer_samples {
                    let mic_sample = mic_samples.get(i).copied().unwrap_or(0);
                    let sys_sample = system_samples.get(i).copied().unwrap_or(0);

                    // Left channel (mic)
                    stereo_bytes.extend_from_slice(&mic_sample.to_le_bytes());
                    // Right channel (system)
                    stereo_bytes.extend_from_slice(&sys_sample.to_le_bytes());
                }

                // Send the stereo audio
                if audio_tx.blocking_send(stereo_bytes).is_err() {
                    break;
                }
            }

            eprintln!("Dual audio capture stopped");
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
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
