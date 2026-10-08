//! Couche de traduction du dump : un fichier par langue (translations/<langue>.toml), appliqué à la volée
//! sur les .js/.html/.json/.css servis. Le dump reste intact (donc mettable à jour sans perdre les traductions).
//! Format : clé = texte exact à remplacer (guillemets compris, pour viser le texte entier), valeur = remplacement.
use aho_corasick::{AhoCorasick, MatchKind};
use regex::Regex;
use std::{collections::{BTreeSet, HashMap}, fs, path::Path};

pub struct Translator { ac: Option<AhoCorasick>, to: Vec<String> }

impl Translator {
    pub fn empty() -> Self { Self { ac: None, to: vec![] } }
    pub fn is_empty(&self) -> bool { self.ac.is_none() }

    pub fn from_map(map: HashMap<String, String>) -> anyhow::Result<Self> {
        let (mut from, mut to): (Vec<String>, Vec<String>) = (vec![], vec![]);
        for (k, v) in map { if !k.is_empty() && !v.is_empty() { from.push(k); to.push(v); } } // valeur vide = pas encore traduit
        if from.is_empty() { return Ok(Self::empty()); }
        let ac = AhoCorasick::builder().match_kind(MatchKind::LeftmostLongest).build(&from)?;
        Ok(Self { ac: Some(ac), to })
    }

    pub fn load(app_dir: &Path, lang: &str) -> anyhow::Result<Self> {
        if lang.is_empty() { return Ok(Self::empty()); }
        let p = app_dir.join("translations").join(format!("{lang}.toml"));
        if !p.is_file() { return Ok(Self::empty()); }
        Self::from_map(toml::from_str(&fs::read_to_string(p)?)?)
    }

    pub fn apply(&self, s: &str) -> String {
        match &self.ac { Some(ac) => ac.replace_all(s, &self.to), None => s.to_string() }
    }
}

const NOISE: [&str; 12] = ["Failed to", "Socket", "RFC", "Encoder", "Decoder", "EventEmitter", "Cyclic", "prototype", "Invalid ", "Unhandled", "mock server", "option provided"];

/// Chaînes de texte probables d'un fichier JS (pour préparer une traduction). Retourne des lignes TOML `"\"texte\"" = ""`.
pub fn extract(js: &str) -> Vec<String> {
    let re = Regex::new(r#""([A-Z][^"\\\n<>{}]{3,80})""#).unwrap();
    let set: BTreeSet<String> = re.captures_iter(js).map(|c| c[1].to_string())
        .filter(|t| (t.contains(' ') || t.chars().all(|c| c.is_ascii_uppercase() || c == ' ')) && !NOISE.iter().any(|n| t.contains(n))).collect();
    set.into_iter().map(|t| {
        let raw = format!("\"{t}\"");
        format!("\"{}\" = \"\"", raw.replace('\\', "\\\\").replace('"', "\\\""))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn applies_longest_first_and_skips_empty() {
        let mut m = HashMap::new();
        m.insert("\"Thanks\"".to_string(), "\"Merci\"".to_string());
        m.insert("\"Thanks for playing!\"".to_string(), "\"Merci d'avoir joué !\"".to_string());
        m.insert("\"Todo\"".to_string(), "".to_string());
        let t = Translator::from_map(m).unwrap();
        assert_eq!(t.apply("a=\"Thanks for playing!\";b=\"Thanks\";c=\"Todo\""), "a=\"Merci d'avoir joué !\";b=\"Merci\";c=\"Todo\"");
        assert!(Translator::from_map(HashMap::new()).unwrap().is_empty());
    }
    #[test] fn extracts_text_not_noise() {
        let js = r#"x("Thanks for playing!");y("Failed to construct 'X'");z("PLAY");w("a b");q("Already read")"#;
        let e = extract(js);
        assert!(e.iter().any(|l| l.contains("Thanks for playing!") && l.ends_with("= \"\"")));
        assert!(e.iter().any(|l| l.contains("Already read")) && !e.iter().any(|l| l.contains("Failed to")));
        // la ligne produite est du TOML valide
        let toml_txt = e.join("\n"); let m: HashMap<String, String> = toml::from_str(&toml_txt).unwrap();
        assert!(m.contains_key("\"Thanks for playing!\""));
    }
}
