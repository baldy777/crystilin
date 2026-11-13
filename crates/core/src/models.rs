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

// ---------- SMART (modelo de dominio) ----------

/// Un atributo SMART de un disco ATA (HDD o SSD SATA).
#[derive(Debug, Clone, PartialEq)]
pub struct AtributoAta {
    pub id: u32,
    pub nombre: String,
    pub valor: u32,       // valor normalizado actual (100 = nuevo, baja con el desgaste)
    pub peor: u32,        // el peor valor que ha tenido
    pub umbral: u32,      // si `valor` cae a este nivel, el fabricante considera fallo
    pub raw_valor: u64,   // valor crudo: su significado depende del fabricante
    pub raw_texto: String // cómo lo imprime smartctl, p. ej. "8 (0 1)"
}

/// Salud de un disco NVMe (campos fijos del estándar NVMe).
#[derive(Debug, Clone, PartialEq)]
pub struct SaludNvme {
    pub advertencia_critica: u32,
    pub repuesto_disponible: u32,        // % de celdas de reserva que quedan
    pub umbral_repuesto: u32,
    pub porcentaje_usado: u32,           // desgaste: 0 = nuevo, 100 = vida útil agotada
    pub unidades_escritas: u64,
    pub unidades_leidas: u64,
    pub errores_de_medio: u64,
    pub apagados_inseguros: u64,
    pub ciclos_de_encendido: u64,
}

/// Lo que cambia según el tipo de disco: es un "union type" de TypeScript.
#[derive(Debug, Clone, PartialEq)]
pub enum DetalleSmart {
    Ata { atributos: Vec<AtributoAta> },
    Nvme(SaludNvme),
    /// Algunos USB no exponen SMART: hay que representarlo, no inventar datos.
    SinDatos,
}

/// Informe SMART unificado: lo único que verá el resto del programa.
#[derive(Debug, Clone, PartialEq)]
pub struct InformeSmart {
    pub modelo: Option<String>,
    pub serie: Option<String>,
    pub aprobado: Option<bool>,          // veredicto global del propio disco
    pub horas_encendido: Option<u64>,
    pub temperatura_c: Option<i32>,
    pub estado_smartctl: u32,            // exit_status: campo de bits, se interpreta en la Fase 3
    pub detalle: DetalleSmart,
}

impl InformeSmart {
    /// Sectores reasignados (solo discos ATA, atributo 5).
    pub fn sectores_reasignados(&self) -> Option<u64> {
        // `&self.detalle` PRESTA el campo (borrow): no lo movemos fuera de `self`.
        match &self.detalle {
            DetalleSmart::Ata { atributos } => atributos
                .iter()
                .find(|a| a.id == 5)
                // Solo los 16 bits bajos son el contador real (65544 -> 8).
                .map(|a| a.raw_valor & 0xFFFF),
            _ => None,
        }
    }

    /// Desgaste en % (por ahora solo NVMe; los SSD SATA lo veremos con el Kingston).
    pub fn desgaste_porcentaje(&self) -> Option<u32> {
        match &self.detalle {
            DetalleSmart::Nvme(n) => Some(n.porcentaje_usado),
            _ => None,
        }
    }
}