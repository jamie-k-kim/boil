use extism_pdk::*;
use serde::Serialize;

#[derive(Serialize)]
pub struct OwnershipRule {
    pub pattern: String,
    pub owners: Vec<String>,
}

#[plugin_fn]
pub fn get_ownership_info(input: String) -> FnResult<String> {
    let _project_root: String = serde_json::from_str(&input)?;
    
    // TODO: Implement your custom ownership logic here
    let rules = vec![
        OwnershipRule {
            pattern: "src/**".to_string(),
            owners: vec!["@core-team".to_string()],
        }
    ];

    let output = serde_json::to_string(&rules)?;
    Ok(output)
}
