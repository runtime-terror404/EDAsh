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

pub fn lockfile_path() -> PathBuf {
    data_dir().join("edash.lock")
}

pub fn catalog_base_dir() -> PathBuf {
    data_dir().join("catalog").join("base")
}

pub fn catalog_user_dir() -> PathBuf {
    config_dir().join("catalog").join("user")
}

pub fn downloads_dir() -> PathBuf {
    data_dir().join("downloads")
}

pub fn installation_yaml_path() -> PathBuf {
    data_dir().join("installation.yaml")
}
