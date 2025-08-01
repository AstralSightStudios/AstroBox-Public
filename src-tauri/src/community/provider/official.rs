use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use reqwest::Client;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Instant;
use std::{any::Any, path::Path, sync::Arc};
use tauri::{ipc::Channel, Manager};
use tokio::{fs::File, io::AsyncWriteExt, sync::RwLock};
use tokio_stream::StreamExt;

use crate::{
    community::provider::{
        official::models::{Banner, IndexRes, ResourceDest},
        Item, ProgressData, Provider, ProviderState, ResourceManifestV1, SearchConfig,
    },
    miwear::device::models::DeviceMap,
};

pub mod models;

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
pub struct OfficialProvider {
    client: Client,
    cdn: Cdn,
    index: Arc<RwLock<Vec<IndexRes>>>,
    state: Arc<RwLock<ProviderState>>,
    seed: Arc<RwLock<u64>>,
}

impl OfficialProvider {
    pub fn new(cdn: Option<&str>) -> Self {
        let cdn = match cdn.unwrap_or("raw") {
            "jsdelivr" => Cdn::JsDelivr,
            "ghfast" => Cdn::GhFast,
            "jsdcdn" => Cdn::JsdCdn,
            _ => Cdn::Raw,
        };
        let provider = Self {
            client: crate::net::default_client(),
            cdn,
            index: Arc::new(RwLock::new(Vec::new())),
            state: Arc::new(RwLock::new(ProviderState::Updating)),
            seed: Arc::new(RwLock::new(rand::random())),
        };
        provider
    }

    pub async fn get_device_map(&self) -> anyhow::Result<DeviceMap> {
        let url = self.cdn.convert_url("https://raw.githubusercontent.com/AstralSightStudios/AstroBox-Repo/refs/heads/main/devices.json5");
        let client = crate::net::default_client();
        let text = client.get(&url).send().await?.text().await?;
        let device_map: DeviceMap = serde_json5::from_str(&text)?;

        Ok(device_map)
    }

    /// 获取官方仓库的 index 列表
    async fn refresh_index(&self) -> Result<()> {
        *self.state.write().await = ProviderState::Updating;
        let url = self.cdn.convert_url(
            "https://raw.githubusercontent.com/AstralSightStudios/AstroBox-Repo/refs/heads/main/index.csv"
        );
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        let raw = resp.bytes().await?;
        let mut list: Vec<IndexRes> = vec![];
        let mut csv_read = csv::Reader::from_reader(raw.as_ref());
        for it in csv_read.deserialize() {
            list.push(it?);
        }
        *self.index.write().await = list;
        *self.seed.write().await = rand::random();
        *self.state.write().await = ProviderState::Ready;
        Ok(())
    }

    pub async fn get_banner(&self) -> Result<Vec<Banner>> {
        let url = self.cdn.convert_url(
            "https://raw.githubusercontent.com/AstralSightStudios/AstroBox-Repo/refs/heads/main/banner.json"
        );
        let resp = self.client.get(&url).send().await?.text().await?;
        let mut banners: Vec<Banner> = serde_json::from_str(&resp)?;
        for banner in &mut banners {
            banner.background = self.cdn.convert_url(&format!(
                "https://raw.githubusercontent.com/AstralSightStudios/AstroBox-Repo/refs/heads/main/{}",
                banner.background
            ));
            banner.foreground = self.cdn.convert_url(&format!(
                "https://raw.githubusercontent.com/AstralSightStudios/AstroBox-Repo/refs/heads/main/{}",
                banner.foreground
            ));
        }

        Ok(banners)
    }

    /// 构建 raw.githubusercontent 的资源 json 路径
    fn build_resource_url(&self, path: &str) -> String {
        let raw_url = format!(
            "https://raw.githubusercontent.com/AstralSightStudios/AstroBox-Repo/refs/heads/main/resources/{}",
            path
        );
        self.cdn.convert_url(&raw_url)
    }

