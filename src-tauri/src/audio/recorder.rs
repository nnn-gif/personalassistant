use crate::error::{AppError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioDevice {
    DefaultInput,
    DefaultOutput,
    SystemAudio,
    Microphone(String),
    VirtualDevice(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordingMethod {
    CPAL,
    CoreAudio,
    ScreenCaptureKit,
    WebRTC,
    AppleScript,
    FFmpeg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordingStatus {
    Idle,
    Recording(RecordingInfo),
    Paused(RecordingInfo),
    Processing,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingInfo {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub duration_seconds: f64,
    pub method: RecordingMethod,
    pub devices: Vec<AudioDevice>,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioRecording {
    pub id: Uuid,
    pub title: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duration_seconds: f64,
    pub file_path: PathBuf,
    pub transcription: Option<String>,
    pub meeting_info: Option<MeetingInfo>,
    pub file_size_bytes: u64,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingInfo {
    pub app_name: String,
    pub meeting_title: Option<String>,
    pub participants: Vec<String>,
    pub meeting_url: Option<String>,
}

pub struct AudioRecorder {
    status: Arc<Mutex<RecordingStatus>>,
    recordings: Arc<Mutex<Vec<AudioRecording>>>,
    current_recorder: Arc<Mutex<Option<Box<CpalRecorder>>>>,
}

// Trait for different recording implementations
trait Recorder {
    fn start_recording(&mut self, devices: Vec<AudioDevice>) -> Result<RecordingInfo>;
    fn stop_recording(&mut self) -> Result<PathBuf>;
    fn pause_recording(&mut self) -> Result<()>;
    fn resume_recording(&mut self) -> Result<()>;
    fn get_method(&self) -> RecordingMethod;
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(RecordingStatus::Idle)),
            recordings: Arc::new(Mutex::new(Vec::new())),
            current_recorder: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start_recording(
        &self,
        devices: Vec<AudioDevice>,
        _title: Option<String>,
    ) -> Result<RecordingInfo> {
        // Try each recording method in order of preference
        let methods = vec![
            RecordingMethod::CPAL,
            RecordingMethod::CoreAudio,
            RecordingMethod::ScreenCaptureKit,
            RecordingMethod::WebRTC,
            RecordingMethod::AppleScript,
        ];

        let mut last_error = None;

        for method in methods {
            match self
                .try_recording_method(method.clone(), devices.clone())
                .await
            {
                Ok(info) => {
                    let mut status = self.status.lock().unwrap();
                    *status = RecordingStatus::Recording(info.clone());

                    println!("Started recording with method: {:?}", method);
                    return Ok(info);
                }
                Err(e) => {
                    println!("Failed to start recording with {:?}: {}", method, e);
                    last_error = Some(e);
                }
            }
        }

        // All methods failed
        let error_msg = last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "All recording methods failed".to_string());

        let mut status = self.status.lock().unwrap();
        *status = RecordingStatus::Failed(error_msg.clone());

        Err(AppError::Audio(error_msg))
    }

    async fn try_recording_method(
        &self,
        method: RecordingMethod,
        devices: Vec<AudioDevice>,
    ) -> Result<RecordingInfo> {
        match method {
            RecordingMethod::CPAL => {
                let mut recorder = CpalRecorder::new()?;
                let info = recorder.start_recording(devices)?;
                let mut current_recorder = self.current_recorder.lock().unwrap();
                *current_recorder = Some(Box::new(recorder));
                Ok(info)
            }
            _ => Err(AppError::Audio(format!(
                "{:?} recording method not yet implemented",
                method
            ))),
        }
    }

    pub async fn stop_recording(&self) -> Result<AudioRecording> {
        let mut current_recorder = self.current_recorder.lock().unwrap();

        if let Some(mut recorder) = current_recorder.take() {
            let file_path = recorder.stop_recording()?;

            // Get recording info from status
            let status = self.status.lock().unwrap();
            if let RecordingStatus::Recording(info) = &*status {
                let recording = AudioRecording {
                    id: info.id,
                    title: format!("Recording {}", info.started_at.format("%Y-%m-%d %H:%M")),
                    started_at: info.started_at,
                    ended_at: Utc::now(),
                    duration_seconds: info.duration_seconds,
                    file_path: file_path.clone(),
                    transcription: None,
                    meeting_info: None,
                    file_size_bytes: std::fs::metadata(&file_path)?.len(),
                    sample_rate: 48000, // Default, should be from recorder
                    channels: 2,        // Default, should be from recorder
                };

                let mut recordings = self.recordings.lock().unwrap();
                recordings.push(recording.clone());

                Ok(recording)
            } else {
                Err(AppError::Audio("No active recording".into()))
            }
        } else {
            Err(AppError::Audio("No recorder available".into()))
        }
    }

    pub fn get_status(&self) -> RecordingStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn get_recordings(&self) -> Vec<AudioRecording> {
        self.recordings.lock().unwrap().clone()
    }

    pub fn list_audio_devices() -> Result<Vec<(String, AudioDevice)>> {
        // Use CPAL to list devices
        CpalRecorder::list_devices()
    }
}

// CPAL Recorder Implementation
struct CpalRecorder {
    stream: Option<cpal::Stream>,
    writer: Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>,
    sample_rate: u32,
    channels: u16,
}

impl CpalRecorder {
    fn new() -> Result<Self> {
        Ok(Self {
            stream: None,
            writer: None,
            sample_rate: 48000,
            channels: 2,
        })
    }

    fn list_devices() -> Result<Vec<(String, AudioDevice)>> {
        use cpal::traits::{DeviceTrait, HostTrait};

        let host = cpal::default_host();
        let mut devices = vec![];

        // Add default devices
        if let Some(device) = host.default_input_device() {
            if let Ok(name) = device.name() {
                devices.push((name.clone(), AudioDevice::DefaultInput));
            }
        }

        if let Some(device) = host.default_output_device() {
            if let Ok(name) = device.name() {
                devices.push((name.clone(), AudioDevice::DefaultOutput));
            }
        }

        // List all devices
        if let Ok(all_devices) = host.devices() {
            for device in all_devices {
                if let Ok(name) = device.name() {
                    devices.push((name.clone(), AudioDevice::Microphone(name)));
                }
            }
        }

        Ok(devices)
    }
}

impl CpalRecorder {
    fn start_recording(&mut self, devices: Vec<AudioDevice>) -> Result<RecordingInfo> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| AppError::Audio("No input device available".into()))?;

        let config = device
            .default_input_config()
            .map_err(|e| AppError::Audio(format!("Failed to get device config: {}", e)))?;

        self.sample_rate = config.sample_rate().0;
        self.channels = config.channels();

        // Create output file
        let recording_dir = dirs::data_dir()
            .ok_or_else(|| AppError::Audio("Could not find data directory".into()))?
            .join("personalassistant")
            .join("recordings");

        std::fs::create_dir_all(&recording_dir)?;

        let id = Uuid::new_v4();
        let file_path = recording_dir.join(format!("{}.wav", id));

        let spec = hound::WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let writer = hound::WavWriter::create(&file_path, spec)
            .map_err(|e| AppError::Audio(format!("Failed to create WAV file: {}", e)))?;

        self.writer = Some(writer);

        // Start recording stream
        let writer_clone = Arc::new(Mutex::new(self.writer.take()));

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                self.build_input_stream::<f32>(&device, &config.into(), writer_clone)?
            }
            cpal::SampleFormat::I16 => {
                self.build_input_stream::<i16>(&device, &config.into(), writer_clone)?
            }
            cpal::SampleFormat::U16 => {
                return Err(AppError::Audio(
                    "U16 sample format not supported by hound".into(),
                ))
            }
            _ => return Err(AppError::Audio("Unsupported sample format".into())),
        };

        stream
            .play()
            .map_err(|e| AppError::Audio(format!("Failed to start stream: {}", e)))?;

        self.stream = Some(stream);

        Ok(RecordingInfo {
            id,
            started_at: Utc::now(),
            duration_seconds: 0.0,
            method: RecordingMethod::CPAL,
            devices,
            file_path,
        })
    }

    fn stop_recording(&mut self) -> Result<PathBuf> {
        use cpal::traits::StreamTrait;

        if let Some(stream) = self.stream.take() {
            stream
                .pause()
                .map_err(|e| AppError::Audio(format!("Failed to stop stream: {}", e)))?;
        }

        if let Some(writer) = self.writer.take() {
            writer
                .finalize()
                .map_err(|e| AppError::Audio(format!("Failed to finalize WAV file: {}", e)))?;
        }

        // Return the file path
        Err(AppError::Audio("File path not available".into()))
    }

    fn pause_recording(&mut self) -> Result<()> {
        use cpal::traits::StreamTrait;

        if let Some(stream) = &self.stream {
            stream
                .pause()
                .map_err(|e| AppError::Audio(format!("Failed to pause stream: {}", e)))?;
        }
        Ok(())
    }

    fn resume_recording(&mut self) -> Result<()> {
        use cpal::traits::StreamTrait;

        if let Some(stream) = &self.stream {
            stream
                .play()
                .map_err(|e| AppError::Audio(format!("Failed to resume stream: {}", e)))?;
        }
        Ok(())
    }
}

impl CpalRecorder {
    fn build_input_stream<T>(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        writer: Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    ) -> Result<cpal::Stream>
    where
        T: cpal::Sample + cpal::SizedSample + hound::Sample + Send + 'static,
    {
        use cpal::traits::DeviceTrait;

        let stream = device
            .build_input_stream(
                config,
                move |data: &[T], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut writer_guard) = writer.lock() {
                        if let Some(writer) = writer_guard.as_mut() {
                            for &sample in data {
                                let _ = writer.write_sample(sample);
                            }
                        }
                    }
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            )
            .map_err(|e| AppError::Audio(format!("Failed to build stream: {}", e)))?;

        Ok(stream)
    }
}

