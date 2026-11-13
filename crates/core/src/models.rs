use serde::Deserialize;

/// Raíz del JSON que devuelve `lsblk --json`.
#[derive(Debug, Deserialize)]
pub struct LsblkSalida {
    pub blockdevices: Vec<NodoLsblk>,
}

/// Un nodo del árbol de lsblk: puede ser disco, partición, volumen cifrado, etc.
#[derive(Debug, Deserialize)]
pub struct NodoLsblk {
    pub name: String,
    pub path: String,
    pub model: Option<String>, // null en particiones y en algunos USB

    pub size: u64, // bytes (por eso pedimos --bytes)

    // `type` es palabra reservada en Rust, así que el campo se llama `tipo`
    // y le decimos a serde que en el JSON se llama "type".
    #[serde(rename = "type")]
    pub tipo: String,

    pub tran: Option<String>, // "nvme", "sata", "usb"... o null
    pub rota: bool,           // true = disco giratorio (HDD)
    pub rm: bool,             // true = removible

    // Cada elemento puede ser null en versiones antiguas de util-linux.
    pub mountpoints: Vec<Option<String>>,

    // Si el campo no viene en el JSON, usa Vec vacío (Default).
    #[serde(default)]
    pub children: Vec<NodoLsblk>,
}

/// Cómo se conecta el disco al equipo.
#[derive(Debug, Clone, PartialEq)]
pub enum Conexion {
    Nvme,
    Sata,
    Usb,
    Otra,
}

/// De qué tecnología es el medio de almacenamiento.
#[derive(Debug, Clone, PartialEq)]
pub enum Medio {
    Hdd,
    Ssd,
}

/// Lo que ve el resto del programa. No sabe nada de lsblk.
#[derive(Debug, Clone)]
pub struct Disco {
    pub ruta: String, // "/dev/nvme0n1"
    pub modelo: Option<String>,
    pub tamano_bytes: u64,
    pub conexion: Conexion,
    pub medio: Medio,
    pub removible: bool, // poco fiable en discos externos
    /// true si este disco contiene el sistema (`/`): nunca se podrá tocar.
    pub protegido: bool,
}

use std::fmt;

impl fmt::Display for Conexion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let texto = match self {
            Conexion::Nvme => "NVMe",
            Conexion::Sata => "SATA",
            Conexion::Usb => "USB",
            Conexion::Otra => "Otra",
        };
        f.pad(texto)
    }
}

impl fmt::Display for Medio {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let texto = match self {
            Medio::Hdd => "HDD",
            Medio::Ssd => "SSD",
        };
        f.pad(texto)
    }
}