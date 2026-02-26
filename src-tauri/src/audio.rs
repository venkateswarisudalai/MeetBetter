use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::path::PathBuf;

use crate::system_audio::{
    get_system_audio_backend, SystemAudioCaptureMethod,
    start_screencapturekit_capture,
};

// We need to handle the Stream in a separate thread since cpal::Stream is not Send
pub struct AudioRecorder {
    stop_signal: Arc<AtomicBool>,
    output_path: String,
    thread_handle: Option<thread::JoinHandle<Result<()>>>,
}

/// Get the recordings folder path (Documents/MeetingRecordings)
pub fn get_recordings_folder() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow!("Could not find HOME directory"))?;
    let recordings_path = PathBuf::from(home).join("Documents").join("MeetingRecordings");

    // Create the folder if it doesn't exist
    if !recordings_path.exists() {
        std::fs::create_dir_all(&recordings_path)?;
    }

    Ok(recordings_path)
}

impl AudioRecorder {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();

        // Try to get the default input device (microphone)
        let mic_device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No input device available"))?;

        let mic_config = mic_device.default_input_config()?;
        let sample_rate = mic_config.sample_rate().0;

        // Determine system audio capture method
        let capture_method = get_system_audio_backend();
        let has_system_audio = capture_method != SystemAudioCaptureMethod::None;
        let wav_channels: u16 = if has_system_audio { 2 } else { 1 };

        eprintln!(
            "AudioRecorder: sample_rate={}, channels={}, capture_method={}",
            sample_rate, wav_channels, capture_method
        );

        // Create output file path in Documents/MeetingRecordings
        let recordings_folder = get_recordings_folder()?;
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let output_path = recordings_folder
            .join(format!("meeting_{}.wav", timestamp))
            .to_string_lossy()
            .to_string();

