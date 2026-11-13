use serde::Deserialize;

use crate::errors::ErrorCrystilin;

// `include_str!` mete el contenido del archivo DENTRO del binario al compilar.
// La ruta es relativa a ESTE archivo (src/reglas.rs), por eso "../data/...".
// Ventaja: el programa no depende de que el archivo exista en la máquina del
// usuario. Es un `const &'static str`: texto que vive todo el programa.
const REGLAS_POR_DEFECTO: &str = include_str!("../data/smart_rules.toml");

/// Qué número del disco miramos. Es un enum "sin datos" (como un enum de Java).
/// `rename_all = "snake_case"` hace que la variante `SectoresReasignados`
/// se lea del TOML como "sectores_reasignados".
/// Si el TOML trae una métrica que no existe aquí, serde da ERROR al cargar,
/// no al usarla. Es como un enum de Jackson, pero más estricto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metrica {
    SectoresReasignados,
    DesgastePorcentaje,
    RepuestoDisponible,
    ErroresDeMedio,
    TemperaturaC,
    HorasEncendido,
}

/// Hacia dónde empeora el número.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sentido {
    MayorEsPeor,
    MenorEsPeor,
}

/// Una regla individual (una tabla `[[regla]]` del TOML).
#[derive(Debug, Clone, Deserialize)]
pub struct Regla {
    pub metrica: Metrica,
    pub sentido: Sentido,
    pub umbral_amarillo: f64,
    // Option: si el campo NO aparece en el TOML, serde lo deja en None
    // sin dar error. Así una regla puede no tener rojo.
    pub umbral_rojo: Option<f64>,
    pub titulo: String,
    pub texto_verde: String,
    pub texto_amarillo: String,
    pub texto_rojo: Option<String>,
}

/// Textos para el veredicto de "PASSED/FAILED" que da el propio disco.
#[derive(Debug, Clone, Deserialize)]
pub struct Autoevaluacion {
    pub titulo: String,
    pub texto_ok: String,
    pub texto_fallo: String,
}

/// Todo el archivo de reglas.
#[derive(Debug, Clone, Deserialize)]
pub struct Reglas {
    pub autoevaluacion: Autoevaluacion,
    // En el TOML cada bloque se llama `[[regla]]` (singular, lee mejor ahí),
    // pero en Rust el campo es una lista, así que lo llamamos `reglas`.
    // `rename` conecta los dos nombres.
    #[serde(rename = "regla")]
    pub reglas: Vec<Regla>,
}

impl Reglas {
    /// Lee reglas desde un texto TOML cualquiera (útil para tests y, más
    /// adelante, para que el usuario cargue su propio archivo).
    pub fn desde_texto(texto: &str) -> Result<Self, ErrorCrystilin> {
        // `from_str` devuelve Result<Reglas, toml::de::Error>.
        // `map_err` convierte ESE error en NUESTRO error de dominio,
        // igual que envolverías una excepción de librería en una propia en Java.
        toml::from_str(texto).map_err(|e| ErrorCrystilin::ReglasInvalidas(e.to_string()))
    }

    /// Las reglas que vienen incluidas dentro del programa.
    pub fn por_defecto() -> Result<Self, ErrorCrystilin> {
        Self::desde_texto(REGLAS_POR_DEFECTO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_reglas_por_defecto_se_cargan() {
        let reglas = Reglas::por_defecto().expect("el TOML incluido debe ser válido");
        assert_eq!(reglas.reglas.len(), 6);
    }

    #[test]
    fn sectores_reasignados_alarma_desde_uno() {
        let reglas = Reglas::por_defecto().unwrap();
        let r = reglas
            .reglas
            .iter()
            .find(|r| r.metrica == Metrica::SectoresReasignados)
            .expect("debe existir la regla de sectores reasignados");
        assert_eq!(r.sentido, Sentido::MayorEsPeor);
        assert_eq!(r.umbral_amarillo, 1.0);
        assert_eq!(r.umbral_rojo, Some(50.0));
    }

    #[test]
    fn las_horas_de_uso_no_tienen_rojo() {
        let reglas = Reglas::por_defecto().unwrap();
        let r = reglas
            .reglas
            .iter()
            .find(|r| r.metrica == Metrica::HorasEncendido)
            .unwrap();
        assert_eq!(r.umbral_rojo, None);
        assert_eq!(r.texto_rojo, None);
    }

    #[test]
    fn una_metrica_desconocida_da_error() {
        let texto = r#"
            [autoevaluacion]
            titulo = "x"
            texto_ok = "x"
            texto_fallo = "x"

            [[regla]]
            metrica = "inventada"
            sentido = "mayor_es_peor"
            umbral_amarillo = 1
            titulo = "x"
            texto_verde = "x"
            texto_amarillo = "x"
        "#;
        let resultado = Reglas::desde_texto(texto);
        assert!(matches!(resultado, Err(ErrorCrystilin::ReglasInvalidas(_))));
    }

    #[test]
    fn un_toml_roto_da_error() {
        let resultado = Reglas::desde_texto("esto no es toml [[[");
        assert!(resultado.is_err());
    }
}
