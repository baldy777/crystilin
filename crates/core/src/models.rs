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