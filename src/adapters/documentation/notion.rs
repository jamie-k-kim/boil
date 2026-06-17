use crate::ports::documentation::{DocumentInfo, DocumentationProvider};
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct NotionResponse {
    results: Vec<NotionPage>,
}

#[derive(Deserialize)]
struct NotionPage {
    id: String,
    url: String,
    // Omitting complex property parsing for the MVP
}

#[derive(Deserialize)]
struct NotionBlockResponse {
    results: Vec<NotionBlock>,
}

#[derive(Deserialize)]
struct NotionBlock {
    #[serde(rename = "type")]
    block_type: String,
    paragraph: Option<NotionParagraph>,
}

#[derive(Deserialize)]
struct NotionParagraph {
    rich_text: Vec<NotionRichText>,
}

#[derive(Deserialize)]
struct NotionRichText {
    plain_text: String,
}

pub struct NotionProvider {
    api_key: String,
    database_id: String,
}

impl Default for NotionProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl NotionProvider {
    pub fn new() -> Self {
        let api_key = std::env::var("NOTION_API_KEY").expect("NOTION_API_KEY required");
        let database_id = std::env::var("NOTION_DATABASE_ID").expect("NOTION_DATABASE_ID required");
        Self {
            api_key,
            database_id,
        }
    }

    fn fetch_page_content(&self, page_id: &str) -> Result<String> {
        let url = format!("https://api.notion.com/v1/blocks/{}/children", page_id);

        let response: NotionBlockResponse = ureq::get(&url)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Notion-Version", "2022-06-28")
            .call()?
            .body_mut()
            .read_json()?;

        let mut content = String::new();
        for block in response.results {
            if block.block_type == "paragraph"
                && let Some(paragraph) = block.paragraph
            {
                for text in paragraph.rich_text {
                    content.push_str(&text.plain_text);
                }
                content.push('\n');
            }
        }
        Ok(content)
    }
}

impl DocumentationProvider for NotionProvider {
    fn analyze_workspace(&self, project_root: &Path) -> Result<Vec<DocumentInfo>> {
        let url = format!(
            "https://api.notion.com/v1/databases/{}/query",
            self.database_id
        );

        let response: NotionResponse = ureq::post(&url)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Notion-Version", "2022-06-28")
            .header("Content-Type", "application/json")
            .send_json(serde_json::json!({}))?
            .body_mut()
            .read_json()?;

        let cache_dir = project_root.join(".boil").join("cache").join("notion");
        fs::create_dir_all(&cache_dir)?;

        let mut docs = Vec::new();

        for page in response.results {
            if let Ok(content) = self.fetch_page_content(&page.id) {
                let file_path = cache_dir.join(format!("{}.md", page.id));
                fs::write(&file_path, &content)?;

                let mut sections = HashMap::new();
                sections.insert("content".to_string(), content);
                sections.insert("url".to_string(), page.url);

                docs.push(DocumentInfo {
                    path: file_path,
                    kind: "notion".to_string(),
                    sections,
                });
            }
        }

        Ok(docs)
    }
}