    /// 将 repo_url 转换为 manifest 的 raw 路径
    fn build_manifest_url(&self, repo_url: &str) -> String {
        let raw = repo_url.replace("https://github.com/", "https://raw.githubusercontent.com/");
        self.cdn
            .convert_url(&format!("{}/refs/heads/main/manifest.json", raw))
    }

    /// 将 repo_url + file_name 转换为下载文件的 raw 路径
    fn build_download_url(&self, repo_url: &str, file_name: &str) -> String {
        let raw = repo_url.replace("https://github.com/", "https://raw.githubusercontent.com/");
        self.cdn
            .convert_url(&format!("{}/refs/heads/main/{}", raw, file_name))
    }

    async fn fetch_json<T: for<'a> serde::de::Deserialize<'a>>(&self, url: &str) -> Result<T> {
        Ok(self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    async fn fetch_dest(&self, path: &str) -> Result<ResourceDest> {
        let url = self.build_resource_url(path);
        log::info!("[Official Repo] fetch dest: {}", &url);
        self.fetch_json(&url).await
    }

    async fn fetch_manifest(&self, repo_url: &str) -> Result<ResourceManifestV1> {
        let url = self.build_manifest_url(repo_url);
        log::info!("[Official Repo] fetch manifest: {}", &url);
        self.fetch_json(&url).await
    }

    fn fuzzy_match(text: &str, keyword: &str) -> bool {
        text.to_lowercase().contains(&keyword.to_lowercase())
    }
}

#[async_trait]
impl Provider for OfficialProvider {
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
            Ok(guard) => guard.clone(),
            Err(_) => ProviderState::Updating,
        }
    }

    async fn get_categories(&self) -> anyhow::Result<Vec<String>> {
        let device_map = self.get_device_map().await?;
        let mut categories: Vec<String> = device_map
            .values()
            .map(|item| item.codename.clone())
            .collect();
        categories.sort();
        categories.dedup();
        categories.splice(0..0, vec!["hidden_paid".to_string(), "hidden_force_paid".to_string()]);
        Ok(categories)
    }

    async fn get_page(&self, page: u32, limit: u32, search: SearchConfig) -> Result<Vec<Item>> {
        //log::info!("{}", serde_json::to_string(&search).unwrap());
        let idx = self.index.read().await;
        let (mut device_list, mut hidden_paid_list) = (vec![], vec![]);
                if let Some(ref cat_vec) = search.category {
                    for val in cat_vec {
                        match val.as_str() {
                            "hidden_paid" | "hidden_force_paid" => {
                                let str = val.replace("hidden_", "");
                                hidden_paid_list.push(str);
                            }
                            _ if !val.is_empty() => device_list.push(val.as_str()),
                            _ => {}
                        }
                    }
                }
        let mut filtered: Vec<_> = idx
            .iter()
            .filter(|res| {
                let keyword_ok = search.filter.as_ref().map_or(true, |kw| {
                    Self::fuzzy_match(&res.name, kw)
                        || res.tags.iter().any(|t| Self::fuzzy_match(t, kw))
                });

                let device_ok = if device_list.is_empty() {
                    true
                } else {
                    device_list
                        .iter()
                        .any(|d| res.devices.iter().any(|r| r == *d))
                };

                let hidden_paid_ok = if !hidden_paid_list.is_empty() {
                    hidden_paid_list.iter().all(|p| p != &res.paid_type)
                } else {
                    true
                };

                keyword_ok && device_ok && hidden_paid_ok
            })
            .collect();

        let need_shuffle = search.filter.unwrap().is_empty() && (search.category.unwrap().len() < 1);
        if need_shuffle {
            let mut hasher = DefaultHasher::new();
            {
                let seed_guard = self.seed.read().await;
                seed_guard.hash(&mut hasher);
            }
            let seed = hasher.finish();
            let mut rng = StdRng::seed_from_u64(seed);
            filtered.shuffle(&mut rng);
        }

        let start = (page * limit) as usize;
        let end = ((page + 1) * limit).min(filtered.len() as u32) as usize;
        if start >= end {
            return Ok(vec![]);
        }

        Ok(filtered[start..end]
            .iter()
            .map(|res| Item {
                name: Some(res.name.clone()),
                restype: Some(res.restype.clone()),
                description: Some(res.restype.clone()),
                preview: Some(vec![self.cdn.convert_url(&res.cover)]),
                icon: Some(self.cdn.convert_url(&res.icon)),
                source_url: None,
                author: None,
                paid_type: res.paid_type.clone(),
                _bandbbs_ext_supported_device: None,
                _bandbbs_ext_resource_id: None,
                _bandbbs_ext_is_community_paid: None,
            })
            .collect())
    }

