use crate::ports::runtime::{RuntimeProvider, RuntimeTrace};
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize)]
struct JaegerResponse {
    data: Vec<JaegerTrace>,
}

#[derive(Deserialize)]
struct JaegerTrace {
    spans: Vec<JaegerSpan>,
}

#[derive(Deserialize)]
struct JaegerSpan {
    tags: Vec<JaegerTag>,
}

#[derive(Deserialize)]
struct JaegerTag {
    key: String,
    value: serde_json::Value,
}

pub struct OpentelemetryProvider {
    jaeger_url: String,
    service_name: String,
}

impl Default for OpentelemetryProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OpentelemetryProvider {
    pub fn new() -> Self {
        let jaeger_url = std::env::var("OTEL_JAEGER_URL")
            .unwrap_or_else(|_| "http://localhost:16686".to_string());
        let service_name = std::env::var("OTEL_SERVICE_NAME").expect("OTEL_SERVICE_NAME required");
        Self {
            jaeger_url,
            service_name,
        }
    }
}

impl RuntimeProvider for OpentelemetryProvider {
    fn get_traces(&self, _project_root: &Path) -> Result<Vec<RuntimeTrace>> {
        let url = format!(
            "{}/api/traces?service={}&limit=1000",
            self.jaeger_url, self.service_name
        );

        let response: JaegerResponse = ureq::get(&url).call()?.body_mut().read_json()?;

        let mut counts = HashMap::new();

        for trace in response.data {
            for span in trace.spans {
                let mut filepath = String::new();
                let mut function = String::new();

                for tag in span.tags {
                    if tag.key == "code.filepath" {
                        if let Some(s) = tag.value.as_str() {
                            filepath = s.to_string();
                        }
                    } else if tag.key == "code.function"
                        && let Some(s) = tag.value.as_str()
                    {
                        function = s.to_string();
                    }
                }

                if !filepath.is_empty() && !function.is_empty() {
                    let key = format!("{}::{}", filepath, function);
                    let entry = counts.entry(key).or_insert((filepath, function, 0));
                    entry.2 += 1;
                }
            }
        }

        let mut traces = Vec::new();
        for (_, (file, symbol, count)) in counts {
            traces.push(RuntimeTrace {
                file,
                symbol,
                count,
            });
        }

        Ok(traces)
    }
}
