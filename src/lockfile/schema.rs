use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Lockfile {
    pub version: u32,
    pub generated: String,
    #[serde(default)]
    pub package: Vec<LockedPackage>,
    #[serde(default)]
    pub pdk: HashMap<String, LockedPdk>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub channel: Option<String>,
    pub backend: String,
    #[serde(default)]
    pub sha256: String,
    /// Exact conda package URLs with build hashes (from catalog/locks/).
    /// Empty = use spec-based install with hermetic flags.
    #[serde(default)]
    pub explicit_urls: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockedPdk {
    pub variant: String,
    pub manager: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub sha256: String,
}

impl Lockfile {
    pub fn new() -> Self {
        let dur = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = dur.as_secs();
        Self {
            version: 1,
            generated: format_timestamp(secs),
            package: Vec::new(),
            pdk: HashMap::new(),
        }
    }

    pub fn find_package(&self, name: &str) -> Option<&LockedPackage> {
        self.package.iter().find(|p| p.name == name)
    }
}

fn format_timestamp(unix_secs: u64) -> String {
    let days_since_epoch = (unix_secs / 86400) as i64;
    let secs_of_day = (unix_secs % 86400) as u32;

    let (year, month, day) = civil_from_days(days_since_epoch + 719_163);

    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + (m <= 2) as i64, m, d)
}
