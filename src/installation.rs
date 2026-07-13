use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationMeta {
    pub install_method: String,
    pub installed_at: String,
    pub binary_version: String,
    pub catalog_version: String,
    pub installer_version: u32,
}

impl InstallationMeta {
    pub fn new(install_method: &str, binary_version: &str, catalog_version: &str) -> Self {
        Self {
            install_method: install_method.to_string(),
            installed_at: chrono_now(),
            binary_version: binary_version.to_string(),
            catalog_version: catalog_version.to_string(),
            installer_version: 3,
        }
    }
}

/// Read installation metadata from disk.
pub fn read_installation(path: &Path) -> Option<InstallationMeta> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_yaml::from_str(&content).ok()
}

/// Write installation metadata to disk.
pub fn write_installation(path: &Path, meta: &InstallationMeta) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let yaml = serde_yaml::to_string(meta)?;
    std::fs::write(path, yaml)?;
    Ok(())
}

fn chrono_now() -> String {
    // ISO 8601 without pulling in the chrono crate
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Approximate: convert unix timestamp to YYYY-MM-DDTHH:MM:SSZ
    let days = secs / 86400;
    let time = secs % 86400;
    let hours = time / 3600;
    let minutes = (time % 3600) / 60;
    let seconds = time % 60;

    // gregorian calendar approximation (good enough for metadata)
    let (y, m, d) = civil_from_days(days as i64 + 719468); // 719468 = days from 0000-01-01 to 1970-01-01

    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

/// Convert days since 0000-03-01 to year/month/day.
/// Algorithm from Howard Hinnant's date library (public domain).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z - 719468;
    let era = (if z >= 0 { z } else { z - 146096 } ) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
