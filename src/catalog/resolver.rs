use crate::catalog::index::{
    BackendKind, CatalogIndex, EnvironmentDef, PackageRequest, PdkRequest, ResolvedItem, ToolRegistry,
};
use std::path::Path;

pub struct Resolver {
    index: CatalogIndex,
    tools: ToolRegistry,
    base_dir: String,
}

impl Resolver {
    pub fn new(index: CatalogIndex, tools: ToolRegistry, base_dir: String) -> Self {
        Self {
            index,
            tools,
            base_dir,
        }
    }

    pub fn load(base_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let index_path = base_dir.join("index.yaml");
        let tools_path = base_dir.join("tools.yaml");

        let index: CatalogIndex = serde_yaml::from_str(&std::fs::read_to_string(&index_path)?)?;
        let tools: ToolRegistry = serde_yaml::from_str(&std::fs::read_to_string(&tools_path)?)?;

        Ok(Self::new(index, tools, base_dir.to_string_lossy().to_string()))
    }

    pub fn resolve(&self, name: &str) -> Result<Vec<ResolvedItem>, Box<dyn std::error::Error>> {
        if let Some(env_path) = self.index.environments.get(name) {
            return Ok(self
                .resolve_env(&Path::new(&self.base_dir).join(env_path))?
                .into_iter()
                .map(ResolvedItem::Tool)
                .collect());
        }

        if let Some(tool) = self.tools.get(name) {
            return Ok(vec![ResolvedItem::Tool(self.tool_to_request(name, tool))]);
        }

        if let Some(pdks) = &self.index.pdks {
            if let Some(pdk) = pdks.get(name) {
                return Ok(vec![ResolvedItem::Pdk(PdkRequest {
                    name: name.to_string(),
                    manager: pdk.manager.clone(),
                    variant: pdk.variant.clone(),
                })]);
            }
        }

        Err(format!("'{}' not found in catalog", name).into())
    }

    fn resolve_env(
        &self,
        env_path: &Path,
    ) -> Result<Vec<PackageRequest>, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(env_path)?;
        let env: EnvironmentDef = serde_yaml::from_str(&content)?;

        let mut requests = Vec::new();
        for tool_name in &env.tools {
            if let Some(tool) = self.tools.get(tool_name) {
                requests.push(self.tool_to_request(tool_name, tool));
            } else {
                return Err(format!(
                    "Tool '{}' referenced by env '{}' not found in tools.yaml",
                    tool_name, env.name
                )
                .into());
            }
        }

        Ok(requests)
    }

    fn tool_to_request(&self, name: &str, tool: &crate::catalog::index::ToolEntry) -> PackageRequest {
        PackageRequest {
            name: name.to_string(),
            backend: BackendKind::from_str(&tool.backend),
            channel: tool.channel.clone(),
            package: tool.package.clone(),
        }
    }

    pub fn which_envs(&self, tool: &str) -> Vec<String> {
        let mut envs = Vec::new();
        for (env_name, env_path) in &self.index.environments {
            let full_path = Path::new(&self.base_dir).join(env_path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                if let Ok(env) = serde_yaml::from_str::<EnvironmentDef>(&content) {
                    if env.tools.iter().any(|t| t == tool) {
                        envs.push(env_name.clone());
                    }
                }
            }
        }
        envs
    }

    pub fn list_environments(&self) -> Vec<String> {
        self.index.environments.keys().cloned().collect()
    }

    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn list_pdks(&self) -> Vec<String> {
        self.index
            .pdks
            .as_ref()
            .map(|p| p.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn search(&self, query: &str) -> Vec<CatalogEntry> {
        let query = query.to_lowercase();
        let mut results = Vec::new();

        for name in self.index.environments.keys() {
            if name.to_lowercase().contains(&query) {
                results.push(CatalogEntry {
                    name: name.clone(),
                    kind: "env".to_string(),
                    description: String::new(),
                });
            }
        }

        for name in self.tools.keys() {
            if name.to_lowercase().contains(&query) {
                results.push(CatalogEntry {
                    name: name.clone(),
                    kind: "tool".to_string(),
                    description: String::new(),
                });
            }
        }

        if let Some(pdks) = &self.index.pdks {
            for name in pdks.keys() {
                if name.to_lowercase().contains(&query) {
                    results.push(CatalogEntry {
                        name: name.clone(),
                        kind: "pdk".to_string(),
                        description: String::new(),
                    });
                }
            }
        }

        results
    }
}

#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub name: String,
    pub kind: String,
    pub description: String,
}
