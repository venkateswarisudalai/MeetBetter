use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex};
use std::thread;
use std::path::PathBuf;

// We need to handle the Stream in a separate thread since cpal::Stream is not Send
pub struct AudioRecorder {
    stop_signal: Arc<Mutex<bool>>,
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
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No input device available"))?;

        let config = device.default_input_config()?;

        // Create output file path in Documents/MeetingRecordings
        let recordings_folder = get_recordings_folder()?;
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let output_path = recordings_folder
            .join(format!("meeting_{}.wav", timestamp))
            .to_string_lossy()
            .to_string();

        let spec = WavSpec {
            channels: config.channels(),
            sample_rate: config.sample_rate().0,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let stop_signal = Arc::new(Mutex::new(false));
        let stop_signal_clone = stop_signal.clone();
        let output_path_clone = output_path.clone();

        // Run the recording in a separate thread
        let thread_handle = thread::spawn(move || -> Result<()> {
            let writer = WavWriter::create(&output_path_clone, spec)?;
            let writer = Arc::new(Mutex::new(Some(writer)));
            let writer_clone = writer.clone();

            let err_fn = |err| eprintln!("Audio stream error: {}", err);

            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &config.into(),
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
                cpal::SampleFormat::I16 => device.build_input_stream(
                    &config.into(),
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
                cpal::SampleFormat::U16 => device.build_input_stream(
                    &config.into(),
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
            loop {
                if let Ok(stop) = stop_signal_clone.lock() {
                    if *stop {
                        break;
                    }
                }
                thread::sleep(std::time::Duration::from_millis(100));
            }

            // Stop the stream
            drop(stream);

            // Finalize the WAV file
            if let Ok(mut writer_guard) = writer.lock() {
                if let Some(w) = writer_guard.take() {
                    w.finalize()?;
                }
            }

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
        if let Ok(mut stop) = self.stop_signal.lock() {
            *stop = true;
        }

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
