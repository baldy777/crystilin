use crystilin_core::adapters::lsblk::listar_discos;

fn main() -> anyhow::Result<()> {
    let discos = listar_discos()?;

    println!(
        "{:<16} {:<28} {:>10}  {:<5} {}",
        "DISPOSITIVO", "MODELO", "TAMAÑO", "TIPO", "ESTADO"
    );

    for d in &discos {
        let modelo = d.modelo.as_deref().unwrap_or("(desconocido)");
        let estado = if d.protegido { "PROTEGIDO (sistema)" } else { "" };

        println!(
            "{:<16} {:<28} {:>10}  {:<5} {}",
            d.ruta,
            modelo,
            formatear_tamano(d.tamano_bytes),
            d.tipo,
            estado
        );
    }

    Ok(())
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