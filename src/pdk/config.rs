use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PdkToolConfig {
    pub name: String,
    pub variant: String,
    pub paths: HashMap<String, String>,
}

/// Load a PDK config from the catalog source.
pub fn load_pdk_config(pdk_name: &str, source: &crate::catalog::CatalogSource) -> Option<PdkToolConfig> {
    let content = source.read_pdk_config(pdk_name)?;
    serde_yaml::from_str(&content).ok()
}

/// Resolve PDK paths to environment variables for installed PDKs.
/// Returns map of ENV_VAR_NAME → full_path_value.
pub fn resolve_pdk_vars(
    installed_pdks: &[String],
    source: &crate::catalog::CatalogSource,
    pdk_root: &Path,
) -> HashMap<String, String> {
    let mut vars: HashMap<String, String> = HashMap::new();

    for pdk_name in installed_pdks {
        let Some(config) = load_pdk_config(pdk_name, source) else {
            continue;
        };

        let prefix = env_prefix(&config.variant);

        for (path_key, rel_path) in &config.paths {
            let full_path = pdk_root.join(&config.variant).join(rel_path);
            if !full_path.exists() {
                continue;
            }

            let var_name = match path_key.as_str() {
                "spice_dir" => format!("{}_SPICE_DIR", prefix),
                "netgen_setup" => format!("{}_NETGEN_SETUP", prefix),
                "magic_rcfile" => format!("{}_MAGIC_RCFILE", prefix),
                "xschem_rcfile" => format!("{}_XSCHEM_RCFILE", prefix),
                "klayout_tech" => format!("{}_KLAYOUT_TECH", prefix),
                _ => continue,
            };

            vars.insert(var_name, full_path.to_string_lossy().to_string());
        }
    }

    vars
}

/// Convert a variant name like "sky130A" or "gf180mcuD" to an env var prefix.
fn env_prefix(variant: &str) -> String {
    variant.to_uppercase().replace('-', "_")
}
