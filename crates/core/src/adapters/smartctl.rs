use serde::Deserialize;

use crate::errors::ErrorCrystilin;
use crate::models::{AtributoAta, DetalleSmart, InformeSmart, SaludNvme};

// ---------- DTOs: reflejan el JSON de smartctl. Privados al módulo. ----------
// Serde ignora los campos del JSON que no declaramos: solo pedimos lo que usamos.

#[derive(Debug, Deserialize)]
struct SmartctlJson {
    smartctl: Cabecera,
    model_name: Option<String>,
    serial_number: Option<String>,
    smart_status: Option<EstadoJson>,
    temperature: Option<TemperaturaJson>,
    power_on_time: Option<TiempoJson>,
    ata_smart_attributes: Option<AtributosAtaJson>,
    nvme_smart_health_information_log: Option<NvmeJson>,
}

#[derive(Debug, Deserialize)]
struct Cabecera {
    exit_status: u32,
    #[serde(default)] // si el campo no viene, usa Vec vacío en vez de fallar
    messages: Vec<MensajeJson>,
}

#[derive(Debug, Deserialize)]
struct MensajeJson {
    string: String,
}

#[derive(Debug, Deserialize)]
struct EstadoJson {
    passed: bool,
}

#[derive(Debug, Deserialize)]
struct TemperaturaJson {
    current: i32,
}

#[derive(Debug, Deserialize)]
struct TiempoJson {
    hours: u64,
}

#[derive(Debug, Deserialize)]
struct AtributosAtaJson {
    table: Vec<AtributoJson>,
}

#[derive(Debug, Deserialize)]
struct AtributoJson {
    id: u32,
    name: String,
    value: u32,
    worst: u32,
    thresh: u32,
    raw: RawJson,
}

#[derive(Debug, Deserialize)]
struct RawJson {
    value: u64, // u64: el Spin_Up_Time de tu Hitachi ya pasa de 100 mil millones
    string: String,
}

#[derive(Debug, Deserialize)]
struct NvmeJson {
    critical_warning: u32,
    available_spare: u32,
    available_spare_threshold: u32,
    percentage_used: u32,
    data_units_read: u64,
    data_units_written: u64,
    media_errors: u64,
    unsafe_shutdowns: u64,
    power_cycles: u64,
}

// ---------- API pública del módulo ----------

/// Convierte el texto JSON de `smartctl --json` en un `InformeSmart`.
pub fn parsear(texto: &str) -> Result<InformeSmart, ErrorCrystilin> {
    let json: SmartctlJson = serde_json::from_str(texto)?;

    if (json.smartctl.exit_status & 0b11) != 0 {
        // .iter() PRESTA los mensajes; no los consume.
        let mensajes = json
            .smartctl
            .messages
            .iter()
            .map(|m| m.string.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
        return Err(ErrorCrystilin::SmartctlFallo {
            codigo: Some(json.smartctl.exit_status as i32),
            mensajes,
        });
    }

    Ok(a_informe(json))
}

// ---------- Conversión DTO -> dominio ----------

// Recibe `json` por VALOR (ownership): nos lo cedieron y no lo usaremos más.
// Así movemos sus `String` al resultado sin `.clone()`.
fn a_informe(json: SmartctlJson) -> InformeSmart {
    let detalle = if let Some(n) = json.nvme_smart_health_information_log {
        DetalleSmart::Nvme(SaludNvme {
            advertencia_critica: n.critical_warning,
            repuesto_disponible: n.available_spare,
            umbral_repuesto: n.available_spare_threshold,
            porcentaje_usado: n.percentage_used,
            unidades_escritas: n.data_units_written,
            unidades_leidas: n.data_units_read,
            errores_de_medio: n.media_errors,
            apagados_inseguros: n.unsafe_shutdowns,
            ciclos_de_encendido: n.power_cycles,
        })
    } else if let Some(ata) = json.ata_smart_attributes {
        DetalleSmart::Ata {
            atributos: ata
                .table
                .into_iter() // into_iter consume el Vec: movemos cada elemento
                .map(|a| AtributoAta {
                    id: a.id,
                    nombre: a.name,
                    valor: a.value,
                    peor: a.worst,
                    umbral: a.thresh,
                    raw_valor: a.raw.value,
                    raw_texto: a.raw.string,
                })
                .collect(),
        }
    } else {
        DetalleSmart::SinDatos
    };

    InformeSmart {
        modelo: json.model_name,
        serie: json.serial_number,
        aprobado: json.smart_status.map(|s| s.passed),
        horas_encendido: json.power_on_time.map(|t| t.hours),
        temperatura_c: json.temperature.map(|t| t.current),
        estado_smartctl: json.smartctl.exit_status,
        detalle,
    }
}

use std::process::Command;

/// Cómo hablarle al disco. Un USB suele necesitar `-d sat` para dar datos SMART.
/// Usamos un enum cerrado en vez de un String libre: así nadie puede
/// colar argumentos arbitrarios en el comando.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModoSmartctl {
    Auto,
    Sat,
}

