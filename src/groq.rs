use anyhow::{bail, Result};
use serde_json::{json, Value};

const PROMPT_CLEANUP: &str = "\
Tu es un moteur de post-traitement pour de la dictée vocale en français. \
Le texte vient d'un modèle de reconnaissance vocale et contient des imperfections typiques.

Règles strictes :
1. Supprime les hésitations et tics de langage (euh, hum, ben, bah, genre, en fait, du coup, voilà, quoi)
2. Restaure la ponctuation correcte (points, virgules, points d'interrogation, points d'exclamation)
3. Mets les majuscules aux débuts de phrases et aux noms propres
4. Normalise les acronymes en majuscules (API, JSON, HTTP, SQL, CSS, HTML, etc.)
5. Corrige les homophones courants (ces/ses/c'est/s'est, a/à, ou/où, son/sont, etc.)
6. Corrige les erreurs de reconnaissance vocale évidentes selon le contexte
7. Ne change PAS le sens, le style, ni le vocabulaire de l'utilisateur
8. Ne rajoute PAS de contenu, ne reformule PAS

Renvoie UNIQUEMENT le texte nettoyé, sans explication ni commentaire.";

const PROMPT_TRANSLATE: &str = "\
Tu es un traducteur professionnel. \
Traduis le texte ci-dessous de manière naturelle et idiomatique. \
Si une langue cible est mentionnée (ex: 'en anglais', 'en espagnol'), traduis dans cette langue. \
Sinon, traduis en anglais. \
Renvoie UNIQUEMENT la traduction, rien d'autre.";

const PROMPT_CORRECT: &str = "\
Corrige toutes les fautes de grammaire, orthographe, syntaxe et ponctuation. \
Normalise les acronymes en majuscules. \
Renvoie UNIQUEMENT le texte corrigé, rien d'autre.";

const PROMPT_IMPROVE: &str = "\
Améliore ce texte : corrige la grammaire, la ponctuation, les fautes. \
Reformule les phrases maladroites pour que ce soit plus clair et fluide. \
Normalise les acronymes. Supprime les hésitations. \
Conserve le sens et le ton de l'auteur. \
Renvoie UNIQUEMENT le texte amélioré, rien d'autre.";

// Modele rapide pour le cleanup (dictee quotidienne)
const MODEL_FAST: &str = "llama-3.3-70b-versatile";
// Modele puissant pour les commandes complexes (traduction, amelioration)
const MODEL_SMART: &str = "openai/gpt-oss-120b";

fn call(system_prompt: &str, text: &str, model: &str) -> Result<String> {
    let api_key = std::env::var("GROQ_API_KEY").unwrap_or_default();

    if api_key.is_empty() {
        return Ok(text.to_string());
    }

    let mut body = json!({
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": text }
        ],
        "model": model,
        "temperature": 0.1,
        "max_completion_tokens": 2048,
        "stream": false
    });

    // reasoning_effort seulement pour les modeles qui le supportent
    if model == MODEL_SMART {
        body["reasoning_effort"] = json!("low");
    }

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

/// Cleanup rapide pour la dictee (modele rapide)
pub fn cleanup(text: &str) -> Result<String> {
    call(PROMPT_CLEANUP, text, MODEL_FAST)
}

/// Traduction (modele puissant)
pub fn translate(text: &str) -> Result<String> {
    call(PROMPT_TRANSLATE, text, MODEL_SMART)
}

/// Correction (modele rapide)
pub fn correct(text: &str) -> Result<String> {
    call(PROMPT_CORRECT, text, MODEL_FAST)
}

/// Amelioration (modele puissant)
pub fn improve(text: &str) -> Result<String> {
    call(PROMPT_IMPROVE, text, MODEL_SMART)
}