// Core Audio Recorder (macOS specific)
#[cfg(target_os = "macos")]
struct CoreAudioRecorder {
    // Implementation using Core Audio APIs
}

#[cfg(target_os = "macos")]
impl CoreAudioRecorder {
    fn new() -> Result<Self> {
        Ok(Self {})
    }
}

#[cfg(target_os = "macos")]
impl Recorder for CoreAudioRecorder {
    fn start_recording(&mut self, devices: Vec<AudioDevice>) -> Result<RecordingInfo> {
        // Use Core Audio APIs
        Err(AppError::Audio("Core Audio not implemented yet".into()))
    }

    fn stop_recording(&mut self) -> Result<PathBuf> {
        Err(AppError::Audio("Core Audio not implemented yet".into()))
    }

    fn pause_recording(&mut self) -> Result<()> {
        Err(AppError::Audio("Core Audio not implemented yet".into()))
    }

    fn resume_recording(&mut self) -> Result<()> {
        Err(AppError::Audio("Core Audio not implemented yet".into()))
    }

    fn get_method(&self) -> RecordingMethod {
        RecordingMethod::CoreAudio
    }
}

// Placeholder for non-macOS
#[cfg(not(target_os = "macos"))]
struct CoreAudioRecorder;

#[cfg(not(target_os = "macos"))]
impl CoreAudioRecorder {
    fn new() -> Result<Self> {
        Err(AppError::Audio("Core Audio only available on macOS".into()))
    }
}

