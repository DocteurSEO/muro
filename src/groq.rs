use anyhow::{bail, Result};
use serde_json::{json, Value};

const PROMPT_CLEANUP: &str = "Corrige uniquement la ponctuation, les majuscules et la grammaire. \
    Ne change ni le sens, ni le style, ni les mots. Renvoie uniquement le texte corrigé, rien d'autre.";

const PROMPT_TRANSLATE: &str = "Tu es un traducteur. Traduis le texte ci-dessous. \
    Si une langue cible est mentionnée (ex: 'en anglais'), traduis dans cette langue. \
    Sinon, traduis en anglais. Renvoie uniquement la traduction, rien d'autre.";

const PROMPT_CORRECT: &str = "Corrige toutes les fautes de grammaire, orthographe et syntaxe. \
    Renvoie uniquement le texte corrigé, rien d'autre.";

const PROMPT_IMPROVE: &str = "Améliore ce texte : corrige la grammaire, la ponctuation, \
    et reformule pour que ce soit plus clair et fluide. \
    Renvoie uniquement le texte amélioré, rien d'autre.";

fn call(system_prompt: &str, text: &str) -> Result<String> {
    let api_key = std::env::var("GROQ_API_KEY").unwrap_or_default();

    if api_key.is_empty() {
        return Ok(text.to_string());
    }

    let body = json!({
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": text }
        ],
        "model": "openai/gpt-oss-120b",
        "temperature": 0.2,
        "max_completion_tokens": 2048,
        "reasoning_effort": "low",
        "stream": false
    });

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .timeout(std::time::Duration::from_secs(15))
        .send()?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        bail!("Groq API erreur {}: {}", status, body);
    }

    let json: Value = resp.json()?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or(text)
        .trim()
        .to_string();

    if content.is_empty() {
        return Ok(text.to_string());
    }

    Ok(content)
}

pub fn cleanup(text: &str) -> Result<String> {
    call(PROMPT_CLEANUP, text)
}

pub fn translate(text: &str) -> Result<String> {
    call(PROMPT_TRANSLATE, text)
}

pub fn correct(text: &str) -> Result<String> {
    call(PROMPT_CORRECT, text)
}

pub fn improve(text: &str) -> Result<String> {
    call(PROMPT_IMPROVE, text)
}
