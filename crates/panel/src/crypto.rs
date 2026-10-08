//! Cifrado de los valores secretos que guarda el historial de cambios.
//!
//! ChaCha20-Poly1305 con una clave de 32 bytes en un fichero propio
//! (`RP_HISTORY_KEY`, permisos 600). Cada sellado lleva un nonce aleatorio de
//! 12 bytes delante del texto cifrado.

use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit},
};
use std::{io::Write, path::Path};

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

/// Clave del historial.
#[derive(Clone)]
pub struct HistoryKey {
    cipher: ChaCha20Poly1305,
}

impl std::fmt::Debug for HistoryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistoryKey(***)")
    }
}

impl HistoryKey {
    /// Lee la clave de `path`; si no existe, la crea (32 bytes aleatorios,
    /// permisos 600). Un fichero que no tenga exactamente 32 bytes o que
    /// otros usuarios puedan leer es un error: nunca se sustituye una clave
    /// existente. Si otro proceso la crea a la vez, se lee la suya.
    pub fn load_or_create(path: &Path) -> Result<HistoryKey, String> {
        match Self::load(path)? {
            Some(k) => Ok(k),
            None => {
                let mut key = [0u8; KEY_LEN];
                getrandom::fill(&mut key).map_err(|e| format!("generador aleatorio: {e}"))?;
                match create_private(path, &key) {
                    Ok(()) => Ok(Self::from_bytes(&key)),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Self::load(path)?
                        .ok_or_else(|| {
                            format!("la clave del historial {} desapareció", path.display())
                        }),
                    Err(e) => Err(format!(
                        "no se puede crear la clave del historial {}: {e}",
                        path.display()
                    )),
                }
            }
        }
    }

    /// Lee una clave existente; `None` si el fichero no existe.
    fn load(path: &Path) -> Result<Option<HistoryKey>, String> {
        let shown = path.display();
        match std::fs::read(path) {
            Ok(bytes) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = std::fs::metadata(path)
                        .map_err(|e| format!("clave del historial {shown}: {e}"))?
                        .permissions()
                        .mode();
                    if mode & 0o077 != 0 {
                        return Err(format!(
                            "la clave del historial {shown} tiene permisos {:o}; deben ser 600",
                            mode & 0o777
                        ));
                    }
                }
                let key: [u8; KEY_LEN] = bytes.try_into().map_err(|b: Vec<u8>| {
                    format!(
                        "la clave del historial {shown} tiene {} bytes (deben ser {KEY_LEN})",
                        b.len()
                    )
                })?;
                Ok(Some(Self::from_bytes(&key)))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!(
                "no se puede leer la clave del historial {shown} (RP_HISTORY_KEY): {e}"
            )),
        }
    }

    fn from_bytes(key: &[u8; KEY_LEN]) -> HistoryKey {
        HistoryKey {
            cipher: ChaCha20Poly1305::new(Key::from_slice(key)),
        }
    }

    /// Cifra `plain`: nonce aleatorio (12 bytes) seguido del texto cifrado.
    pub fn seal(&self, plain: &[u8]) -> Vec<u8> {
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce).expect("generador aleatorio del sistema");
        let sealed = self
            .cipher
            .encrypt(Nonce::from_slice(&nonce), plain)
            .expect("ChaCha20-Poly1305 cifra cualquier longitud razonable");
        let mut out = nonce.to_vec();
        out.extend(sealed);
        out
    }

    /// Descifra lo que produjo [`HistoryKey::seal`]; `None` si está dañado o
    /// se cifró con otra clave.
    pub fn open(&self, sealed: &[u8]) -> Option<Vec<u8>> {
        if sealed.len() < NONCE_LEN {
            return None;
        }
        let (nonce, data) = sealed.split_at(NONCE_LEN);
        self.cipher.decrypt(Nonce::from_slice(nonce), data).ok()
    }
}

/// Crea `path` con `key` de forma atómica: se escribe un temporal (600) y
/// se enlaza con su nombre definitivo, así nadie ve el fichero a medias. Si
/// `path` ya existe, `AlreadyExists`.
fn create_private(path: &Path, key: &[u8]) -> std::io::Result<()> {
    let mut sufijo = [0u8; 8];
    getrandom::fill(&mut sufijo).map_err(std::io::Error::other)?;
    let nombre = format!(
        ".{}.{:016x}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("clave"),
        u64::from_le_bytes(sufijo)
    );
    let tmp = path.with_file_name(nombre);
    let r = (|| {
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp)?;
        f.write_all(key)?;
        f.sync_all()?;
        std::fs::hard_link(&tmp, path)
    })();
    let _ = std::fs::remove_file(&tmp);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crea_la_clave_con_permisos_600_y_la_reutiliza() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.key");
        let k1 = HistoryKey::load_or_create(&path).unwrap();
        assert_eq!(std::fs::read(&path).unwrap().len(), 32);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let sealed = k1.seal(b"clave wifi");
        let k2 = HistoryKey::load_or_create(&path).unwrap();
        assert_eq!(k2.open(&sealed).as_deref(), Some(&b"clave wifi"[..]));
    }

    #[test]
    fn sellado_con_nonce_aleatorio_y_autenticado() {
        let dir = tempfile::tempdir().unwrap();
        let k = HistoryKey::load_or_create(&dir.path().join("k")).unwrap();
        let a = k.seal(b"secreto");
        let b = k.seal(b"secreto");
        assert_ne!(a, b, "el nonce debe cambiar en cada sellado");
        assert_eq!(a.len(), 12 + 7 + 16);
        assert!(!a.windows(7).any(|w| w == b"secreto"));
        let mut roto = a.clone();
        *roto.last_mut().unwrap() ^= 1;
        assert_eq!(k.open(&roto), None);
        assert_eq!(k.open(&a[..5]), None);
        let otra = HistoryKey::load_or_create(&dir.path().join("otra")).unwrap();
        assert_eq!(otra.open(&a), None);
        assert_eq!(k.open(&k.seal(b"")).as_deref(), Some(&b""[..]));
    }

    #[cfg(unix)]
    #[test]
    fn clave_con_permisos_abiertos_es_error() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("k");
        HistoryKey::load_or_create(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let e = HistoryKey::load_or_create(&path).unwrap_err();
        assert!(e.contains("644"), "{e}");
        assert_eq!(std::fs::read(&path).unwrap().len(), 32, "no se toca");
    }

    #[test]
    fn creacion_simultanea_comparte_la_clave() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("k");
        let hilos: Vec<_> = (0..8)
            .map(|_| {
                let path = path.clone();
                std::thread::spawn(move || HistoryKey::load_or_create(&path))
            })
            .collect();
        let claves: Vec<HistoryKey> = hilos
            .into_iter()
            .map(|h| h.join().unwrap().unwrap())
            .collect();
        let sellado = claves[0].seal(b"x");
        for k in &claves {
            assert_eq!(k.open(&sellado).as_deref(), Some(&b"x"[..]));
        }
    }

    #[test]
    fn clave_de_longitud_incorrecta_es_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("k");
        std::fs::write(&path, b"corta").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let e = HistoryKey::load_or_create(&path).unwrap_err();
        assert!(e.contains("5 bytes"), "{e}");
        assert_eq!(std::fs::read(&path).unwrap(), b"corta");
    }
}
