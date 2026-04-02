use anyhow::{Context, Result};
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct Transcriber {
    ctx: WhisperContext,
}

impl Transcriber {
    pub fn new(model_path: &Path) -> Result<Self> {
        let mut ctx_params = WhisperContextParameters::default();
        // Activer GPU Metal (Apple Silicon)
        ctx_params.use_gpu(true);

        let ctx = WhisperContext::new_with_params(
            model_path.to_str().context("Chemin du modele invalide")?,
            ctx_params,
        )
        .map_err(|e| anyhow::anyhow!("Impossible de charger le modele Whisper: {}", e))?;

        Ok(Self { ctx })
    }

    pub fn transcribe(&self, audio: &[f32]) -> Result<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Langue francaise forcee (pas de detection auto = plus rapide)
        params.set_language(Some("fr"));
        params.set_translate(false);

        // Performance : utiliser tous les coeurs disponibles
        params.set_n_threads(num_cpus());

        // Desactiver les sorties inutiles
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Mode segment unique = plus rapide pour la dictee courte
        params.set_single_segment(true);

        // Pas de tokens de contexte initial (plus rapide)
        params.set_no_context(true);

        // Seuils pour ignorer les silences et les hallucinations
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);

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

fn num_cpus() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4)
}
