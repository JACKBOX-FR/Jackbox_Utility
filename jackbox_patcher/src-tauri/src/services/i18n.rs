use std::{collections::HashMap, fs, path::Path};

pub struct I18n(pub HashMap<String, String>);
impl I18n {
    pub fn load(dir: &Path, lang: &str) -> Self {
        let read = |l: &str| fs::read_to_string(dir.join("langs").join(format!("{l}.toml"))).ok();
        let txt = read(lang).or_else(|| read("en")).unwrap_or_default(); // langue absente -> anglais
        Self(toml::from_str(&txt).unwrap_or_default())
    }
    pub fn t(&self, key: &str, args: &[&str]) -> String {
        let mut s = self.0.get(key).cloned().unwrap_or_else(|| key.to_string());
        for (i, a) in args.iter().enumerate() { s = s.replace(&format!("{{{i}}}"), a); }
        s
    }
}
