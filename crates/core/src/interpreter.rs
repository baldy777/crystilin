use crate::models::{DetalleSmart, Hallazgo, InformeSmart, Semaforo, Veredicto};
use crate::reglas::{Metrica, Regla, Reglas, Sentido};

/// Pieza 1: sacar del informe el número que pide la regla.
/// Devuelve `None` si ese disco no tiene esa métrica
/// (un HDD no tiene desgaste %, un USB sin SMART no tiene nada).
fn valor_de(informe: &InformeSmart, metrica: Metrica) -> Option<f64> {
    // `match` sobre un enum SIN `_`: el compilador te obliga a cubrir TODAS las
    // variantes. Si mañana añades `Metrica::Foo` al enum, este archivo deja de
    // compilar hasta que decidas qué hacer con ella. En Java, un `switch` sobre
    // un enum no te obliga a eso.
    match metrica {
        // `as f64` convierte u64 -> f64. Para u64 no existe `From`, porque un
        // u64 enorme perdería precisión; aquí nuestros valores son pequeños.
        Metrica::SectoresReasignados => informe.sectores_reasignados().map(|v| v as f64),
        Metrica::HorasEncendido => informe.horas_encendido.map(|v| v as f64),
        // u32 e i32 caben siempre en f64, por eso aquí sí hay `From`.
        // `f64::from` es lo mismo que `|v| f64::from(v)`.
        Metrica::DesgastePorcentaje => informe.desgaste_porcentaje().map(f64::from),
        Metrica::TemperaturaC => informe.temperatura_c.map(f64::from),
        // Estas dos viven DENTRO de SaludNvme: abrimos el enum prestándolo (&).
        Metrica::RepuestoDisponible => match &informe.detalle {
            DetalleSmart::Nvme(n) => Some(f64::from(n.repuesto_disponible)),
            _ => None,
        },
        Metrica::ErroresDeMedio => match &informe.detalle {
            DetalleSmart::Nvme(n) => Some(n.errores_de_medio as f64),
            _ => None,
        },
    }
}

/// Pieza 2: comparar el valor con los umbrales de UNA regla.
fn evaluar(regla: &Regla, valor: f64) -> Hallazgo {
    // Una closure (como una lambda de Java): "¿el valor ya cruzó este umbral?".
    // Captura `regla` y `valor` por referencia, no hace falta pasarlos.
    let alcanzado = |umbral: f64| match regla.sentido {
        Sentido::MayorEsPeor => valor >= umbral,
        Sentido::MenorEsPeor => valor <= umbral,
    };

    // Hacemos `match` sobre una TUPLA y usamos "guards" (`if ...`).
    // Se evalúa de arriba abajo y gana la primera rama que encaje:
    //  1) hay umbral rojo Y texto rojo Y el valor lo alcanzó -> rojo
    //  2) si no, ¿alcanzó el amarillo?                       -> amarillo
    //  3) si no                                              -> verde
    let (semaforo, plantilla) = match (regla.umbral_rojo, &regla.texto_rojo) {
        (Some(umbral), Some(texto)) if alcanzado(umbral) => (Semaforo::Rojo, texto),
        _ if alcanzado(regla.umbral_amarillo) => (Semaforo::Amarillo, &regla.texto_amarillo),
        _ => (Semaforo::Verde, &regla.texto_verde),
    };

    Hallazgo {
        semaforo,
        titulo: regla.titulo.clone(),
        // Cambia "{valor}" por el número real. f64 se imprime sin decimales
        // cuando es entero: 8.0 -> "8".
        explicacion: plantilla.replace("{valor}", &valor.to_string()),
    }
}

