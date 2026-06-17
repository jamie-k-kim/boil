use crate::ports::ownership::{OwnershipProvider, OwnershipRule};
use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct JiraComponent {
    name: String,
    lead: Option<JiraUser>,
}

#[derive(Deserialize)]
struct JiraUser {
    #[serde(rename = "emailAddress")]
    email_address: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

pub struct JiraProvider {
    domain: String,
    token: String,
    project_key: String,
}

impl Default for JiraProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl JiraProvider {
    pub fn new() -> Self {
        let domain = std::env::var("JIRA_DOMAIN").expect("JIRA_DOMAIN env var required");
        let token = std::env::var("JIRA_API_TOKEN").expect("JIRA_API_TOKEN env var required");
        let project_key =
            std::env::var("JIRA_PROJECT_KEY").expect("JIRA_PROJECT_KEY env var required");

        Self {
            domain,
            token,
            project_key,
        }
    }
}

impl OwnershipProvider for JiraProvider {
    fn get_ownership_info(&self, _project_root: &Path) -> Result<Vec<OwnershipRule>> {
        let url = format!(
            "https://{}/rest/api/3/project/{}/components",
            self.domain, self.project_key
        );

        let response: Vec<JiraComponent> = ureq::get(&url)
            .header("Authorization", &format!("Basic {}", self.token))
            .header("Accept", "application/json")
            .call()?
            .body_mut()
            .read_json()?;

        let mut rules = Vec::new();
        for component in response {
            if let Some(lead) = component.lead {
                let owner = lead
                    .email_address
                    .unwrap_or_else(|| lead.display_name.unwrap_or_else(|| "unknown".to_string()));
                rules.push(OwnershipRule {
                    pattern: format!("{}/**", component.name),
                    owners: vec![owner],
                });
            }
        }

        Ok(rules)
    }
}