#[cfg(not(target_os = "macos"))]
impl Recorder for CoreAudioRecorder {
    fn start_recording(&mut self, _devices: Vec<AudioDevice>) -> Result<RecordingInfo> {
        unreachable!()
    }
    fn stop_recording(&mut self) -> Result<PathBuf> {
        unreachable!()
    }
    fn pause_recording(&mut self) -> Result<()> {
        unreachable!()
    }
    fn resume_recording(&mut self) -> Result<()> {
        unreachable!()
    }
    fn get_method(&self) -> RecordingMethod {
        RecordingMethod::CoreAudio
    }
}

// ScreenCaptureKit Recorder (macOS 12+)
struct ScreenCaptureRecorder {
    // Implementation for screen capture with audio
}

impl ScreenCaptureRecorder {
    fn new() -> Result<Self> {
        Ok(Self {})
    }
}

impl Recorder for ScreenCaptureRecorder {
    fn start_recording(&mut self, devices: Vec<AudioDevice>) -> Result<RecordingInfo> {
        Err(AppError::Audio(
            "ScreenCaptureKit not implemented yet".into(),
        ))
    }

    fn stop_recording(&mut self) -> Result<PathBuf> {
        Err(AppError::Audio(
            "ScreenCaptureKit not implemented yet".into(),
        ))
    }

