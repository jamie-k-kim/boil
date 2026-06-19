use crate::ports::ownership::{OwnershipProvider, OwnershipRule};
use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct GitHubMember {
    login: String,
}

pub struct GithubTeamsProvider {
    token: String,
}

impl Default for GithubTeamsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GithubTeamsProvider {
    pub fn new() -> Self {
        let token =
            std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env var required for GitHub Teams");
        Self { token }
    }

    fn fetch_team_members(&self, org: &str, team_slug: &str) -> Result<Vec<String>> {
        let url = format!(
            "https://api.github.com/orgs/{}/teams/{}/members",
            org, team_slug
        );

        let response: Vec<GitHubMember> = ureq::get(&url)
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "boil-agent")
            .call()?
            .body_mut()
            .read_json()?;

        Ok(response
            .into_iter()
            .map(|m| format!("@{}", m.login))
            .collect())
    }
}

impl OwnershipProvider for GithubTeamsProvider {
    fn get_ownership_info(&self, project_root: &Path) -> Result<Vec<OwnershipRule>> {
        let mut rules = Vec::new();

        let codeowners_paths = vec![
            project_root.join("CODEOWNERS"),
            project_root.join(".github/CODEOWNERS"),
            project_root.join("docs/CODEOWNERS"),
        ];

        let mut codeowners_content = String::new();
        for path in codeowners_paths {
            if path.exists() {
                codeowners_content = fs::read_to_string(path).unwrap_or_default();
                break;
            }
        }

        if codeowners_content.is_empty() {
            return Ok(rules);
        }

        for line in codeowners_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut parts = line.split_whitespace();
            if let Some(pattern) = parts.next() {
                let mut owners = Vec::new();
                for owner in parts {
                    if owner.starts_with("@") && owner.contains('/') {
                        // Looks like a team: @org/team
                        let trimmed = &owner[1..];
                        let org_team: Vec<&str> = trimmed.split('/').collect();
                        if org_team.len() == 2 {
                            if let Ok(members) = self.fetch_team_members(org_team[0], org_team[1]) {
                                owners.extend(members);
                            } else {
                                owners.push(owner.to_string()); // Fallback
                            }
                        } else {
                            owners.push(owner.to_string());
                        }
                    } else {
                        owners.push(owner.to_string());
                    }
                }

                if !owners.is_empty() {
                    rules.push(OwnershipRule {
                        pattern: pattern.to_string(),
                        owners,
                    });
                }
            }
        }

        Ok(rules)
    }
}
