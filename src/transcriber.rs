use anyhow::{Context, Result};
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct Transcriber {
    ctx: WhisperContext,
}

impl Transcriber {
    pub fn new(model_path: &Path) -> Result<Self> {
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().context("Chemin du modele invalide")?,
            WhisperContextParameters::default(),
        )
        .map_err(|e| anyhow::anyhow!("Impossible de charger le modele Whisper: {}", e))?;

        Ok(Self { ctx })
    }

    pub fn transcribe(&self, audio: &[f32]) -> Result<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("fr"));
        params.set_translate(false);
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_realtime(false);
        params.set_single_segment(false);

        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| anyhow::anyhow!("Erreur creation state Whisper: {}", e))?;

        state
            .full(params, audio)
            .map_err(|e| anyhow::anyhow!("Erreur transcription: {}", e))?;

        let n_segments = state.full_n_segments();

        let mut text = String::new();
        for i in 0..n_segments {
            if let Some(segment) = state.get_segment(i) {
                if let Ok(s) = segment.to_str() {
                    text.push_str(s);
                }
            }
        }

        Ok(text.trim().to_string())
    }
}
