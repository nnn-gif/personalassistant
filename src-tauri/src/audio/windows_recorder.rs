use crate::audio::simple_recorder::{AudioRecording, RecordingInfo};
use crate::error::{AppError, Result};
use chrono::Utc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Windows-specific audio recorder that properly handles WASAPI streams
pub struct WindowsAudioRecorder {
    recordings_dir: PathBuf,
    recordings: Arc<Mutex<Vec<AudioRecording>>>,
    current_stream: Arc<Mutex<Option<cpal::Stream>>>,
    current_recording: Arc<Mutex<Option<RecordingInfo>>>,
    writer: Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    sample_count: Arc<Mutex<u64>>,
    sample_rate: Arc<Mutex<u32>>,
    channels: Arc<Mutex<u16>>,
    goal_id: Arc<Mutex<Option<String>>>,
}

impl WindowsAudioRecorder {
    pub fn new() -> Result<Self> {
        let recordings_dir = dirs::data_dir()
            .ok_or_else(|| AppError::Audio("Could not find data directory".to_string()))?
            .join("personalassistant")
            .join("recordings");

        std::fs::create_dir_all(&recordings_dir)?;

        Ok(Self {
            recordings_dir,
            recordings: Arc::new(Mutex::new(Vec::new())),
            current_stream: Arc::new(Mutex::new(None)),
            current_recording: Arc::new(Mutex::new(None)),
            writer: Arc::new(Mutex::new(None)),
            sample_count: Arc::new(Mutex::new(0)),
            sample_rate: Arc::new(Mutex::new(48000)),
            channels: Arc::new(Mutex::new(2)),
            goal_id: Arc::new(Mutex::new(None)),
        })
    }

