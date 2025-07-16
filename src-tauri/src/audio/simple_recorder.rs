use crate::error::{AppError, Result};
use chrono::Utc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingInfo {
    pub id: Uuid,
    pub started_at: chrono::DateTime<Utc>,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioRecording {
    pub id: String,
    pub title: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_seconds: f64,
    pub file_path: String,
    pub file_size_bytes: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub transcription: Option<String>,
    pub goal_id: Option<String>,
}

struct ActiveRecording {
    info: RecordingInfo,
    writer: Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    sample_count: Arc<Mutex<u64>>,
    sample_rate: u32,
    channels: u16,
    ended_at: Arc<Mutex<Option<chrono::DateTime<Utc>>>>,
    goal_id: Option<String>,
}

pub struct SimpleAudioRecorder {
    recordings_dir: PathBuf,
    recordings: Arc<Mutex<Vec<AudioRecording>>>,
    current_recording: Arc<Mutex<Option<ActiveRecording>>>,
}

impl SimpleAudioRecorder {
    pub fn new() -> Result<Self> {
        let recordings_dir = dirs::data_dir()
            .ok_or_else(|| AppError::Audio("Could not find data directory".to_string()))?
            .join("personalassistant")
            .join("recordings");

        std::fs::create_dir_all(&recordings_dir)?;

        // Load existing recordings
        let mut recordings = Vec::new();
        if let Ok(entries) = fs::read_dir(&recordings_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "wav" {
                        // Create a basic recording entry for existing files
                        if let Ok(metadata) = entry.metadata() {
                            let file_name = entry.file_name().to_string_lossy().to_string();
                            let id = file_name.trim_end_matches(".wav").to_string();

                            // Try to read WAV header for actual duration
                            let duration = if let Ok(reader) = hound::WavReader::open(entry.path())
                            {
                                let spec = reader.spec();
                                let sample_count = reader.len() as f64;
                                sample_count / (spec.sample_rate as f64 * spec.channels as f64)
                            } else {
                                0.0
                            };

                            recordings.push(AudioRecording {
                                id: id.clone(),
                                title: format!("Recording {id}"),
                                started_at: chrono::DateTime::<Utc>::from(
                                    metadata
                                        .created()
                                        .unwrap_or_else(|_| std::time::SystemTime::now()),
                                )
                                .to_rfc3339(),
                                ended_at: chrono::DateTime::<Utc>::from(
                                    metadata
                                        .modified()
                                        .unwrap_or_else(|_| std::time::SystemTime::now()),
                                )
                                .to_rfc3339(),
                                duration_seconds: duration,
                                file_path: entry.path().to_string_lossy().to_string(),
                                file_size_bytes: metadata.len(),
                                sample_rate: 48000, // Default, would need to read from file
                                channels: 2,        // Default
                                transcription: None, // Will be loaded from database if available
                                goal_id: None,      // Existing recordings not associated with goals
                            });
                        }
                    }
                }
            }
        }

        Ok(Self {
            recordings_dir,
            recordings: Arc::new(Mutex::new(recordings)),
            current_recording: Arc::new(Mutex::new(None)),
        })
    }

    pub fn list_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let mut device_names = vec![];

        if let Some(device) = host.default_input_device() {
            if let Ok(name) = device.name() {
                device_names.push(format!("Default Input: {name}"));
            }
        }

        if let Ok(devices) = host.input_devices() {
            for device in devices {
                if let Ok(name) = device.name() {
                    device_names.push(name);
                }
            }
        }

        Ok(device_names)
    }

    pub fn start_recording(&self, device_name: Option<String>) -> Result<RecordingInfo> {
        self.start_recording_with_goal(device_name, None)
    }

    pub fn start_recording_with_goal(
        &self,
        device_name: Option<String>,
        goal_id: Option<String>,
    ) -> Result<RecordingInfo> {
        let host = cpal::default_host();

        // Get the device
        let device = if let Some(name) = device_name {
            // Check if it's the default device
            if name.starts_with("Default Input:") {
                host.default_input_device()
                    .ok_or_else(|| AppError::Audio("No default input device found".to_string()))?
            } else {
                // Search for the device by name
                host.input_devices()
                    .map_err(|e| AppError::Audio(format!("Failed to list devices: {e}")))?
                    .find(|d| d.name().ok() == Some(name.clone()))
                    .ok_or_else(|| AppError::Audio(format!("Device {name} not found")))?
            }
        } else {
            host.default_input_device()
                .ok_or_else(|| AppError::Audio("No default input device found".to_string()))?
        };

        // Get default config
        let config = device
            .default_input_config()
            .map_err(|e| AppError::Audio(format!("Failed to get device config: {e}")))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        // Create recording file
        let id = Uuid::new_v4();
        let file_path = self.recordings_dir.join(format!("{id}.wav"));

        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let writer = Arc::new(Mutex::new(Some(
            WavWriter::create(&file_path, spec)
                .map_err(|e| AppError::Audio(format!("Failed to create WAV file: {e}")))?,
        )));

        // Store recording info
        let info = RecordingInfo {
            id,
            started_at: Utc::now(),
            file_path: file_path.clone(),
        };

        // Track sample count for duration calculation
        let sample_count = Arc::new(Mutex::new(0u64));
        let ended_at = Arc::new(Mutex::new(None));

        // Create the stream
        let writer_clone = writer.clone();
        let sample_count_clone = sample_count.clone();
        let ended_at_clone = ended_at.clone();
        let ended_at_clone2 = ended_at.clone();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[f32], _: &_| {
                            if let Ok(mut guard) = writer_clone.lock() {
                                if let Some(ref mut writer) = *guard {
                                    for &sample in data {
                                        let sample_i16 =
                                            (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                                        let _ = writer.write_sample(sample_i16);
                                    }
                                    // Update sample count
                                    if let Ok(mut count) = sample_count_clone.lock() {
                                        *count += data.len() as u64;
                                    }
                                }
                            }
                        },
                        move |err| {
                            eprintln!("Stream error: {err}");
                            *ended_at_clone.lock().unwrap() = Some(Utc::now());
                        },
                        None,
                    )
                    .map_err(|e| AppError::Audio(format!("Failed to build stream: {e}")))?
            }
            cpal::SampleFormat::I16 => {
                let writer_clone2 = writer.clone();
                let sample_count_clone2 = sample_count.clone();
                device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[i16], _: &_| {
                            if let Ok(mut guard) = writer_clone2.lock() {
                                if let Some(ref mut writer) = *guard {
                                    for &sample in data {
                                        let _ = writer.write_sample(sample);
                                    }
                                    // Update sample count
                                    if let Ok(mut count) = sample_count_clone2.lock() {
                                        *count += data.len() as u64;
                                    }
                                }
                            }
                        },
                        move |err| {
                            eprintln!("Stream error: {err}");
                            *ended_at_clone2.lock().unwrap() = Some(Utc::now());
                        },
                        None,
                    )
                    .map_err(|e| AppError::Audio(format!("Failed to build stream: {e}")))?
            }
            _ => return Err(AppError::Audio("Unsupported sample format".into())),
        };

        stream
            .play()
            .map_err(|e| AppError::Audio(format!("Failed to start stream: {e}")))?;

        // Store the active recording
        let active_recording = ActiveRecording {
            info: info.clone(),
            writer,
            sample_count,
            sample_rate,
            channels,
            ended_at,
            goal_id: goal_id.clone(),
        };

        // Store the active recording before forgetting the stream
        *self.current_recording.lock().unwrap() = Some(active_recording);

        // IMPORTANT: We use std::mem::forget here to keep the stream alive
        // The stream will continue recording until stop_recording is called
        // When the recording is stopped, the writer will be finalized which
        // ensures all data is properly written
        std::mem::forget(stream);

        Ok(info)
    }

    pub fn stop_recording(&self) -> Result<AudioRecording> {
        let mut current_guard = self.current_recording.lock().unwrap();
        let active_recording = current_guard
            .take()
            .ok_or_else(|| AppError::Audio("No recording in progress".to_string()))?;

        // Set the end time first
        let ended_at = Utc::now();
        *active_recording.ended_at.lock().unwrap() = Some(ended_at);

        // Close the writer to ensure all data is flushed
        if let Ok(mut writer_guard) = active_recording.writer.lock() {
            if let Some(writer) = writer_guard.take() {
                writer
                    .finalize()
                    .map_err(|e| AppError::Audio(format!("Failed to finalize recording: {e}")))?;
            }
        }

        // Calculate actual duration from sample count
        let sample_count = *active_recording.sample_count.lock().unwrap();
        let duration = sample_count as f64
            / (active_recording.sample_rate as f64 * active_recording.channels as f64);

        // Get file info
        let metadata = fs::metadata(&active_recording.info.file_path)?;

        let recording = AudioRecording {
            id: active_recording.info.id.to_string(),
            title: format!(
                "Recording {}",
                active_recording.info.started_at.format("%Y-%m-%d %H:%M:%S")
            ),
            started_at: active_recording.info.started_at.to_rfc3339(),
            ended_at: ended_at.to_rfc3339(),
            duration_seconds: duration,
            file_path: active_recording
                .info
                .file_path
                .to_string_lossy()
                .to_string(),
            file_size_bytes: metadata.len(),
            sample_rate: active_recording.sample_rate,
            channels: active_recording.channels,
            transcription: None,
            goal_id: active_recording.goal_id,
        };

        // Store the recording
        self.recordings.lock().unwrap().push(recording.clone());

        Ok(recording)
    }

    pub fn get_recordings(&self) -> Vec<AudioRecording> {
        self.recordings.lock().unwrap().clone()
    }

    pub fn delete_recording(&self, recording_id: &str) -> Result<()> {
        let mut recordings = self.recordings.lock().unwrap();

        // Find the recording
        let index = recordings
            .iter()
            .position(|r| r.id == recording_id)
            .ok_or_else(|| AppError::Audio(format!("Recording {recording_id} not found")))?;

        let recording = &recordings[index];
        let file_path = PathBuf::from(&recording.file_path);

        // Delete the file
        if file_path.exists() {
            fs::remove_file(&file_path)
                .map_err(|e| AppError::Audio(format!("Failed to delete file: {e}")))?;
        }

        // Remove from the list
        recordings.remove(index);

        Ok(())
    }

    pub fn update_transcription(&self, recording_id: &str, transcription: String) -> Result<()> {
        let mut recordings = self.recordings.lock().unwrap();

        // Find the recording
        let recording = recordings
            .iter_mut()
            .find(|r| r.id == recording_id)
            .ok_or_else(|| AppError::Audio(format!("Recording {recording_id} not found")))?;

        // Update the transcription
        recording.transcription = Some(transcription);

        Ok(())
    }
}