    async fn get_item(&self, name: String) -> Result<ResourceManifestV1> {
        log::info!("get item name={}", &name);
        let idx = self.index.read().await;
        let res = idx
            .iter()
            .find(|val| val.name == name)
            .ok_or_else(|| anyhow!("resource not found"))?;
        let dest = self.fetch_dest(&res.path).await?;
        let mut manifest = self.fetch_manifest(&dest.repo_url).await?;
        manifest.item.icon =
            Some(self.build_download_url(&dest.repo_url, &manifest.item.icon.unwrap()));
        if let Some(preview_vec) = &mut manifest.item.preview {
            for img in preview_vec.iter_mut() {
                *img = self.build_download_url(&dest.repo_url, img);
            }
        }
        Ok(manifest)
    }

    async fn download(
        &self,
        name: String,
        device: String,
        progress_cb: Channel<ProgressData>,
    ) -> Result<String> {
        let idx = self.index.read().await;
        let res = idx
            .iter()
            .find(|val| val.name == name)
            .ok_or_else(|| anyhow!("resource not found"))?;
        let dest = self.fetch_dest(&res.path).await?;
        let manifest = self.fetch_manifest(&dest.repo_url).await?;

        let file_name = manifest
            .downloads
            .get(&device)
            .ok_or_else(|| anyhow!("device not supported"))?
            .file_name
            .clone();

        let url = self.build_download_url(&dest.repo_url, &file_name);

        let tmp_path = format!(
            "{}/tmp/{}-{}",
            crate::APP_HANDLE
                .get()
                .unwrap()
                .path()
                .app_cache_dir()
                .unwrap()
                .to_string_lossy(),
            crate::tools::random_string(10),
            file_name
        );
        let parent = Path::new(&tmp_path).parent().unwrap();
        tokio::fs::create_dir_all(parent).await?;
        let mut file = File::create(&tmp_path).await?;

        let resp = self.client.get(&url).send().await?.error_for_status()?;
        let total = resp.content_length().unwrap_or(0);
        let mut last_update = Instant::now();
        let mut downloaded: u64 = 0;
        let mut stream = resp.bytes_stream();

        while let Some(chunk_res) = stream.next().await {
            let chunk: Bytes = chunk_res?;
            downloaded += chunk.len() as u64;
            file.write_all(&chunk).await?;

            if last_update.elapsed().as_millis() > 100
                || (total > 0 && downloaded % (total / 100) == 0)
            {
                let pct = if total > 0 {
                    downloaded as f32 / total as f32
                } else {
                    0.0
                };
                progress_cb.send(ProgressData {
                    progress: pct,
                    status: "downloading".into(),
                    status_text: format!("{:.2}%", pct * 100.0),
                })?;
                last_update = Instant::now();
            }
        }

        progress_cb.send(ProgressData {
            progress: 1.0,
            status: "finished".into(),
            status_text: "下载完成".into(),
        })?;

        Ok(tmp_path)
    }

    async fn get_total_items(&self, filter: Option<String>) -> Result<u64> {
        let idx = self.index.read().await;
        Ok(match filter {
            Some(kw) => idx
                .iter()
                .filter(|it| Self::fuzzy_match(&it.name, &kw))
                .count() as u64,
            None => idx.len() as u64,
        })
    }
}
