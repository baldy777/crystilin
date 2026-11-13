use std::process::Command;

use crate::errors::ErrorCrystilin;
use crate::models::{Conexion, Disco, LsblkSalida, Medio, NodoLsblk};
/// Convierte el texto JSON de lsblk en structs.
pub fn parsear(json: &str) -> Result<LsblkSalida, ErrorCrystilin> {
    let salida = serde_json::from_str(json)?;
    Ok(salida)
}

/// Ejecuta `lsblk` en el sistema y devuelve los discos reales.
pub fn listar_discos() -> Result<Vec<Disco>, ErrorCrystilin> {
    let resultado = Command::new("lsblk")
        .args([
            "--json",
            "--bytes",
            "--output",
            "NAME,PATH,MODEL,SIZE,TYPE,TRAN,ROTA,RM,MOUNTPOINTS",
        ])
        .output() // espera a que termine y captura stdout y stderr
        .map_err(ErrorCrystilin::NoSePudoEjecutar)?;

    if !resultado.status.success() {
        return Err(ErrorCrystilin::LsblkFallo {
            codigo: resultado.status.code(),
            stderr: String::from_utf8_lossy(&resultado.stderr).trim().to_string(),
        });
    }

    // stdout llega como bytes (Vec<u8>); lo convertimos a texto.
    let texto = String::from_utf8_lossy(&resultado.stdout);
    let salida = parsear(&texto)?;
    Ok(a_discos(&salida))
}

pub fn a_discos(salida: &LsblkSalida) -> Vec<Disco> {
    salida
        .blockdevices
        .iter()
        .filter(|n| n.tipo == "disk" && !n.name.starts_with("zram"))
        .map(|n| Disco {
            ruta: n.path.clone(),
            modelo: n.model.clone(),
            tamano_bytes: n.size,
            conexion: conexion_de(n),
            medio: medio_de(n),
            removible: n.rm,
            protegido: contiene_raiz(n),
        })
        .collect()
}

fn conexion_de(n: &NodoLsblk) -> Conexion {
    match n.tran.as_deref() {
        Some("usb") => Conexion::Usb,
        Some("nvme") => Conexion::Nvme,
        Some("sata") | Some("ata") => Conexion::Sata,
        _ => Conexion::Otra,
    }
}

fn medio_de(n: &NodoLsblk) -> Medio {
    if n.rota { Medio::Hdd } else { Medio::Ssd }
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
        assert_eq!(d.conexion, Conexion::Nvme);
        assert_eq!(d.medio, Medio::Ssd);
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
        assert_eq!(discos[0].conexion, Conexion::Usb);
        assert!(discos[0].removible);
        assert!(!discos[0].protegido);
    }

        const FIXTURE_HDD_USB: &str =
        include_str!("../../../../tests/fixtures/lsblk_nvme_y_hdd_usb.json");

    #[test]
    fn hdd_en_usb_no_esta_protegido_y_el_nvme_si() {
        let discos = a_discos(&parsear(FIXTURE_HDD_USB).unwrap());
        assert_eq!(discos.len(), 2);

        let sdb = discos.iter().find(|d| d.ruta == "/dev/sdb").unwrap();
        assert_eq!(sdb.conexion, Conexion::Usb);
        assert_eq!(sdb.medio, Medio::Hdd);
        assert!(!sdb.protegido);

        let nvme = discos.iter().find(|d| d.ruta == "/dev/nvme0n1").unwrap();
        assert!(nvme.protegido);
    }
}