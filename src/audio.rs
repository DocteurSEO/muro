use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct Recorder {
    stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    device_sample_rate: u32,
}

impl Recorder {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("Pas de microphone detecte")?;

        let config = device
            .default_input_config()
            .context("Impossible d'obtenir la config audio")?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        let stream_config: cpal::StreamConfig = config.into();

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();

        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let Ok(mut buf) = buffer_clone.lock() else { return };
                if channels > 1 {
                    for chunk in data.chunks(channels) {
                        let mono: f32 = chunk.iter().sum::<f32>() / channels as f32;
                        buf.push(mono);
                    }
                } else {
                    buf.extend_from_slice(data);
                }
            },
            |err| tracing::error!("Erreur flux audio: {}", err),
            None,
        )?;

        stream.play()?;

        Ok(Self {
            stream,
            buffer,
            device_sample_rate: sample_rate,
        })
    }

    /// Arrete l'enregistrement et retourne l'audio en 16kHz mono f32
    pub fn stop(self) -> Vec<f32> {
        drop(self.stream);
        let raw = self.buffer.lock().unwrap_or_else(|e| e.into_inner()).clone();
        resample(&raw, self.device_sample_rate, 16000)
    }
}

/// Resampler par interpolation lineaire (suffisant pour la parole)
fn resample(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (input.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        let frac = (src_idx - idx as f64) as f32;

        let sample = if idx + 1 < input.len() {
            input[idx] * (1.0 - frac) + input[idx + 1] * frac
        } else {
            input[idx.min(input.len() - 1)]
        };
        output.push(sample);
    }

    output
}
