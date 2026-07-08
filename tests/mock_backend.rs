use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct MockPackage {
    pub name: String,
    pub version: String,
    pub channel: Option<String>,
    pub backend: String,
}

pub struct MockBackend {
    pub installed: Mutex<Vec<MockPackage>>,
    pub should_fail: Mutex<bool>,
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            installed: Mutex::new(Vec::new()),
            should_fail: Mutex::new(false),
        }
    }

    pub fn install(&self, name: &str, version: &str, channel: Option<&str>) -> Result<(), String> {
        if *self.should_fail.lock().unwrap() {
            return Err("mock failure".into());
        }
        self.installed.lock().unwrap().push(MockPackage {
            name: name.to_string(),
            version: version.to_string(),
            channel: channel.map(|s| s.to_string()),
            backend: "mock".to_string(),
        });
        Ok(())
    }

    pub fn list(&self) -> Vec<MockPackage> {
        self.installed.lock().unwrap().clone()
    }

    pub fn verify(&self, name: &str) -> bool {
        self.installed
            .lock()
            .unwrap()
            .iter()
            .any(|p| p.name == name)
    }

    pub fn remove(&self, name: &str) {
        self.installed.lock().unwrap().retain(|p| p.name != name);
    }
}