    fn pause_recording(&mut self) -> Result<()> {
        Err(AppError::Audio(
            "ScreenCaptureKit not implemented yet".into(),
        ))
    }

    fn resume_recording(&mut self) -> Result<()> {
        Err(AppError::Audio(
            "ScreenCaptureKit not implemented yet".into(),
        ))
    }

    fn get_method(&self) -> RecordingMethod {
        RecordingMethod::ScreenCaptureKit
    }
}

// WebRTC Recorder
struct WebRTCRecorder {
    // WebRTC implementation
}

impl WebRTCRecorder {
    fn new() -> Result<Self> {
        Ok(Self {})
    }
}

impl Recorder for WebRTCRecorder {
    fn start_recording(&mut self, devices: Vec<AudioDevice>) -> Result<RecordingInfo> {
        Err(AppError::Audio("WebRTC not implemented yet".into()))
    }

    fn stop_recording(&mut self) -> Result<PathBuf> {
        Err(AppError::Audio("WebRTC not implemented yet".into()))
    }

    fn pause_recording(&mut self) -> Result<()> {
        Err(AppError::Audio("WebRTC not implemented yet".into()))
    }

    fn resume_recording(&mut self) -> Result<()> {
        Err(AppError::Audio("WebRTC not implemented yet".into()))
    }

    fn get_method(&self) -> RecordingMethod {
        RecordingMethod::WebRTC
    }
}

// AppleScript/QuickTime Recorder
struct AppleScriptRecorder {
    recording_path: Option<PathBuf>,
}

impl AppleScriptRecorder {
    fn new() -> Result<Self> {
        Ok(Self {
            recording_path: None,
        })
    }
}

impl Recorder for AppleScriptRecorder {
    fn start_recording(&mut self, devices: Vec<AudioDevice>) -> Result<RecordingInfo> {
        use std::process::Command;

        let id = Uuid::new_v4();
        let recording_dir = dirs::data_dir()
            .ok_or_else(|| AppError::Audio("Could not find data directory".into()))?
            .join("personalassistant")
            .join("recordings");

        std::fs::create_dir_all(&recording_dir)?;

        let file_path = recording_dir.join(format!("{}.m4a", id));
        self.recording_path = Some(file_path.clone());

        let script = format!(
            r#"
            tell application "QuickTime Player"
                activate
                set newRecording to new audio recording
                tell newRecording
                    start
                end tell
            end tell
            "#
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| AppError::Audio(format!("Failed to run AppleScript: {}", e)))?;

        if !output.status.success() {
            return Err(AppError::Audio(
                "Failed to start QuickTime recording".into(),
            ));
        }

        Ok(RecordingInfo {
            id,
            started_at: Utc::now(),
            duration_seconds: 0.0,
            method: RecordingMethod::AppleScript,
            devices,
            file_path,
        })
    }

    fn stop_recording(&mut self) -> Result<PathBuf> {
        use std::process::Command;

        let file_path = self
            .recording_path
            .take()
            .ok_or_else(|| AppError::Audio("No recording path".into()))?;

        let script = format!(
            r#"
            tell application "QuickTime Player"
                tell document 1
                    stop
                    save in POSIX file "{}" 
                    close
                end tell
            end tell
            "#,
            file_path.display()
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| AppError::Audio(format!("Failed to run AppleScript: {}", e)))?;

        if !output.status.success() {
            return Err(AppError::Audio("Failed to stop QuickTime recording".into()));
        }

        Ok(file_path)
    }

    fn pause_recording(&mut self) -> Result<()> {
        use std::process::Command;

        let script = r#"
            tell application "QuickTime Player"
                tell document 1
                    pause
                end tell
            end tell
        "#;

        Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| AppError::Audio(format!("Failed to pause recording: {}", e)))?;

        Ok(())
    }

    fn resume_recording(&mut self) -> Result<()> {
        use std::process::Command;

        let script = r#"
            tell application "QuickTime Player"
                tell document 1
                    resume
                end tell
            end tell
        "#;

        Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| AppError::Audio(format!("Failed to resume recording: {}", e)))?;

        Ok(())
    }

    fn get_method(&self) -> RecordingMethod {
        RecordingMethod::AppleScript
    }
}
