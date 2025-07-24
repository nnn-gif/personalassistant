use crate::audio::transcriber::{AudioTranscriber, TranscriptionMethod};
use crate::audio::{AudioProcessor, SimpleAudioRecorder};
use crate::error::Result;
use crate::goals::GoalService;
use crate::llm::LlmClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub device_type: String,
}

#[tauri::command]
pub async fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>> {
    let devices = SimpleAudioRecorder::list_devices()?;
    Ok(devices
        .into_iter()
        .map(|name| AudioDeviceInfo {
            name: name.clone(),
            device_type: if name.starts_with("Default") {
                "DefaultInput".to_string()
            } else {
                "Input".to_string()
            },
        })
        .collect())
}

#[tauri::command]
pub async fn start_audio_recording(
    devices: Vec<String>,
    _title: String,
    recorder: State<'_, Arc<SimpleAudioRecorder>>,
    goal_service: State<'_, Arc<Mutex<GoalService>>>,
    _app: tauri::AppHandle,
) -> Result<String> {
    let device_name = devices.first().cloned();

    // Get the current active goal or default goal
    let goal_id = {
        let goal_service = goal_service.lock().await;
        Some(goal_service.get_current_or_default_goal_id().to_string())
    };

    let info = recorder.inner().start_recording_with_goal(device_name, goal_id)?;

    println!("Started audio recording with goal: {:?}", info);

    // Return recording info as JSON
    Ok(serde_json::to_string(&info)?)
}

#[tauri::command]
pub async fn stop_audio_recording(recorder: State<'_, Arc<SimpleAudioRecorder>>) -> Result<String> {
    let recording = recorder.inner().stop_recording()?;

    if let Some(ref goal_id) = recording.goal_id {
        println!(
            "Stopped audio recording: {} associated with goal: {}",
            recording.id, goal_id
        );
    } else {
        println!(
            "Stopped audio recording: {} (no goal association)",
            recording.id
        );
    }

    Ok(serde_json::to_string(&recording)?)
}

#[tauri::command]
pub async fn pause_audio_recording(_recorder: State<'_, Arc<SimpleAudioRecorder>>) -> Result<()> {
    // TODO: Implement pause
    Ok(())
}

#[tauri::command]
pub async fn resume_audio_recording(_recorder: State<'_, Arc<SimpleAudioRecorder>>) -> Result<()> {
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
    recording_id: String,
    recording_path: String,
    llm: State<'_, Arc<LlmClient>>,
    recorder: State<'_, Arc<SimpleAudioRecorder>>,
) -> Result<String> {
    // Check if transcription already exists
    let recordings = recorder.inner().get_recordings();
    if let Some(recording) = recordings.iter().find(|r| r.id == recording_id) {
        if let Some(ref existing_transcription) = recording.transcription {
            // Return existing transcription if it exists
            return Ok(existing_transcription.clone());
        }
    }

    // If no existing transcription, create a new one
    let transcriber = AudioTranscriber::new(llm.inner().clone());
    let result = transcriber
        .transcribe_audio(
            std::path::Path::new(&recording_path),
            Some(TranscriptionMethod::Whisper),
        )
        .await?;

    // Save the transcription to the recording
    let transcription_json = serde_json::to_string(&result)?;
    recorder.inner().update_transcription(&recording_id, transcription_json.clone())?;

    Ok(transcription_json)
}

#[tauri::command]
pub async fn generate_meeting_summary(
    transcription: String,
    llm: State<'_, Arc<LlmClient>>,
) -> Result<String> {
    let transcriber = AudioTranscriber::new(llm.inner().clone());
    let summary = transcriber.generate_meeting_summary(&transcription).await?;
    Ok(serde_json::to_string(&summary)?)
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
                std::path::Path::new(&output_path),
            )?;
        }
        "reduce_noise" => {
            processor.reduce_noise(
                std::path::Path::new(&input_path),
                std::path::Path::new(&output_path),
            )?;
        }
        _ => return Err(crate::error::AppError::Audio("Unknown operation".into())),
    }

    Ok("Audio processed successfully".to_string())
}

#[tauri::command]
pub async fn get_audio_info(audio_path: String) -> Result<crate::audio::processor::AudioInfo> {
    let processor = AudioProcessor::new();
    processor.get_audio_info(std::path::Path::new(&audio_path))
}

#[tauri::command]
pub async fn delete_recording(
    #[allow(non_snake_case)] recordingId: String,
    recorder: State<'_, Arc<SimpleAudioRecorder>>,
) -> Result<()> {
    recorder.inner().delete_recording(&recordingId)?;
    Ok(())
}
