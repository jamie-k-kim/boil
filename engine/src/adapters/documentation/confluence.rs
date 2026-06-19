use crate::ports::documentation::{DocumentInfo, DocumentationProvider};
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct ConfluenceResponse {
    results: Vec<ConfluencePage>,
}

#[derive(Deserialize)]
struct ConfluencePage {
    id: String,
    title: String,
    body: Option<ConfluenceBody>,
    #[serde(rename = "_links")]
    links: ConfluenceLinks,
}

#[derive(Deserialize)]
struct ConfluenceBody {
    storage: Option<ConfluenceStorage>,
}

#[derive(Deserialize)]
struct ConfluenceStorage {
    value: String,
}

#[derive(Deserialize)]
struct ConfluenceLinks {
    base: String,
    webui: String,
}

pub struct ConfluenceProvider {
    domain: String,
    token: String,
    space_key: String,
}

impl Default for ConfluenceProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfluenceProvider {
    pub fn new() -> Self {
        let domain = std::env::var("CONFLUENCE_DOMAIN").expect("CONFLUENCE_DOMAIN required");
        let token = std::env::var("CONFLUENCE_API_TOKEN").expect("CONFLUENCE_API_TOKEN required");
        let space_key =
            std::env::var("CONFLUENCE_SPACE_KEY").expect("CONFLUENCE_SPACE_KEY required");
        Self {
            domain,
            token,
            space_key,
        }
    }
}

impl DocumentationProvider for ConfluenceProvider {
    fn analyze_workspace(&self, project_root: &Path) -> Result<Vec<DocumentInfo>> {
        let url = format!(
            "https://{}/wiki/rest/api/content?spaceKey={}&expand=body.storage",
            self.domain, self.space_key
        );

        let response: ConfluenceResponse = ureq::get(&url)
            .header("Authorization", &format!("Basic {}", self.token))
            .header("Accept", "application/json")
            .call()?
            .body_mut()
            .read_json()?;

        let cache_dir = project_root.join(".boil").join("cache").join("confluence");
        fs::create_dir_all(&cache_dir)?;

        let mut docs = Vec::new();

        for page in response.results {
            if let Some(body) = page.body
                && let Some(storage) = body.storage
            {
                let content = storage.value;
                let file_path = cache_dir.join(format!("{}.html", page.id));
                fs::write(&file_path, &content)?;

                let mut sections = HashMap::new();
                sections.insert("title".to_string(), page.title);
                sections.insert("content".to_string(), content);
                sections.insert(
                    "url".to_string(),
                    format!("{}{}", page.links.base, page.links.webui),
                );

                docs.push(DocumentInfo {
                    path: file_path,
                    kind: "confluence".to_string(),
                    sections,
                });
            }
        }

        Ok(docs)
    }
}
