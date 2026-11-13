use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErrorCrystilin {
    #[error("no se pudo interpretar la salida de lsblk: {0}")]
    JsonInvalido(#[from] serde_json::Error),

    // El sistema operativo no pudo lanzar el programa (no instalado, sin permisos...).
    #[error("no se pudo ejecutar lsblk: {0}")]
    NoSePudoEjecutar(#[source] std::io::Error),

    // El programa arrancó, pero terminó con código de error.
    #[error("lsblk terminó con error (código {codigo:?}): {stderr}")]
    LsblkFallo { codigo: Option<i32>, stderr: String },

    #[error("smartctl falló (código {codigo:?}): {mensajes}")]
    SmartctlFallo { codigo: Option<i32>, mensajes: String },

    #[error("dispositivo no válido: {0}")]
    DispositivoInvalido(String),
}