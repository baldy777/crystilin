use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErrorCrystilin {
    // #[from] genera automáticamente la conversión serde_json::Error -> ErrorCrystilin,
    // lo que permite usar el operador `?` sobre resultados de serde_json.
    #[error("no se pudo interpretar la salida de lsblk: {0}")]
    JsonInvalido(#[from] serde_json::Error),
}