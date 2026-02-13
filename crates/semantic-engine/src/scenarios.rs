use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RegistryEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    pub location: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub ontologies: Vec<String>,
    #[serde(default)]
    pub data_files: Vec<String>,
    #[serde(default)]
    pub docs: Vec<String>,
}

pub struct ScenarioManager {
    base_path: PathBuf,
    client: reqwest::Client,
}

impl ScenarioManager {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            client: reqwest::Client::new(),
        }
    }

    /// Fetches the list of available scenarios from the registry.
    pub async fn list_scenarios(&self) -> Result<Vec<RegistryEntry>> {
        // Try local registry first (useful for development)
        let local_registry = Path::new("scenarios/registry.json");
        if local_registry.exists() {
            let content = fs::read_to_string(local_registry).await?;
            let registry: Vec<RegistryEntry> = serde_json::from_str(&content)?;
            return Ok(registry);
        }

        // Fallback to remote registry
        let url =
            "https://raw.githubusercontent.com/pmaojo/synapse-engine/main/scenarios/registry.json";
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch remote registry")?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch registry: {}",
                resp.status()
            ));
        }

        let registry: Vec<RegistryEntry> =
            resp.json().await.context("Failed to parse registry JSON")?;
        Ok(registry)
    }

    /// Installs a scenario by name.
    /// Returns the path to the installed scenario directory.
    pub async fn install_scenario(&self, name: &str) -> Result<PathBuf> {
        let registry = self.list_scenarios().await?;
        let entry = registry
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| anyhow::anyhow!("Scenario '{}' not found in registry", name))?;

        let scenario_dir = self.base_path.join("scenarios").join(name);
        fs::create_dir_all(&scenario_dir).await?;

        // Check if we can install from local source (dev mode)
        // Since we are running from the repo root usually, check if `scenarios/{name}` exists there.
        let local_source = Path::new("scenarios").join(name);
        if local_source.exists() && local_source.join("manifest.json").exists() {
            return self
                .install_from_local_path(&local_source, &scenario_dir)
                .await;
        }

        // If not local, try to download from the URL in registry
        if entry.location.starts_with("http") {
            return self
                .install_from_remote(&entry.location, &scenario_dir)
                .await;
        }

        Err(anyhow::anyhow!(
            "Could not find installation source for scenario '{}'",
            name
        ))
    }

    async fn install_from_local_path(&self, source: &Path, dest: &Path) -> Result<PathBuf> {
        // Prevent self-copy
        if source.canonicalize()? == dest.canonicalize().unwrap_or(dest.to_path_buf()) {
            eprintln!("Source and destination are the same, skipping copy.");
            return Ok(dest.to_path_buf());
        }

        // 1. Copy Manifest
        let manifest_path = source.join("manifest.json");
        fs::copy(&manifest_path, dest.join("manifest.json")).await?;

        let content = fs::read_to_string(&manifest_path).await?;
        let manifest: Manifest = serde_json::from_str(&content)?;

        // 2. Copy Ontologies
        if !manifest.ontologies.is_empty() {
            let schema_dest = dest.join("schema");
            fs::create_dir_all(&schema_dest).await?;
            for file in &manifest.ontologies {
                let src = source.join("schema").join(file);
                if src.exists() {
                    fs::copy(&src, schema_dest.join(file)).await?;
                }
            }
        }

        // 3. Copy Data
        if !manifest.data_files.is_empty() {
            let data_dest = dest.join("data");
            fs::create_dir_all(&data_dest).await?;
            for file in &manifest.data_files {
                let src = source.join("data").join(file);
                if src.exists() {
                    fs::copy(&src, data_dest.join(file)).await?;
                }
            }
        }

        // 4. Copy Docs
        if !manifest.docs.is_empty() {
            let docs_dest = dest.join("docs");
            fs::create_dir_all(&docs_dest).await?;
            for file in &manifest.docs {
                let src = source.join("docs").join(file);
                if src.exists() {
                    fs::copy(&src, docs_dest.join(file)).await?;
                }
            }
        }

        Ok(dest.to_path_buf())
    }

    async fn install_from_remote(&self, base_url: &str, dest: &Path) -> Result<PathBuf> {
        let clean_base = base_url.trim_end_matches('/');

        // 1. Fetch Manifest
        let manifest_url = format!("{}/manifest.json", clean_base);
        let resp = self.client.get(&manifest_url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch manifest from {}",
                manifest_url
            ));
        }
        let content = resp.text().await?;
        fs::write(dest.join("manifest.json"), &content).await?;

        let manifest: Manifest = serde_json::from_str(&content)?;

        // 2. Download Ontologies
        if !manifest.ontologies.is_empty() {
            let schema_dest = dest.join("schema");
            fs::create_dir_all(&schema_dest).await?;
            for file in &manifest.ontologies {
                let url = format!("{}/schema/{}", clean_base, file);
                self.download_file(&url, &schema_dest.join(file)).await?;
            }
        }

        // 3. Download Data
        if !manifest.data_files.is_empty() {
            let data_dest = dest.join("data");
            fs::create_dir_all(&data_dest).await?;
            for file in &manifest.data_files {
                let url = format!("{}/data/{}", clean_base, file);
                self.download_file(&url, &data_dest.join(file)).await?;
            }
        }

        // 4. Download Docs
        if !manifest.docs.is_empty() {
            let docs_dest = dest.join("docs");
            fs::create_dir_all(&docs_dest).await?;
            for file in &manifest.docs {
                let url = format!("{}/docs/{}", clean_base, file);
                self.download_file(&url, &docs_dest.join(file)).await?;
            }
        }

        Ok(dest.to_path_buf())
    }

    async fn download_file(&self, url: &str, dest: &Path) -> Result<()> {
        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download {}: status {}",
                url,
                resp.status()
            ));
        }
        let bytes = resp.bytes().await?;
        fs::write(dest, bytes).await?;
        Ok(())
    }

    pub async fn get_manifest(&self, scenario_name: &str) -> Result<Manifest> {
        let manifest_path = self
            .base_path
            .join("scenarios")
            .join(scenario_name)
            .join("manifest.json");
        if !manifest_path.exists() {
            // Try to find it in the repo root scenarios/ folder if dev
            let local_dev_path = Path::new("scenarios")
                .join(scenario_name)
                .join("manifest.json");
            if local_dev_path.exists() {
                let content = fs::read_to_string(local_dev_path).await?;
                return Ok(serde_json::from_str(&content)?);
            }
            return Err(anyhow::anyhow!(
                "Scenario '{}' is not installed",
                scenario_name
            ));
        }
        let content = fs::read_to_string(manifest_path).await?;
        Ok(serde_json::from_str(&content)?)
    }
}
