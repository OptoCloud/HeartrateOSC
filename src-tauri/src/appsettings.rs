use std::hash::Hash;

#[derive(Debug, Clone, Hash, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub osc_adress: String,
    pub osc_port: u16
}