use crate::errors::ErrorCrystilin;
use crate::models::{Disco, LsblkSalida, NodoLsblk, TipoDisco};

/// Convierte el texto JSON de lsblk en structs.
pub fn parsear(json: &str) -> Result<LsblkSalida, ErrorCrystilin> {
    let salida = serde_json::from_str(json)?;
    Ok(salida)
}

/// Convierte la salida cruda de lsblk en la lista de discos reales.
pub fn a_discos(salida: &LsblkSalida) -> Vec<Disco> {
    salida
        .blockdevices
        .iter()
        // Solo discos, y fuera los de memoria comprimida (zram0 es swap, no un disco).
        .filter(|n| n.tipo == "disk" && !n.name.starts_with("zram"))
        .map(|n| Disco {
            ruta: n.path.clone(),
            modelo: n.model.clone(),
            tamano_bytes: n.size,
            tipo: clasificar(n),
            removible: n.rm,
            protegido: contiene_raiz(n),
        })
        .collect()
}

/// Deduce el tipo de disco. El orden importa: un USB-SSD debe salir como USB.
fn clasificar(n: &NodoLsblk) -> TipoDisco {
    match n.tran.as_deref() {
        Some("usb") => TipoDisco::Usb,
        Some("nvme") => TipoDisco::Nvme,
        _ if n.rota => TipoDisco::Hdd,
        _ => TipoDisco::Ssd,
    }
}

/// ¿Este nodo, o cualquiera de sus descendientes, está montado en "/"?
/// Es recursiva: en tu equipo "/" está en disco -> partición -> volumen LUKS.
fn contiene_raiz(nodo: &NodoLsblk) -> bool {
    nodo.mountpoints
        .iter()
        .flatten() // descarta los None y desenvuelve los Some
        .any(|m| m.as_str() == "/")
        || nodo.children.iter().any(contiene_raiz)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ruta relativa a ESTE archivo: sube 4 niveles hasta la raíz del repo.
    const FIXTURE: &str = include_str!("../../../../tests/fixtures/lsblk_nvme_luks.json");

    #[test]
    fn parsea_fixture_real() {
        let salida = parsear(FIXTURE).expect("el fixture debe parsear");
        assert_eq!(salida.blockdevices.len(), 2); // zram0 + nvme0n1
        let nvme = &salida.blockdevices[1];
        assert_eq!(nvme.name, "nvme0n1");
        assert_eq!(nvme.model.as_deref(), Some("SAMSUNG MZAL8512HDLU-00BLL"));
        assert_eq!(nvme.children.len(), 3);
    }

    #[test]
    fn json_roto_devuelve_error() {
        assert!(parsear("esto no es json").is_err());
    }

    #[test]
    fn filtra_zram_y_protege_el_disco_del_sistema() {
        let salida = parsear(FIXTURE).unwrap();
        let discos = a_discos(&salida);

        assert_eq!(discos.len(), 1); // zram0 quedó fuera
        let d = &discos[0];
        assert_eq!(d.ruta, "/dev/nvme0n1");
        assert_eq!(d.tipo, TipoDisco::Nvme);
        assert!(d.protegido, "el disco con / debe salir protegido");
    }

    #[test]
    fn usb_sin_raiz_no_esta_protegido() {
        // JSON escrito a mano: un USB con mountpoints [null] (formato de util-linux antiguo).
        let json = r#"{
            "blockdevices": [{
                "name": "sda", "path": "/dev/sda", "model": "Cruzer",
                "size": 16000000000, "type": "disk", "tran": "usb",
                "rota": false, "rm": true, "mountpoints": [null],
                "children": [{
                    "name": "sda1", "path": "/dev/sda1", "model": null,
                    "size": 15999000000, "type": "part", "tran": "usb",
                    "rota": false, "rm": true,
                    "mountpoints": ["/run/media/clarence/USB"]
                }]
            }]
        }"#;

        let discos = a_discos(&parsear(json).unwrap());
        assert_eq!(discos.len(), 1);
        assert_eq!(discos[0].tipo, TipoDisco::Usb);
        assert!(discos[0].removible);
        assert!(!discos[0].protegido);
    }
}