        let spec = WavSpec {
            channels: wav_channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();
        let output_path_clone = output_path.clone();

        // Run the recording in a separate thread
        let thread_handle = thread::spawn(move || -> Result<()> {
            let writer = WavWriter::create(&output_path_clone, spec)?;
            let writer = Arc::new(Mutex::new(Some(writer)));

            if has_system_audio {
                // STEREO MODE: capture mic + system audio, interleave into WAV
                let mic_buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
                let system_buffer: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));

                // Build mic stream
                let mic_buffer_clone = mic_buffer.clone();
                let mic_stream_config = cpal::StreamConfig {
                    channels: 1,
                    sample_rate: cpal::SampleRate(sample_rate),
                    buffer_size: cpal::BufferSize::Default,
                };

                let mic_stream = mic_device.build_input_stream(
                    &mic_stream_config,
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

                // Start system audio capture
                let system_stream = match capture_method {
                    SystemAudioCaptureMethod::ScreenCaptureKit => {
                        match start_screencapturekit_capture(
                            system_buffer.clone(),
                            sample_rate,
                            stop_signal_clone.clone(),
                        ) {
                            Ok(()) => eprintln!("AudioRecorder: ScreenCaptureKit capture active"),
                            Err(e) => eprintln!("AudioRecorder: ScreenCaptureKit failed: {}. System audio channel will be silent.", e),
                        }
                        None
                    }
                    SystemAudioCaptureMethod::BlackHole => {
                        let system_buffer_clone = system_buffer.clone();
                        match crate::system_audio::get_blackhole_device() {
                            Some(sys_device) => {
                                let sys_config = cpal::StreamConfig {
                                    channels: 2,
                                    sample_rate: cpal::SampleRate(sample_rate),
                                    buffer_size: cpal::BufferSize::Default,
                                };
                                match sys_device.build_input_stream(
                                    &sys_config,
                                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                        let samples: Vec<i16> = data
                                            .chunks(2)
                                            .map(|chunk| {
                                                let left = chunk.first().copied().unwrap_or(0.0);
                                                let right = chunk.get(1).copied().unwrap_or(0.0);
                                                let mono = (left + right) / 2.0;
                                                (mono.clamp(-1.0, 1.0) * 32767.0) as i16
                                            })
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
                                        eprintln!("Failed to build BlackHole stream: {}", e);
                                        None
                                    }
                                }
                            }
                            None => {
                                eprintln!("AudioRecorder: BlackHole device not found");
                                None
                            }
                        }
                    }
                    SystemAudioCaptureMethod::None => None,
                };

                // Start streams
                if let Ok(ref stream) = mic_stream {
                    let _ = stream.play();
                    eprintln!("AudioRecorder: Mic capture started (Channel 0 = You)");
                }
                if let Some(ref stream) = system_stream {
                    let _ = stream.play();
                    eprintln!("AudioRecorder: System audio capture started (Channel 1 = Participant)");
                }

                let samples_per_100ms = sample_rate as usize / 10;

                // Early silence detection for system audio
                let interleave_start = std::time::Instant::now();
                let mut system_empty_ticks: u64 = 0;
                let mut system_data_ticks: u64 = 0;
                let mut silence_warning_emitted = false;

                // Main loop: interleave mic + system audio and write to WAV
                while !stop_signal_clone.load(Ordering::SeqCst) {
                    thread::sleep(std::time::Duration::from_millis(50));

                    let mic_samples: Vec<i16>;
                    let system_samples: Vec<i16>;

                    // Get mic samples
                    {
                        let mut buf = mic_buffer.lock().unwrap();
                        if buf.len() >= samples_per_100ms {
                            mic_samples = buf.drain(..samples_per_100ms).collect();
                        } else {
                            continue;
                        }
                    }

                    // Get system samples
                    let system_had_data;
                    {
                        let mut buf = system_buffer.lock().unwrap();
                        if buf.len() >= samples_per_100ms {
                            system_samples = buf.drain(..samples_per_100ms).collect();
                            system_had_data = true;
                        } else {
                            // Pad with silence if system audio is behind
                            system_samples = vec![0i16; samples_per_100ms];
                            system_had_data = false;
                        }
                    }

                    // Track system audio health
                    if system_had_data {
                        system_data_ticks += 1;
                    } else {
                        system_empty_ticks += 1;
                    }

                    // Early silence detection at 10 seconds
                    if !silence_warning_emitted && interleave_start.elapsed().as_secs() >= 10 {
                        if system_data_ticks == 0 && system_empty_ticks > 0 {
                            eprintln!("AudioRecorder WARNING: System audio buffer empty for 10s ({} empty ticks). Recording will have silent system channel.", system_empty_ticks);
                        } else if system_data_ticks > 0 {
                            eprintln!("AudioRecorder: System audio interleave healthy ({} data ticks, {} empty ticks in first 10s)", system_data_ticks, system_empty_ticks);
                        }
                        silence_warning_emitted = true;
                    }

                    // Write interleaved stereo samples to WAV
                    // Channel 0 (left) = Microphone = You
                    // Channel 1 (right) = System Audio = Participant
                    if let Ok(mut writer_guard) = writer.lock() {
                        if let Some(ref mut w) = *writer_guard {
                            for i in 0..samples_per_100ms {
                                let mic_sample = mic_samples.get(i).copied().unwrap_or(0);
                                let sys_sample = system_samples.get(i).copied().unwrap_or(0);
                                let _ = w.write_sample(mic_sample);
                                let _ = w.write_sample(sys_sample);
                            }
                        }
                    }
                }

                // Streams are dropped here when they go out of scope
                drop(mic_stream);
                drop(system_stream);
            } else {
                // MONO MODE: Just capture mic (original behavior)
                let writer_clone = writer.clone();

                let err_fn = |err| eprintln!("Audio stream error: {}", err);

                let stream = match mic_config.sample_format() {
                    cpal::SampleFormat::F32 => mic_device.build_input_stream(
                        &mic_config.into(),
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            if let Ok(mut writer_guard) = writer_clone.lock() {
                                if let Some(ref mut writer) = *writer_guard {
                                    for &sample in data {
                                        let sample_i16 = (sample * i16::MAX as f32) as i16;
                                        let _ = writer.write_sample(sample_i16);
                                    }
                                }
                            }
                        },
                        err_fn,
                        None,
                    )?,
                    cpal::SampleFormat::I16 => mic_device.build_input_stream(
                        &mic_config.into(),
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            if let Ok(mut writer_guard) = writer_clone.lock() {
                                if let Some(ref mut writer) = *writer_guard {
                                    for &sample in data {
                                        let _ = writer.write_sample(sample);
                                    }
                                }
                            }
                        },
                        err_fn,
                        None,
                    )?,
                    cpal::SampleFormat::U16 => mic_device.build_input_stream(
                        &mic_config.into(),
                        move |data: &[u16], _: &cpal::InputCallbackInfo| {
                            if let Ok(mut writer_guard) = writer_clone.lock() {
                                if let Some(ref mut writer) = *writer_guard {
                                    for &sample in data {
                                        let sample_i16 = (sample as i32 - 32768) as i16;
                                        let _ = writer.write_sample(sample_i16);
                                    }
                                }
                            }
                        },
                        err_fn,
                        None,
                    )?,
                    _ => return Err(anyhow!("Unsupported sample format")),
                };

                stream.play()?;

                // Keep recording until stop signal
                while !stop_signal_clone.load(Ordering::SeqCst) {
                    thread::sleep(std::time::Duration::from_millis(100));
                }

                // Stop the stream
                drop(stream);
            }

            // Finalize the WAV file
            if let Ok(mut writer_guard) = writer.lock() {
                if let Some(w) = writer_guard.take() {
                    w.finalize()?;
                }
            }

            eprintln!("AudioRecorder: recording thread ended");
            Ok(())
        });

        Ok(Self {
            stop_signal,
            output_path,
            thread_handle: Some(thread_handle),
        })
    }

    pub fn get_output_path(&self) -> &str {
        &self.output_path
    }

    pub fn stop(mut self) -> Result<String> {
        // Signal the recording thread to stop
        self.stop_signal.store(true, Ordering::SeqCst);

        // Wait for the thread to finish
        if let Some(handle) = self.thread_handle.take() {
            handle.join().map_err(|_| anyhow!("Recording thread panicked"))??;
        }

        Ok(self.output_path.clone())
    }
}

/// List all recordings in the recordings folder
pub fn list_recordings() -> Result<Vec<String>> {
    let recordings_folder = get_recordings_folder()?;

    let mut recordings: Vec<String> = std::fs::read_dir(&recordings_folder)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "wav")
                .unwrap_or(false)
        })
        .map(|entry| entry.path().to_string_lossy().to_string())
        .collect();

    // Sort by name (newest first since they have timestamps)
    recordings.sort();
    recordings.reverse();

    Ok(recordings)
}
