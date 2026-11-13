use crate::errors::ErrorCrystilin;
use crate::models::LsblkSalida;

/// Convierte el texto JSON de lsblk en structs.
pub fn parsear(json: &str) -> Result<LsblkSalida, ErrorCrystilin> {
    let salida = serde_json::from_str(json)?;
    Ok(salida)
}

#[cfg(test)]
mod tests {
    use super::*;

    // include_str! mete el archivo en el binario de test en tiempo de compilación.
    // La ruta es relativa a ESTE archivo: sube 4 niveles hasta la raíz del repo.
    const FIXTURE: &str = include_str!("../../../../tests/fixtures/lsblk_nvme_luks.json");

    #[test]
    fn parsea_fixture_real() {
        let salida = parsear(FIXTURE).expect("el fixture debe parsear");

        // zram0 + nvme0n1
        assert_eq!(salida.blockdevices.len(), 2);

        let nvme = &salida.blockdevices[1];
        assert_eq!(nvme.name, "nvme0n1");
        assert_eq!(nvme.model.as_deref(), Some("SAMSUNG MZAL8512HDLU-00BLL"));
        assert_eq!(nvme.children.len(), 3); // p1, p2, p3
    }

    #[test]
    fn json_roto_devuelve_error() {
        assert!(parsear("esto no es json").is_err());
    }
}