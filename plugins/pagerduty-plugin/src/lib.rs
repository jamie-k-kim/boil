use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct OwnershipRule {
    pub pattern: String,
    pub owners: Vec<String>,
}

#[derive(Deserialize)]
struct PdOnCallResponse {
    oncalls: Vec<OnCall>,
}

#[derive(Deserialize)]
struct OnCall {
    user: PdUser,
    escalation_policy: PdPolicy,
}

#[derive(Deserialize)]
struct PdUser {
    summary: String,
}

#[derive(Deserialize)]
struct PdPolicy {
    summary: String,
}

#[plugin_fn]
pub fn get_ownership_info(_input: String) -> FnResult<String> {
    let mut rules = Vec::new();

    // In a real plugin, we would securely pull API tokens from the plugin configuration:
    let api_key = config::get("pagerduty_api_key");
    
    if let Ok(Some(token)) = api_key {
        let req = HttpRequest::new("https://api.pagerduty.com/oncalls")
            .with_header("Accept", "application/vnd.pagerduty+json;version=2")
            .with_header("Authorization", format!("Token token={}", token));
            
        if let Ok(res) = http::request::<()>(&req, None) {
            if let Ok(parsed) = serde_json::from_slice::<PdOnCallResponse>(&res.body()) {
                for oncall in parsed.oncalls {
                    // E.g., Map "src/billing Policy" -> "src/billing/**"
                    let dir_pattern = oncall.escalation_policy.summary.replace(" Policy", "/**");
                    rules.push(OwnershipRule {
                        pattern: dir_pattern,
                        owners: vec![oncall.user.summary],
                    });
                }
            }
        }
    }

    // Fallback: If no API key is provided, demonstrate dynamic resolution with mock data
    if rules.is_empty() {
        rules.push(OwnershipRule {
            pattern: "src/adapters/temporal/**".to_string(),
            owners: vec!["Alice Smith (On-Call)".to_string()],
        });
        rules.push(OwnershipRule {
            pattern: "src/adapters/input/**".to_string(),
            owners: vec!["Bob Jones (On-Call)".to_string()],
        });
        rules.push(OwnershipRule {
            pattern: "src/core/**".to_string(),
            owners: vec!["Charlie Brown (On-Call)".to_string()],
        });
    }

    let output = serde_json::to_string(&rules)?;
    Ok(output)
}
