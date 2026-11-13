use crystilin_core::adapters::lsblk::listar_discos;

use std::fmt::Display;

use crystilin_core::adapters::smartctl::{LectorSmart, ModoSmartctl, SmartctlReal};
use crystilin_core::models::{Conexion, DetalleSmart, InformeSmart};

fn main() -> anyhow::Result<()> {
    let discos = listar_discos()?;

        println!(
        "{:<16} {:<28} {:>10}  {:<6} {:<5} {}",
        "DISPOSITIVO", "MODELO", "TAMAÑO", "CONEX.", "MEDIO", "ESTADO"
    );

    for d in &discos {
        let modelo = d.modelo.as_deref().unwrap_or("(desconocido)");
        let estado = if d.protegido { "PROTEGIDO (sistema)" } else { "" };

                println!(
            "{:<16} {:<28} {:>10}  {:<6} {:<5} {}",
            d.ruta,
            modelo,
            formatear_tamano(d.tamano_bytes),
            d.conexion,
            d.medio,
            estado
        );
    }

        println!("\n=== Lectura SMART (solo lectura) ===");
    println!("Se pedirá tu contraseña de sudo para abrir los discos.\n");

    let lector = SmartctlReal { con_sudo: true }; // TEMPORAL: en la Fase 4 pasa a pkexec

    for disco in &discos {
        println!("{} ({})", disco.ruta, disco.modelo.as_deref().unwrap_or("modelo desconocido"));

        // Un disco en caja USB suele necesitar `-d sat` para dar SMART.
        let modo = if disco.conexion == Conexion::Usb {
            ModoSmartctl::Sat
        } else {
            ModoSmartctl::Auto
        };

        // Un fallo en un disco NO debe tumbar el programa entero: lo mostramos y seguimos.
        match lector.leer(&disco.ruta, modo) {
            Ok(informe) => mostrar_informe(&informe),
            Err(e) => println!("  No se pudo leer el SMART: {e}"),
        }
        println!();
    }

    Ok(())
}

/// Formatea un Option: el valor con su unidad, o "—" si no hay dato.
/// `T: Display` significa "cualquier tipo que se sepa imprimir".
fn o<T: Display>(valor: Option<T>, unidad: &str) -> String {
    match valor {
        Some(v) => format!("{v}{unidad}"),
        None => "—".to_string(),
    }
}

fn mostrar_informe(i: &InformeSmart) {
    let veredicto_disco = match i.aprobado {
        Some(true) => "PASSED (el propio disco dice que está bien)",
        Some(false) => "FAILED (el propio disco reporta fallo)",
        None => "sin dato",
    };
    println!("  Autodiagnóstico SMART : {veredicto_disco}");
    println!("  Horas encendido       : {}", o(i.horas_encendido, " h"));
    println!("  Temperatura           : {}", o(i.temperatura_c, " °C"));

    match &i.detalle {
        DetalleSmart::Ata { .. } => {
            println!("  Sectores reasignados  : {}", o(i.sectores_reasignados(), ""));
        }
        DetalleSmart::Nvme(n) => {
            println!("  Desgaste              : {}", o(i.desgaste_porcentaje(), " %"));
            println!("  Repuesto disponible   : {} %", n.repuesto_disponible);
            println!("  Errores de medio      : {}", n.errores_de_medio);
            println!("  Apagados inseguros    : {}", n.apagados_inseguros);
        }
        DetalleSmart::SinDatos => {
            println!("  Este dispositivo no expone datos SMART.");
        }
    }
}

/// Los fabricantes usan unidades decimales (1 GB = 1.000.000.000 bytes),
/// así que usamos las mismas para que "960 GB" coincida con la caja del disco.
fn formatear_tamano(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1e12 {
        format!("{:.2} TB", b / 1e12)
    } else {
        format!("{:.1} GB", b / 1e9)
    }
}