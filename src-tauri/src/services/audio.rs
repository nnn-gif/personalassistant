use crate::audio::{SimpleAudioRecorder, AudioProcessor};
use crate::error::Result;
use crate::llm::LlmClient;
use std::sync::Arc;
use tauri::State;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub device_type: String,
}

#[tauri::command]
pub async fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>> {
    let devices = SimpleAudioRecorder::list_devices()?;
    Ok(devices.into_iter().map(|name| AudioDeviceInfo {
        name: name.clone(),
        device_type: if name.starts_with("Default") { "DefaultInput".to_string() } else { "Input".to_string() },
    }).collect())
}

#[tauri::command]
pub async fn start_audio_recording(
    devices: Vec<String>,
    _title: String,
    recorder: State<'_, Arc<SimpleAudioRecorder>>,
    _app: tauri::AppHandle,
) -> Result<String> {
    let device_name = devices.first().cloned();
    let info = recorder.start_recording(device_name)?;
    
    // Return recording info as JSON
    Ok(serde_json::to_string(&info)?)
}

#[tauri::command]
pub async fn stop_audio_recording(
    recorder: State<'_, Arc<SimpleAudioRecorder>>,
) -> Result<String> {
    let recording = recorder.stop_recording()?;
    Ok(serde_json::to_string(&recording)?)
}

#[tauri::command]
pub async fn pause_audio_recording(
    _recorder: State<'_, Arc<SimpleAudioRecorder>>,
) -> Result<()> {
    // TODO: Implement pause
    Ok(())
}

#[tauri::command]
pub async fn resume_audio_recording(
    _recorder: State<'_, Arc<SimpleAudioRecorder>>,
) -> Result<()> {
    // TODO: Implement resume
    Ok(())
}

#[tauri::command]
pub async fn get_recording_status(
    _recorder: State<'_, Arc<SimpleAudioRecorder>>,
) -> Result<String> {
    // Return idle status for now
    Ok(r#"{"status":"Idle"}"#.to_string())
}

#[tauri::command]
pub async fn get_recordings(
    recorder: State<'_, Arc<SimpleAudioRecorder>>,
) -> Result<Vec<crate::audio::simple_recorder::AudioRecording>> {
    Ok(recorder.inner().get_recordings())
}

#[tauri::command]
pub async fn transcribe_recording(
    _recording_id: String,
    _recording_path: String,
) -> Result<String> {
    // Return placeholder for now
    Ok("Transcription placeholder".to_string())
}

#[tauri::command]
pub async fn generate_meeting_summary(
    _transcription: String,
    _llm: State<'_, Arc<LlmClient>>,
) -> Result<String> {
    // Return placeholder for now
    Ok("Meeting summary placeholder".to_string())
}

#[tauri::command]
pub async fn process_audio_file(
    input_path: String,
    output_path: String,
    operation: String,
) -> Result<String> {
    let processor = AudioProcessor::new();
    
    match operation.as_str() {
        "standardize" => {
            processor.standardize_audio(
                std::path::Path::new(&input_path),
                std::path::Path::new(&output_path)
            )?;
        },
        "reduce_noise" => {
            processor.reduce_noise(
                std::path::Path::new(&input_path),
                std::path::Path::new(&output_path)
            )?;
        },
        _ => return Err(crate::error::AppError::Audio("Unknown operation".into())),
    }
    
    Ok("Audio processed successfully".to_string())
}

#[tauri::command]
pub async fn get_audio_info(
    audio_path: String,
) -> Result<crate::audio::processor::AudioInfo> {
    let processor = AudioProcessor::new();
    processor.get_audio_info(std::path::Path::new(&audio_path))
}

#[tauri::command]
pub async fn delete_recording(
    #[allow(non_snake_case)]
    recordingId: String,
    recorder: State<'_, Arc<SimpleAudioRecorder>>,
) -> Result<()> {
    recorder.delete_recording(&recordingId)?;
    Ok(())
}