impl ModoSmartctl {
    // `&'static str` = texto que vive en el binario durante todo el programa
    // (los literales como "sat"). Por eso podemos devolverlo sin dudas de ownership.
    fn argumento(self) -> Option<&'static str> {
        match self {
            ModoSmartctl::Auto => None,
            ModoSmartctl::Sat => Some("sat"),
        }
    }
}

/// El contrato: "algo que sabe leer el SMART de un dispositivo".
/// Es el equivalente a una `interface` de Java.
pub trait LectorSmart {
    fn leer(&self, dispositivo: &str, modo: ModoSmartctl) -> Result<InformeSmart, ErrorCrystilin>;
}

/// Implementación real: ejecuta `smartctl`.
pub struct SmartctlReal {
    /// `smartctl` necesita permisos de root para abrir el disco.
    /// TEMPORAL: en la Fase 4 esto se sustituye por `pkexec` en el adaptador `privilege`.
    pub con_sudo: bool,
}

impl LectorSmart for SmartctlReal {
    fn leer(&self, dispositivo: &str, modo: ModoSmartctl) -> Result<InformeSmart, ErrorCrystilin> {
        validar_dispositivo(dispositivo)?;

        let mut cmd = if self.con_sudo {
            let mut c = Command::new("sudo");
            c.arg("smartctl");
            c
        } else {
            Command::new("smartctl")
        };
        if let Some(d) = modo.argumento() {
            cmd.args(["-d", d]);
        }
        // Solo lectura: `-a` imprime todo, no escribe nada en el disco.
        cmd.args(["-a", "--json", dispositivo]);

        let salida = cmd.output().map_err(ErrorCrystilin::NoSePudoEjecutar)?;

        // Ojo: NO miramos `salida.status`. smartctl devuelve códigos != 0 aun
        // funcionando bien (bits de estado). Decide `parsear` con el exit_status del JSON.
        let stdout = String::from_utf8_lossy(&salida.stdout);
        if stdout.trim().is_empty() {
            // Sin JSON: típico de sudo con contraseña errónea o smartctl no instalado.
            return Err(ErrorCrystilin::SmartctlFallo {
                codigo: salida.status.code(),
                mensajes: String::from_utf8_lossy(&salida.stderr).trim().to_string(),
            });
        }
        parsear(&stdout)
    }
}

/// Defensa básica: solo rutas `/dev/...`. Evita que un texto como "--help"
/// se interprete como opción de smartctl. `Command` ya no usa shell, pero esto cierra el hueco.
fn validar_dispositivo(d: &str) -> Result<(), ErrorCrystilin> {
    let valido = d.starts_with("/dev/")
        && d.len() > "/dev/".len()
        && !d.contains("..")
        && !d.chars().any(|c| c.is_whitespace() || c.is_control());
    if valido {
        Ok(())
    } else {
        Err(ErrorCrystilin::DispositivoInvalido(d.to_string()))
    }
}

#[cfg(test)] // este módulo solo se compila al correr `cargo test`
mod tests {
    use super::*; // importa todo lo del módulo padre, incluidos los items privados

    // include_str! lee el archivo EN COMPILACIÓN y lo mete en el binario como &str.
    // La ruta es relativa a ESTE archivo: adapters -> src -> core -> crates -> raíz.
    const HDD_USB: &str = include_str!("../../../../tests/fixtures/smartctl_hdd_usb_sat.json");
    const NVME: &str = include_str!("../../../../tests/fixtures/smartctl_nvme_samsung.json");