    pub fn start_recording(
        &self,
        device_name: Option<String>,
        goal_id: Option<String>,
    ) -> Result<RecordingInfo> {
        tracing::info!("Starting Windows audio recording with device: {:?}", device_name);
        
        // Ensure no recording is in progress
        if self.current_recording.lock().unwrap().is_some() {
            tracing::warn!("Recording already in progress");
            return Err(AppError::Audio("Recording already in progress".to_string()));
        }

        let host = cpal::default_host();
        tracing::debug!("Using audio host: {:?}", host.id());

        // Get the device
        let device = if let Some(name) = device_name.clone() {
            tracing::debug!("Looking for specific device: {}", name);
            let devices: Vec<_> = host.input_devices()
                .map_err(|e| AppError::Audio(format!("Failed to list devices: {e}")))?
                .collect();
            
            tracing::debug!("Found {} input devices", devices.len());
            for d in &devices {
                if let Ok(device_name) = d.name() {
                    tracing::debug!("  - {}", device_name);
                }
            }
            
            devices.into_iter()
                .find(|d| d.name().ok() == Some(name.clone()))
                .ok_or_else(|| AppError::Audio(format!("Device '{}' not found", name)))?
        } else {
            tracing::debug!("Using default input device");
            host.default_input_device()
                .ok_or_else(|| AppError::Audio("No default input device found".to_string()))?
        };
        
        let device_name_str = device.name().unwrap_or_else(|_| "Unknown".to_string());
        tracing::info!("Selected device: {}", device_name_str);

        // Get supported configs and pick the best one
        let mut supported_configs = device
            .supported_input_configs()
            .map_err(|e| AppError::Audio(format!("Failed to get supported configs: {e}")))?
            .collect::<Vec<_>>();

        if supported_configs.is_empty() {
            return Err(AppError::Audio(
                "No supported input configurations found".to_string(),
            ));
        }

        tracing::debug!("Found {} supported configurations", supported_configs.len());
        
        // Sort by sample rate and channels to get the best quality
        supported_configs.sort_by(|a, b| {
            let a_rate = a.max_sample_rate().0;
            let b_rate = b.max_sample_rate().0;
            let a_channels = a.channels() as u32;
            let b_channels = b.channels() as u32;

            // Prefer standard sample rates (48000, 44100) over others
            let a_standard = matches!(a_rate, 48000 | 44100);
            let b_standard = matches!(b_rate, 48000 | 44100);
            
            if a_standard != b_standard {
                return b_standard.cmp(&a_standard);
            }

            // Then prefer higher sample rate and more channels
            (b_rate * b_channels).cmp(&(a_rate * a_channels))
        });

        let config = supported_configs.into_iter().next().unwrap();
        let sample_rate = config.max_sample_rate().0;
        let channels = config.channels();
        let sample_format = config.sample_format();
        
        tracing::info!(
            "Selected config: {} Hz, {} channels, {:?} format",
            sample_rate, channels, sample_format
        );

        // Create config
        let stream_config = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        // Create recording file
        let id = Uuid::new_v4();
        let file_path = self.recordings_dir.join(format!("{id}.wav"));

        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let wav_writer = WavWriter::create(&file_path, spec)
            .map_err(|e| AppError::Audio(format!("Failed to create WAV file: {e}")))?;

        // Store writer and recording info
        *self.writer.lock().unwrap() = Some(wav_writer);
        *self.sample_count.lock().unwrap() = 0;
        *self.sample_rate.lock().unwrap() = sample_rate;
        *self.channels.lock().unwrap() = channels;
        *self.goal_id.lock().unwrap() = goal_id;

        let info = RecordingInfo {
            id,
            started_at: Utc::now(),
            file_path: file_path.clone(),
        };

        // Create the stream with proper error handling
        let writer_clone = self.writer.clone();
        let sample_count_clone = self.sample_count.clone();

        let err_fn = |err| {
            tracing::error!("Audio stream error: {}", err);
            eprintln!("Audio stream error: {}", err);
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut guard) = writer_clone.lock() {
                            if let Some(ref mut writer) = *guard {
                                for &sample in data {
                                    let sample_i16 =
                                        (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                                    let _ = writer.write_sample(sample_i16);
                                }
                                if let Ok(mut count) = sample_count_clone.lock() {
                                    *count += data.len() as u64;
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AppError::Audio(format!("Failed to build F32 stream: {e}")))?,
            cpal::SampleFormat::I16 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut guard) = writer_clone.lock() {
                            if let Some(ref mut writer) = *guard {
                                for &sample in data {
                                    let _ = writer.write_sample(sample);
                                }
                                if let Ok(mut count) = sample_count_clone.lock() {
                                    *count += data.len() as u64;
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AppError::Audio(format!("Failed to build I16 stream: {e}")))?,
            cpal::SampleFormat::U16 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut guard) = writer_clone.lock() {
                            if let Some(ref mut writer) = *guard {
                                for &sample in data {
                                    let sample_i16 =
                                        (sample as i32 - 32768).clamp(-32768, 32767) as i16;
                                    let _ = writer.write_sample(sample_i16);
                                }
                                if let Ok(mut count) = sample_count_clone.lock() {
                                    *count += data.len() as u64;
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AppError::Audio(format!("Failed to build U16 stream: {e}")))?,
            _ => {
                return Err(AppError::Audio(format!(
                    "Unsupported sample format: {:?}",
                    sample_format
                )))
            }
        };

        // Start the stream
        stream
            .play()
            .map_err(|e| AppError::Audio(format!("Failed to start stream: {e}")))?;

        // Store the stream and recording info
        *self.current_stream.lock().unwrap() = Some(stream);
        *self.current_recording.lock().unwrap() = Some(info.clone());

        tracing::info!(
            "Recording started successfully: {} at {}",
            info.id,
            info.file_path.display()
        );

        Ok(info)
    }

    pub fn stop_recording(&self) -> Result<AudioRecording> {
        // Take the stream to stop it (dropping it stops the stream)
        let stream = self
            .current_stream
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| AppError::Audio("No recording in progress".to_string()))?;

        // Explicitly drop the stream to stop recording
        drop(stream);

        // Take the recording info
        let info = self
            .current_recording
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| AppError::Audio("No recording info found".to_string()))?;

        // Get the sample count and recording parameters
        let sample_count = *self.sample_count.lock().unwrap();
        let sample_rate = *self.sample_rate.lock().unwrap();
        let channels = *self.channels.lock().unwrap();
        let goal_id = self.goal_id.lock().unwrap().clone();

        // Close the writer
        if let Some(writer) = self.writer.lock().unwrap().take() {
            writer
                .finalize()
                .map_err(|e| AppError::Audio(format!("Failed to finalize recording: {e}")))?;
        }

        // Calculate duration
        let duration = sample_count as f64 / (sample_rate as f64 * channels as f64);

        // Get file info
        let metadata = fs::metadata(&info.file_path)?;
        let ended_at = Utc::now();

        let recording = AudioRecording {
            id: info.id.to_string(),
            title: format!("Recording {}", info.started_at.format("%Y-%m-%d %H:%M:%S")),
            started_at: info.started_at.to_rfc3339(),
            ended_at: ended_at.to_rfc3339(),
            duration_seconds: duration,
            file_path: info.file_path.to_string_lossy().to_string(),
            file_size_bytes: metadata.len(),
            sample_rate,
            channels,
            transcription: None,
            goal_id,
        };

        // Store the recording
        self.recordings.lock().unwrap().push(recording.clone());

        Ok(recording)
    }

    pub fn is_recording(&self) -> bool {
        self.current_recording.lock().unwrap().is_some()
    }

    pub fn get_recordings(&self) -> Vec<AudioRecording> {
        self.recordings.lock().unwrap().clone()
    }

    pub fn list_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let mut devices = vec!["Default Input".to_string()];

        // Get all input devices
        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                if let Ok(name) = device.name() {
                    // Skip duplicate default device entries
                    if !name.contains("Default") {
                        devices.push(name);
                    }
                }
            }
        }

        Ok(devices)
    }

    pub fn start_recording_with_goal(
        &self,
        device_name: Option<String>,
        goal_id: Option<String>,
    ) -> Result<RecordingInfo> {
        self.start_recording(device_name, goal_id)
    }

    pub fn delete_recording(&self, recording_id: &str) -> Result<()> {
        // Find and remove the recording from memory
        let mut recordings = self.recordings.lock().unwrap();
        if let Some(pos) = recordings.iter().position(|r| r.id == recording_id) {
            let recording = recordings.remove(pos);
            // Try to delete the file
            if let Ok(path) = std::path::PathBuf::from(&recording.file_path).canonicalize() {
                let _ = std::fs::remove_file(path);
            }
        }
        Ok(())
    }

    pub fn update_transcription(&self, recording_id: &str, transcription: String) -> Result<()> {
        let mut recordings = self.recordings.lock().unwrap();
        if let Some(recording) = recordings.iter_mut().find(|r| r.id == recording_id) {
            recording.transcription = Some(transcription);
        }
        Ok(())
    }
}
