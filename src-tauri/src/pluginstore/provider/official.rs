use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::Client;
use std::io::Write;
use std::{any::Any, sync::Arc};
use tauri::{ipc::Channel, Manager};
use tokio::sync::RwLock;

use crate::{
    community::provider::{ProgressData, ProviderState},
    pluginstore::provider::{Provider, StorePluginManifest},
};

#[derive(Debug, Clone, Copy)]
enum Cdn {
    Raw,
    JsDelivr,
    GhFast,
    JsdCdn,
}

impl Cdn {
    fn convert_url(self, url: &str) -> String {
        if !url.contains("https://raw.githubusercontent.com/") {
            return url.to_owned();
        }
        match self {
            Cdn::Raw => url.to_owned(),
            Cdn::JsDelivr => url
                .replace(
                    "https://raw.githubusercontent.com/",
                    "https://cdn.jsdelivr.net/gh/",
                )
                .replace("/refs/heads/main/", "@main/"),
            Cdn::GhFast => format!(
                "https://ghfast.top/{}",
                url.strip_prefix("https://").unwrap_or(url)
            ),
            Cdn::JsdCdn => url
                .replace(
                    "https://raw.githubusercontent.com/",
                    "https://cdn.615873.xyz/gh/",
                )
                .replace("/refs/heads/main/", "@main/"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OfficialPluginStore {
    client: Client,
    cdn: Cdn,
    // (repo_base, folder, manifest)
    index: Arc<RwLock<Vec<(String, String, StorePluginManifest)>>>,
    state: Arc<RwLock<ProviderState>>,
    base: String,
}

impl OfficialPluginStore {
    pub fn new(base: &str, cdn: Option<&str>) -> Self {
        let cdn = match cdn.unwrap_or("raw") {
            "jsdelivr" => Cdn::JsDelivr,
            "ghfast" => Cdn::GhFast,
            "jsdcdn" => Cdn::JsdCdn,
            _ => Cdn::Raw,
        };
        Self {
            client: crate::net::default_client(),
            cdn,
            index: Arc::new(RwLock::new(Vec::new())),
            state: Arc::new(RwLock::new(ProviderState::Updating)),
            base: base.trim_end_matches('/').to_string() + "/",
        }
    }

    fn to_raw(&self, repo: &str) -> String {
        if repo.starts_with("https://raw.githubusercontent.com/") {
            repo.trim_end_matches('/').to_string() + "/"
        } else if repo.starts_with("https://github.com/") {
            let raw = repo
                .trim_end_matches('/')
                .replace("https://github.com/", "https://raw.githubusercontent.com/");
            raw + "/refs/heads/main/"
        } else {
            repo.trim_end_matches('/').to_string() + "/"
        }
    }

    fn build_file_url(&self, repo: &str, folder: &str, file: &str) -> String {
        let raw = self.to_raw(repo);
        self.cdn.convert_url(&format!(
            "{}{}{}",
            raw,
            folder.trim_end_matches('/').to_string() + "/",
            file
        ))
    }

    async fn refresh_index(&self) -> Result<()> {
        *self.state.write().await = ProviderState::Updating;
        let url = self.cdn.convert_url(&format!("{}index.txt", self.base));
        let text = self.client.get(&url).send().await?.text().await?;
        let mut list = Vec::new();
        for repo in text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
            let repo_raw = self.to_raw(repo);
            let idx_url = self.cdn.convert_url(&format!("{}index.txt", repo_raw));
            let idx_txt = self.client.get(&idx_url).send().await?.text().await?;
            for folder in idx_txt.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                let manifest_url = self
                    .cdn
                    .convert_url(&format!("{}{}/manifest.json", repo_raw, folder));
                let manifest: StorePluginManifest =
                    self.client.get(&manifest_url).send().await?.json().await?;
                list.push((repo.to_string(), folder.to_string(), manifest));
            }
        }
        *self.index.write().await = list;
        *self.state.write().await = ProviderState::Ready;
        Ok(())
    }

    fn fuzzy_match(a: &str, b: &str) -> bool {
        a.to_lowercase().contains(&b.to_lowercase())
    }
}

#[async_trait]
impl Provider for OfficialPluginStore {
    fn provider_name(&self) -> String {
        "official".into()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn refresh(&self) -> Result<()> {
        self.refresh_index().await
    }

    fn state(&self) -> ProviderState {
        match self.state.try_read() {
            Ok(g) => g.clone(),
            Err(_) => ProviderState::Updating,
        }
    }

    async fn get_page(
        &self,
        page: u32,
        limit: u32,
        filter: Option<String>,
    ) -> Result<Vec<StorePluginManifest>> {
        let idx = self.index.read().await;
        let mut filtered: Vec<_> = idx
            .iter()
            .filter(|(_, _, m)| {
                filter
                    .as_ref()
                    .map_or(true, |f| Self::fuzzy_match(&m.name, f))
            })
            .collect();
        filtered.sort_by(|a, b| a.2.name.cmp(&b.2.name));
        let start = (page * limit) as usize;
        let end = ((page + 1) * limit).min(filtered.len() as u32) as usize;
        if start >= end {
            return Ok(vec![]);
        }
        let mut res = Vec::new();
        for (repo, folder, manifest) in filtered[start..end].iter() {
            let mut m = manifest.clone();
            m.icon = self.build_file_url(repo, folder, &manifest.icon);
            m.entry = self.build_file_url(repo, folder, &manifest.entry);
            m.additional_files = manifest
                .additional_files
                .iter()
                .map(|f| self.build_file_url(repo, folder, f))
                .collect();
            res.push(m);
        }
        Ok(res)
    }

    async fn get_item(&self, name: String) -> Result<StorePluginManifest> {
        let idx = self.index.read().await;
        let (repo, folder, manifest) = idx
            .iter()
            .find(|(_, _, m)| m.name == name)
            .ok_or_else(|| anyhow!("plugin not found"))?;
        let mut m = manifest.clone();
        m.icon = self.build_file_url(repo, folder, &manifest.icon);
        m.entry = self.build_file_url(repo, folder, &manifest.entry);
        m.additional_files = manifest
            .additional_files
            .iter()
            .map(|f| self.build_file_url(repo, folder, f))
            .collect();
        Ok(m)
    }

    async fn download(&self, name: String, progress_cb: Channel<ProgressData>) -> Result<String> {
        let idx = self.index.read().await;
        let (repo, folder, manifest) = idx
            .iter()
            .find(|(_, _, m)| m.name == name)
            .ok_or_else(|| anyhow!("plugin not found"))?;
        let mut files: Vec<(String, Bytes)> = Vec::new();
        let manifest_url =
            self.cdn
                .convert_url(&format!("{}{}/manifest.json", self.to_raw(repo), folder));
        files.push((
            "manifest.json".to_string(),
            self.client.get(&manifest_url).send().await?.bytes().await?,
        ));
        let entry_bytes = self
            .client
            .get(&self.build_file_url(repo, folder, &manifest.entry))
            .send()
            .await?
            .bytes()
            .await?;
        files.push((manifest.entry.clone(), entry_bytes));
        let icon_bytes = self
            .client
            .get(&self.build_file_url(repo, folder, &manifest.icon))
            .send()
            .await?
            .bytes()
            .await?;
        files.push((manifest.icon.clone(), icon_bytes));
        for f in &manifest.additional_files {
            let bytes = self
                .client
                .get(&self.build_file_url(repo, folder, f))
                .send()
                .await?
                .bytes()
                .await?;
            files.push((f.clone(), bytes));
        }
        drop(idx);

        progress_cb.send(ProgressData {
            progress: 0.5,
            status: "downloading".into(),
            status_text: "Packaging".into(),
        })?;

        let cache_dir = crate::APP_HANDLE
            .get()
            .unwrap()
            .path()
            .app_cache_dir()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let tmp_dir = format!("{}/tmp", cache_dir);

        // 确保 tmp 目录存在
        tokio::fs::create_dir_all(&tmp_dir).await?;

        let tmp_path = format!(
            "{}/{}-{}.abp",
            tmp_dir,
            crate::tools::random_string(10),
            name
        );

        let path = tokio::task::spawn_blocking(move || -> Result<String> {
            let f = std::fs::File::create(&tmp_path)?;
            let mut zip = zip::ZipWriter::new(f);
            let opt = zip::write::FileOptions::<()>::default();
            for (path, data) in files {
                zip.start_file(path, opt)?;
                zip.write_all(&data)?;
            }
            zip.finish()?;
            Ok(tmp_path)
        })
        .await??;

        Ok(path)
    }

    async fn get_total_items(&self, filter: Option<String>) -> Result<u64> {
        let idx = self.index.read().await;
        Ok(match filter {
            Some(f) => idx
                .iter()
                .filter(|(_, _, m)| Self::fuzzy_match(&m.name, &f))
                .count() as u64,
            None => idx.len() as u64,
        })
    }
}