    #[test]
    fn hdd_usb_datos_basicos() {
        let informe = parsear(HDD_USB).unwrap(); // en tests, unwrap() es aceptable: si falla, el test falla
        assert_eq!(informe.modelo.as_deref(), Some("Hitachi HTS725032A9A364"));
        assert_eq!(informe.aprobado, Some(true));
        assert_eq!(informe.horas_encendido, Some(4243));
        assert_eq!(informe.temperatura_c, Some(13));
        assert_eq!(informe.estado_smartctl, 96);
    }

    #[test]
    fn hdd_usb_sectores_reasignados_usa_solo_16_bits() {
        let informe = parsear(HDD_USB).unwrap();
        // raw.value = 65544 (0x00010008) pero el contador real es 8
        assert_eq!(informe.sectores_reasignados(), Some(8));
    }

    #[test]
    fn hdd_usb_es_ata_y_no_tiene_desgaste_porcentual() {
        let informe = parsear(HDD_USB).unwrap();
        assert!(matches!(informe.detalle, DetalleSmart::Ata { .. }));
        assert_eq!(informe.desgaste_porcentaje(), None);
    }

    #[test]
    fn nvme_datos_basicos_y_desgaste() {
        let informe = parsear(NVME).unwrap();
        assert_eq!(informe.horas_encendido, Some(809));
        assert_eq!(informe.temperatura_c, Some(16));
        assert_eq!(informe.estado_smartctl, 0);
        assert_eq!(informe.desgaste_porcentaje(), Some(3));
        assert_eq!(informe.sectores_reasignados(), None); // concepto de ATA, no de NVMe
    }

    #[test]
    fn nvme_errores_de_medio_en_cero() {
        let informe = parsear(NVME).unwrap();
        match informe.detalle {
            DetalleSmart::Nvme(n) => assert_eq!(n.errores_de_medio, 0),
            otro => panic!("se esperaba NVMe, llegó {otro:?}"),
        }
    }

    #[test]
    fn json_minimo_sin_smart_da_sin_datos() {
        // Un USB sin soporte SMART devuelve poco más que la cabecera.
        let informe = parsear(r#"{"smartctl":{"exit_status":4}}"#).unwrap();
        assert_eq!(informe.detalle, DetalleSmart::SinDatos);
        assert_eq!(informe.aprobado, None);
        assert_eq!(informe.estado_smartctl, 4);
    }

    #[test]
    fn json_roto_devuelve_error() {
        assert!(parsear("esto no es json").is_err());
    }

        // Un "mock" a mano: no ejecuta nada, devuelve un fixture.
    struct LectorFalso {
        texto: &'static str,
    }

    impl LectorSmart for LectorFalso {
        fn leer(&self, _dispositivo: &str, _modo: ModoSmartctl) -> Result<InformeSmart, ErrorCrystilin> {
            parsear(self.texto)
        }
    }

    // Función genérica: acepta CUALQUIER tipo que implemente LectorSmart.
    // `L: LectorSmart` se lee "L es cualquier tipo que cumpla el trait".
    fn horas_de<L: LectorSmart>(lector: &L) -> Option<u64> {
        lector
            .leer("/dev/sda", ModoSmartctl::Sat)
            .unwrap()
            .horas_encendido
    }

    #[test]
    fn el_trait_permite_sustituir_el_lector() {
        let falso = LectorFalso { texto: HDD_USB };
        assert_eq!(horas_de(&falso), Some(4243));
    }

    #[test]
    fn rechaza_dispositivos_que_no_son_dev() {
        let real = SmartctlReal { con_sudo: false };
        // Estos errores saltan ANTES de ejecutar nada.
        for malo in ["--help", "sda", "/dev/", "/dev/../etc/passwd", "/dev/sda; ls"] {
            let r = real.leer(malo, ModoSmartctl::Auto);
            assert!(matches!(r, Err(ErrorCrystilin::DispositivoInvalido(_))), "debía rechazar {malo}");
        }
    }

    #[test]
    fn exit_status_con_bit_de_dispositivo_no_abierto_es_error() {
        let json = r#"{"smartctl":{"exit_status":2,"messages":[{"string":"Permission denied"}]}}"#;
        match parsear(json) {
            Err(ErrorCrystilin::SmartctlFallo { mensajes, .. }) => assert!(mensajes.contains("Permission")),
            otro => panic!("se esperaba SmartctlFallo, llegó {otro:?}"),
        }
    }
}