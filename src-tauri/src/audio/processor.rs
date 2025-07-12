use crate::error::{Result, AppError};
use std::path::{Path, PathBuf};
use hound::{WavReader, WavWriter, WavSpec};
use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};

pub struct AudioProcessor;

impl AudioProcessor {
    pub fn new() -> Self {
        Self
    }
    
    /// Convert audio file to standard format for processing
    pub fn standardize_audio(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        // Read the input file
        let reader = WavReader::open(input_path)
            .map_err(|e| AppError::Audio(format!("Failed to open audio file: {}", e)))?;
        
        let spec = reader.spec();
        
        // Check if resampling is needed (target: 16kHz mono for transcription)
        if spec.sample_rate != 16000 || spec.channels != 1 {
            self.resample_audio(input_path, output_path, 16000, 1)?;
        } else {
            // Just copy the file
            std::fs::copy(input_path, output_path)?;
        }
        
        Ok(())
    }
    
    /// Resample audio to target sample rate and channels
    pub fn resample_audio(
        &self,
        input_path: &Path,
        output_path: &Path,
        target_sample_rate: u32,
        target_channels: u16,
    ) -> Result<()> {
        let mut reader = WavReader::open(input_path)
            .map_err(|e| AppError::Audio(format!("Failed to open audio file: {}", e)))?;
        
        let spec = reader.spec();
        let source_sample_rate = spec.sample_rate;
        
        // Read all samples
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>()
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| AppError::Audio(format!("Failed to read samples: {}", e)))?
            }
            hound::SampleFormat::Int => {
                let bit_depth = spec.bits_per_sample;
                let max_val = (1 << (bit_depth - 1)) as f32;
                reader.samples::<i32>()
                    .map(|s| s.map(|v| v as f32 / max_val))
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| AppError::Audio(format!("Failed to read samples: {}", e)))?
            }
        };
        
        // Convert to channels
        let channel_samples = self.deinterleave_samples(&samples, spec.channels);
        
        // Mix down to mono if needed
        let mono_samples = if target_channels == 1 && spec.channels > 1 {
            self.mix_to_mono(&channel_samples)
        } else {
            channel_samples[0].clone()
        };
        
        // Resample if needed
        let output_samples = if source_sample_rate != target_sample_rate {
            self.resample(&mono_samples, source_sample_rate, target_sample_rate)?
        } else {
            mono_samples
        };
        
        // Write output file
        let out_spec = WavSpec {
            channels: target_channels,
            sample_rate: target_sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        
        let mut writer = WavWriter::create(output_path, out_spec)
            .map_err(|e| AppError::Audio(format!("Failed to create output file: {}", e)))?;
        
        // Convert back to i16
        for sample in output_samples {
            let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(sample_i16)
                .map_err(|e| AppError::Audio(format!("Failed to write sample: {}", e)))?;
        }
        
        writer.finalize()
            .map_err(|e| AppError::Audio(format!("Failed to finalize output: {}", e)))?;
        
        Ok(())
    }
    
    /// Split audio into segments for better transcription
    pub fn split_audio(&self, input_path: &Path, segment_duration_seconds: f32) -> Result<Vec<PathBuf>> {
        let mut reader = WavReader::open(input_path)
            .map_err(|e| AppError::Audio(format!("Failed to open audio file: {}", e)))?;
        
        let spec = reader.spec();
        let samples_per_segment = (spec.sample_rate as f32 * segment_duration_seconds) as usize * spec.channels as usize;
        
        let mut segments = Vec::new();
        let mut segment_index = 0;
        let base_path = input_path.parent().unwrap_or(Path::new("."));
        let base_name = input_path.file_stem().unwrap_or_default();
        
        let samples: Vec<i16> = reader.samples::<i16>()
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| AppError::Audio(format!("Failed to read samples: {}", e)))?;
        
        for chunk in samples.chunks(samples_per_segment) {
            let segment_path = base_path.join(format!("{}_segment_{}.wav", base_name.to_string_lossy(), segment_index));
            
            let mut writer = WavWriter::create(&segment_path, spec)
                .map_err(|e| AppError::Audio(format!("Failed to create segment file: {}", e)))?;
            
            for &sample in chunk {
                writer.write_sample(sample)
                    .map_err(|e| AppError::Audio(format!("Failed to write sample: {}", e)))?;
            }
            
            writer.finalize()
                .map_err(|e| AppError::Audio(format!("Failed to finalize segment: {}", e)))?;
            
            segments.push(segment_path);
            segment_index += 1;
        }
        
        Ok(segments)
    }
    
    /// Apply noise reduction
    pub fn reduce_noise(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        // Simple noise gate implementation
        let mut reader = WavReader::open(input_path)
            .map_err(|e| AppError::Audio(format!("Failed to open audio file: {}", e)))?;
        
        let spec = reader.spec();
        let mut writer = WavWriter::create(output_path, spec)
            .map_err(|e| AppError::Audio(format!("Failed to create output file: {}", e)))?;
        
        let threshold = 0.02; // Adjust based on noise level
        
        let samples: std::result::Result<Vec<_>, _> = reader.samples::<i16>().collect();
        let samples = samples.map_err(|e| AppError::Audio(format!("Failed to read samples: {}", e)))?;
        
        for sample in samples {
            let normalized = sample as f32 / 32768.0;
            let processed = if normalized.abs() < threshold {
                0i16
            } else {
                sample
            };
            
            writer.write_sample(processed)
                .map_err(|e| AppError::Audio(format!("Failed to write sample: {}", e)))?;
        }
        
        writer.finalize()
            .map_err(|e| AppError::Audio(format!("Failed to finalize output: {}", e)))?;
        
        Ok(())
    }
    
    /// Get audio file information
    pub fn get_audio_info(&self, path: &Path) -> Result<AudioInfo> {
        let reader = WavReader::open(path)
            .map_err(|e| AppError::Audio(format!("Failed to open audio file: {}", e)))?;
        
        let spec = reader.spec();
        let sample_count = reader.len();
        let duration_seconds = sample_count as f64 / (spec.sample_rate as f64 * spec.channels as f64);
        let file_size = std::fs::metadata(path)?.len();
        
        Ok(AudioInfo {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            bits_per_sample: spec.bits_per_sample,
            duration_seconds,
            file_size_bytes: file_size,
        })
    }
    
    // Helper methods
    
    fn deinterleave_samples(&self, samples: &[f32], channels: u16) -> Vec<Vec<f32>> {
        let mut channel_samples = vec![Vec::new(); channels as usize];
        
        for (i, &sample) in samples.iter().enumerate() {
            let channel = i % channels as usize;
            channel_samples[channel].push(sample);
        }
        
        channel_samples
    }
    
    fn mix_to_mono(&self, channels: &[Vec<f32>]) -> Vec<f32> {
        let len = channels[0].len();
        let mut mono = vec![0.0; len];
        
        for i in 0..len {
            let sum: f32 = channels.iter().map(|ch| ch[i]).sum();
            mono[i] = sum / channels.len() as f32;
        }
        
        mono
    }
    
    fn resample(&self, input: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Nearest,
            oversampling_factor: 160,
            window: WindowFunction::BlackmanHarris2,
        };
        
        let mut resampler = SincFixedIn::<f32>::new(
            to_rate as f64 / from_rate as f64,
            2.0,
            params,
            input.len(),
            1,
        ).map_err(|e| AppError::Audio(format!("Failed to create resampler: {}", e)))?;
        
        let input_frames = vec![input.to_vec()];
        let output_frames = resampler.output_frames_next();
        let mut output = vec![vec![0.0f32; output_frames]; 1];
        
        resampler.process_into_buffer(&input_frames, &mut output, None)
            .map_err(|e| AppError::Audio(format!("Resampling failed: {}", e)))?;
        
        Ok(output[0].clone())
    }
}

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub duration_seconds: f64,
    pub file_size_bytes: u64,
}