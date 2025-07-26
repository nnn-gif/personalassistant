pub mod processor;
pub mod recorder;
pub mod simple_recorder;
pub mod transcriber;
#[cfg(target_os = "windows")]
pub mod windows_recorder;

// Platform-specific audio recorder selection
#[cfg(target_os = "windows")]
pub use windows_recorder::WindowsAudioRecorder as PlatformAudioRecorder;

#[cfg(not(target_os = "windows"))]
pub use simple_recorder::SimpleAudioRecorder as PlatformAudioRecorder;

pub use processor::AudioProcessor;
pub use simple_recorder::SimpleAudioRecorder;
#[cfg(target_os = "windows")]
pub use windows_recorder::WindowsAudioRecorder;