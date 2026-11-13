/// Devuelve el saludo del programa.
/// `pub` = visible desde otros crates (sin él, la función sería privada).
/// `&'static str` = referencia a un texto que vive durante todo el programa
/// (los literales como "hola" están incrustados en el binario).
/// Por ahora basta saber que es un String de solo lectura; los lifetimes los veremos a fondo en la Fase 2.

pub mod adapters;
pub mod errors;
pub mod models;
pub mod reglas;
pub mod interpreter;

pub fn saludo() -> &'static str {
    "crystilin: listo para revisar discos"
}