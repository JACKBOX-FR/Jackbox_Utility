//! Contrôleur d'installation "tmp3" (port de tmp3_install_controller.dart) : remplace les URLs UTF-16 dans l'exécutable.
use std::{fs, path::Path};

const ORIG_CTRL: &str = "jackbox.tv";
const ORIG_SERVICE: &str = "https://api.jackboxgames.com/arcade";

pub fn run(game_dir: &Path, controller_url: &str, service_url: &str) -> anyhow::Result<()> {
    let exe = game_dir.join("TMP3/Binaries/Win64/TMP3-Win64-Shipping.exe");
    let mut b = fs::read(&exe)?;
    patch(&mut b, controller_url, service_url)?;
    fs::write(exe, b)?;
    Ok(())
}

pub fn patch(b: &mut [u8], ctrl: &str, service: &str) -> anyhow::Result<()> {
    if ctrl.len() > ORIG_CTRL.len() || service.len() > ORIG_SERVICE.len() { anyhow::bail!("URL TMP3 trop longue"); }
    replace(b, ORIG_SERVICE, service)?;
    replace(b, ORIG_CTRL, ctrl)
}

fn u16le(s: &str) -> Vec<u8> { s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect() }
fn find_all(h: &[u8], n: &[u8]) -> Vec<usize> {
    if n.is_empty() || h.len() < n.len() { return vec![]; }
    (0..=h.len() - n.len()).filter(|&i| &h[i..i + n.len()] == n).collect()
}
fn replace(b: &mut [u8], from: &str, to: &str) -> anyhow::Result<()> {
    let (f, t) = (u16le(from), u16le(to));
    let offs = find_all(b, &f);
    if offs.is_empty() && find_all(b, &t).len() == 1 { return Ok(()); } // déjà patché
    if offs.len() != 1 { anyhow::bail!("TMP3 : {} occurrence(s) de {from}, 1 attendue", offs.len()); }
    let o = offs[0];
    b[o..o + f.len()].fill(0);
    b[o..o + t.len()].copy_from_slice(&t);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patches_once() {
        let mut b = [vec![1u8; 4], u16le(ORIG_SERVICE), vec![2; 4], u16le(ORIG_CTRL), vec![3; 4]].concat();
        patch(&mut b, "ex.com", "https://a.b/c").unwrap();
        assert_eq!(find_all(&b, &u16le("ex.com")).len(), 1);
        patch(&mut b, "ex.com", "https://a.b/c").unwrap(); // idempotent
    }
}