/// Pieza 3: la función pública. Recibe PRÉSTAMOS (&), no se queda con nada.
pub fn interpretar(informe: &InformeSmart, reglas: &Reglas) -> Veredicto {
    let mut hallazgos = Vec::new();

    // Autoevaluación del propio disco (PASSED / FAILED).
    // `None` = el disco no la reportó: no inventamos hallazgo.
    if let Some(aprobado) = informe.aprobado {
        let a = &reglas.autoevaluacion;
        // `.clone()` porque `reglas` es prestado: no podemos MOVER sus Strings,
        // solo copiarlas hacia el Hallazgo nuevo, que es dueño de las suyas.
        hallazgos.push(if aprobado {
            Hallazgo {
                semaforo: Semaforo::Verde,
                titulo: a.titulo.clone(),
                explicacion: a.texto_ok.clone(),
            }
        } else {
            Hallazgo {
                semaforo: Semaforo::Rojo,
                titulo: a.titulo.clone(),
                explicacion: a.texto_fallo.clone(),
            }
        });
    }

    for regla in &reglas.reglas {
        // Si el disco no tiene esa métrica, la regla simplemente se salta.
        if let Some(valor) = valor_de(informe, regla.metrica) {
            hallazgos.push(evaluar(regla, valor));
        }
    }

    Veredicto::desde_hallazgos(hallazgos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::smartctl::parsear;

    // Rutas relativas a ESTE archivo (crates/core/src/): tres ".." llegan a la raíz.
    const HDD_USB: &str = include_str!("../../../tests/fixtures/smartctl_hdd_usb_sat.json");
    const NVME: &str = include_str!("../../../tests/fixtures/smartctl_nvme_samsung.json");

    fn regla(m: Metrica) -> Regla {
        Reglas::por_defecto()
            .unwrap()
            .reglas
            .into_iter()
            .find(|r| r.metrica == m)
            .unwrap()
    }

    fn buscar<'a>(v: &'a Veredicto, titulo: &str) -> &'a Hallazgo {
        v.hallazgos.iter().find(|h| h.titulo == titulo).unwrap()
    }

    #[test]
    fn mayor_es_peor_cruza_umbrales() {
        let r = regla(Metrica::SectoresReasignados); // amarillo >= 1, rojo >= 50
        assert_eq!(evaluar(&r, 0.0).semaforo, Semaforo::Verde);
        assert_eq!(evaluar(&r, 8.0).semaforo, Semaforo::Amarillo);
        assert_eq!(evaluar(&r, 50.0).semaforo, Semaforo::Rojo);
    }

    #[test]
    fn menor_es_peor_cruza_umbrales() {
        let r = regla(Metrica::RepuestoDisponible); // amarillo <= 20, rojo <= 10
        assert_eq!(evaluar(&r, 100.0).semaforo, Semaforo::Verde);
        assert_eq!(evaluar(&r, 15.0).semaforo, Semaforo::Amarillo);
        assert_eq!(evaluar(&r, 5.0).semaforo, Semaforo::Rojo);
    }

    #[test]
    fn una_regla_sin_rojo_nunca_da_rojo() {
        let r = regla(Metrica::HorasEncendido);
        assert_eq!(evaluar(&r, 999_999.0).semaforo, Semaforo::Amarillo);
    }

    #[test]
    fn la_explicacion_reemplaza_el_valor() {
        let r = regla(Metrica::SectoresReasignados);
        assert!(evaluar(&r, 8.0).explicacion.contains("8 sectores"));
    }

    #[test]
    fn hitachi_da_amarillo_por_sectores_y_nunca_rojo() {
        let informe = parsear(HDD_USB).unwrap();
        let v = interpretar(&informe, &Reglas::por_defecto().unwrap());
        assert_eq!(v.semaforo, Semaforo::Amarillo);
        assert_eq!(buscar(&v, "Sectores reasignados").semaforo, Semaforo::Amarillo);
        // El atributo 190 con worst < thresh NO debe volverse un rojo.
        assert!(v.hallazgos.iter().all(|h| h.semaforo != Semaforo::Rojo));
    }

    #[test]
    fn nvme_sano_da_verde() {
        let informe = parsear(NVME).unwrap();
        let v = interpretar(&informe, &Reglas::por_defecto().unwrap());
        assert_eq!(v.semaforo, Semaforo::Verde);
        // Los 1232 apagados inseguros son informativos: no hay regla para ellos.
        assert!(v.hallazgos.iter().all(|h| !h.titulo.contains("apag")));
    }

    #[test]
    fn nvme_danado_a_mano_da_rojo() {
        let mut informe = parsear(NVME).unwrap();
        // `&mut` = préstamo que PERMITE modificar. Solo puede existir uno a la vez.
        if let DetalleSmart::Nvme(n) = &mut informe.detalle {
            n.repuesto_disponible = 5;
            n.errores_de_medio = 20;
        }
        let v = interpretar(&informe, &Reglas::por_defecto().unwrap());
        assert_eq!(v.semaforo, Semaforo::Rojo);
        assert_eq!(buscar(&v, "Reserva de repuesto").semaforo, Semaforo::Rojo);
        assert_eq!(buscar(&v, "Errores de lectura o escritura").semaforo, Semaforo::Rojo);
    }

    #[test]
    fn si_el_disco_reporta_fallo_es_rojo() {
        let mut informe = parsear(NVME).unwrap();
        informe.aprobado = Some(false);
        let v = interpretar(&informe, &Reglas::por_defecto().unwrap());
        assert_eq!(v.semaforo, Semaforo::Rojo);
    }
}