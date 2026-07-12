pub mod ciel;
pub mod config;

#[derive(Debug, Clone)]
pub struct PdkRequest {
    pub name: String,
    pub manager: String,
    pub variant: Option<String>,
}
