use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("edash")
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("edash")
}

pub fn envs_dir() -> PathBuf {
    data_dir().join("envs")
}

pub fn pdks_dir() -> PathBuf {
    data_dir().join("pdks")
}

pub fn cache_dir() -> PathBuf {
    data_dir().join("cache")
}

pub fn bin_dir() -> PathBuf {
    data_dir().join("bin")
}

pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}
