//! Réécriture de `joinUrl` dans tous les jbg.config.jet (remplace la valeur, ou ajoute la propriété si absente).
use regex::Regex;
use std::{fs, path::Path};
use walkdir::WalkDir;

#[derive(Default)]
pub struct Outcome { pub changed: usize, pub errors: Vec<String> }

/// Normalise l'adresse (retire http(s):// et `/` final) et refuse tout caractère risqué.
pub fn clean_url(input: &str) -> anyhow::Result<String> {
    let t = input.trim();
    let t = t.strip_prefix("https://").or_else(|| t.strip_prefix("http://")).unwrap_or(t).trim_end_matches('/');
    if !Regex::new(r"^[A-Za-z0-9._\-]+(:\d{1,5})?(/[A-Za-z0-9._~\-/%]*)?$")?.is_match(t) { anyhow::bail!("adresse invalide : {input}"); }
    Ok(t.to_string())
}

/// Remplace (ou ajoute) la valeur de `key`. Retourne le nouveau contenu, ou None si rien à changer / format non géré.
/// Gère : fichier multi-lignes ou minifié, clés avec ou sans guillemets, CRLF, BOM. La clé doit être entière
/// (`serverUrl` ne touche pas `apiServerUrl`).
pub fn apply_key(src: &str, key: &str, url: &str) -> Option<String> {
    let (bom, body) = match src.strip_prefix('\u{feff}') { Some(b) => ("\u{feff}", b), None => ("", src) };
    let re = Regex::new(&format!(r#"(^|[^A-Za-z0-9_])(["']?{}["']?\s*:\s*)("[^"\r\n]*"|'[^'\r\n]*'|[^,}}\r\n]*[^,}}\s])"#, regex::escape(key))).unwrap();
    let out = if re.is_match(body) {
        re.replace_all(body, |c: &regex::Captures| format!("{}{}\"{}\"", &c[1], &c[2], url)).into_owned()
    } else { insert(body, key, url)? };
    let out = format!("{bom}{out}");
    if out == src { None } else { Some(out) }
}
#[cfg(test)]
fn apply(src: &str, url: &str) -> Option<String> { apply_key(src, "joinUrl", url) }

/// Ajoute la propriété juste après la première `{` (toujours valide : pas de virgule orpheline).
fn insert(body: &str, name: &str, url: &str) -> Option<String> {
    let i = body.find('{')?;
    let after = &body[i + 1..];
    let nl = if body.contains("\r\n") { "\r\n" } else { "\n" };
    let quoted = Regex::new(r#""[A-Za-z_][A-Za-z0-9_]*"\s*:"#).unwrap().is_match(body);
    let key = if quoted { format!("\"{name}\"") } else { name.to_string() };
    let comma = if after.trim_start().starts_with('}') { "" } else { "," };
    let prop = if body.contains('\n') { format!("{nl}  {key}: \"{url}\"{comma}") } else { format!("{key}:\"{url}\"{comma}") };
    Some(format!("{}{}{}", &body[..=i], prop, after))
}

/// Applique plusieurs clés (ex. joinUrl + serverUrl) à chaque jbg.config.jet, en une seule écriture par fichier.
pub fn set_keys(game_dir: &str, kv: &[(&str, &str)]) -> Outcome {
    let mut o = Outcome::default();
    if game_dir.trim().is_empty() || !Path::new(game_dir).is_dir() { o.errors.push(format!("dossier introuvable : '{game_dir}'")); return o; }
    for e in WalkDir::new(game_dir).follow_links(false).into_iter().filter_map(Result::ok) {
        if e.file_name() != "jbg.config.jet" { continue; }
        let r = (|| -> anyhow::Result<bool> {
            let src = String::from_utf8(fs::read(e.path())?).map_err(|_| anyhow::anyhow!("pas de l'UTF-8"))?;
            let mut cur = src.clone();
            for (k, v) in kv { if let Some(n) = apply_key(&cur, k, v) { cur = n; } }
            if cur == src { return Ok(false); }
            let tmp = e.path().with_extension("jet.jbx-tmp");
            fs::write(&tmp, cur)?; fs::rename(&tmp, e.path())?; // écriture atomique
            Ok(true)
        })();
        match r { Ok(true) => o.changed += 1, Ok(false) => {}, Err(x) => o.errors.push(format!("{} : {x}", e.path().display())) }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn replace_multiline_keeps_comma() {
        assert_eq!(apply("{\n  joinUrl: \"a\",\n  x: 1\n}", "b").unwrap(), "{\n  joinUrl: \"b\",\n  x: 1\n}");
    }
    #[test] fn replace_minified_json() {
        assert_eq!(apply("{\"x\":1,\"joinUrl\":\"jackbox.tv\",\"y\":2}", "h:80").unwrap(), "{\"x\":1,\"joinUrl\":\"h:80\",\"y\":2}");
    }
    #[test] fn replace_last_property_no_comma() {
        assert_eq!(apply("{\"x\":1,\"joinUrl\":\"a\"}", "b").unwrap(), "{\"x\":1,\"joinUrl\":\"b\"}");
    }
    #[test] fn insert_json_quoted() { assert_eq!(apply("{\n  \"x\": 1\n}", "h").unwrap(), "{\n  \"joinUrl\": \"h\",\n  \"x\": 1\n}"); }
    #[test] fn insert_js_unquoted() { assert!(apply("{\n  x: 1\n}", "h").unwrap().contains("joinUrl: \"h\",")); }
    #[test] fn insert_empty_object() { assert_eq!(apply("{}", "h").unwrap(), "{joinUrl:\"h\"}"); }
    #[test] fn crlf_and_bom_preserved() {
        let r = apply("\u{feff}{\r\n  \"x\": 1\r\n}", "h").unwrap();
        assert!(r.starts_with('\u{feff}') && r.contains("\r\n  \"joinUrl\": \"h\",\r\n") && !r.contains("\n\n"));
    }
    #[test] fn idempotent_and_no_object() { assert!(apply("{\"joinUrl\":\"h\"}", "h").is_none()); assert!(apply("no braces", "h").is_none()); }
    #[test] fn server_url_and_prefix_safety() {
        let r = apply_key("{\"apiServerUrl\":\"a\",\"serverUrl\":\"ecast.jackboxgames.com\",\"joinUrl\":\"j\"}", "serverUrl", "jonahbox.local").unwrap();
        assert_eq!(r, "{\"apiServerUrl\":\"a\",\"serverUrl\":\"jonahbox.local\",\"joinUrl\":\"j\"}");
        assert!(apply_key("{\"x\":1}", "serverUrl", "h").unwrap().starts_with("{\"serverUrl\":\"h\","));
    }
    #[test] fn url_validation() {
        assert_eq!(clean_url("https://a.b:8080/").unwrap(), "a.b:8080");
        assert!(clean_url("a b").is_err()); assert!(clean_url("x\"y").is_err()); assert!(clean_url("").is_err());
    }
}
