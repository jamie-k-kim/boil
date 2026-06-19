use std::io::{self, BufRead};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use crate::config;
use crate::batch::Batch;
use crate::commands;

#[derive(Deserialize)]
#[allow(dead_code)]
struct RpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct RpcResponse {
    jsonrpc: String,
    id: Value,
    result: Value,
}

#[derive(Serialize)]
struct RpcErrorResponse {
    jsonrpc: String,
    id: Option<Value>,
    error: RpcError,
}

#[derive(Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

fn send_response(id: Value, result: Value) {
    use std::io::Write;
    let res = RpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result,
    };
    let out = serde_json::to_string(&res).unwrap();
    println!("{}", out);
    let _ = io::stdout().flush();
}

fn send_error(id: Option<Value>, code: i32, message: String) {
    use std::io::Write;
    let res = RpcErrorResponse {
        jsonrpc: "2.0".to_string(),
        id,
        error: RpcError { code, message },
    };
    let out = serde_json::to_string(&res).unwrap();
    println!("{}", out);
    let _ = io::stdout().flush();
}

use std::collections::HashMap;

struct ServerState {
    config: config::Config,
    batch: Option<Batch>,
    index: Option<Vec<crate::batch::FileInfo>>,
    symbol_map: Option<HashMap<String, Vec<(usize, usize)>>>,
}

impl ServerState {
    fn new() -> Self {
        let config = config::load_config().unwrap_or_default();
        let mut state = ServerState {
            config,
            batch: None,
            index: None,
            symbol_map: None,
        };
        state.reload_batch();
        state
    }

    fn reload_batch(&mut self) {
        if let Some(ref path) = self.config.last_batch {
            if let Ok(batch) = Batch::load(path.clone()) {
                let index_path = batch.root.join("index.json");
                let mut index = None;
                let mut symbol_map = None;
                if let Ok(content) = std::fs::read_to_string(&index_path) {
                    if let Ok(idx) = serde_json::from_str::<Vec<crate::batch::FileInfo>>(&content) {
                        let mut sym_map: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
                        for (f_idx, file) in idx.iter().enumerate() {
                            for (s_idx, sym) in file.symbols.iter().enumerate() {
                                sym_map.entry(sym.name.clone()).or_default().push((f_idx, s_idx));
                            }
                        }
                        index = Some(idx);
                        symbol_map = Some(sym_map);
                    }
                }
                self.batch = Some(batch);
                self.index = index;
                self.symbol_map = symbol_map;
                return;
            }
        }
        self.batch = None;
        self.index = None;
        self.symbol_map = None;
    }
}

pub fn run_mcp_server() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();
    let mut state = ServerState::new();

    loop {
        buffer.clear();
        if handle.read_line(&mut buffer)? == 0 {
            break; // EOF
        }

        let req: RpcRequest = match serde_json::from_str(&buffer) {
            Ok(r) => r,
            Err(_) => {
                // Not JSON-RPC or empty line, ignore
                continue;
            }
        };

        if req.method == "initialize" {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "Steampipe",
                    "version": "0.1.0"
                }
            });
            if let Some(id) = req.id {
                send_response(id, result);
            }
        } else if req.method == "notifications/initialized" {
            // Nothing to do
        } else if req.method == "tools/list" {
            let result = json!({
                "tools": [
                    {
                        "name": "boil_set_batch",
                        "description": "Latch onto a specific batch for Steampipe to use.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "boil_status",
                        "description": "Show the status of the currently active batch.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "boil_reset",
                        "description": "Reset your active batch configuration.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "boil_ls",
                        "description": "List files and directories inside a specific layer.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "layer": { "type": "string" },
                                "path": { "type": "string" }
                            },
                            "required": ["layer"]
                        }
                    },
                    {
                        "name": "boil_find",
                        "description": "Search for a specific code symbol across the active batch.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "symbol": { "type": "string" }
                            },
                            "required": ["symbol"]
                        }
                    },
                    {
                        "name": "boil_read_file",
                        "description": "Read the contents of a file at the specified layer.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "layer": { "type": "string" },
                                "file": { "type": "string" }
                            },
                            "required": ["layer", "file"]
                        }
                    },
                    {
                        "name": "boil_read_symbol",
                        "description": "Read a specific symbol definition at the specified layer.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "layer": { "type": "string" },
                                "symbol": { "type": "string" },
                                "id": { "type": "integer" }
                            },
                            "required": ["layer", "symbol"]
                        }
                    },
                    {
                        "name": "boil_write",
                        "description": "Write or insert code at a specific line in a file.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file": { "type": "string" },
                                "line": { "type": "integer" },
                                "content": { "type": "string" }
                            },
                            "required": ["file", "line"]
                        }
                    },
                    {
                        "name": "boil_delete",
                        "description": "Delete a specific line of code from a file.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file": { "type": "string" },
                                "line": { "type": "integer" }
                            },
                            "required": ["file", "line"]
                        }
                    }
                ]
            });
            if let Some(id) = req.id {
                send_response(id, result);
            }
        } else if req.method == "tools/call" {
            if let Some(id) = req.id {
                let params = req.params.unwrap_or_else(|| json!({}));
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

                let output_result = match name {
                    "boil_set_batch" => {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        let p = std::path::PathBuf::from(path);
                        match Batch::load(p.clone()) {
                            Ok(_) => {
                                state.config.last_batch = Some(p.clone());
                                let _ = config::save_config(&state.config);
                                state.reload_batch();
                                Ok(serde_json::to_string(&json!({ "status": "success", "message": format!("Batch path set to: {}", p.display()) })).unwrap())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    "boil_status" => {
                        match &state.batch {
                            Some(b) => commands::run_status(b, true),
                            None => Err(anyhow::anyhow!("No active batch")),
                        }
                    }
                    "boil_reset" => {
                        state.config.last_batch = None;
                        let _ = config::save_config(&state.config);
                        state.reload_batch();
                        Ok(serde_json::to_string(&json!({ "status": "success", "message": "Batch configuration reset." })).unwrap())
                    }
                    "boil_ls" => {
                        let layer = args.get("layer").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let path = args.get("path").and_then(|v| v.as_str()).map(|s| s.to_string());
                        match &state.batch {
                            Some(b) => commands::ls_show::run_ls(b, path, Some(layer), true),
                            None => Err(anyhow::anyhow!("No active batch")),
                        }
                    }
                    "boil_find" => {
                        let symbol = args.get("symbol").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        match &state.index {
                            Some(idx) => commands::find_expand::run_find_with_index(idx, &symbol, true),
                            None => Err(anyhow::anyhow!("No index loaded. Active batch might not be set.")),
                        }
                    }
                    "boil_read_file" => {
                        let layer = args.get("layer").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let file = args.get("file").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        match &state.batch {
                            Some(b) => commands::ls_show::run_show(b, file, Some(layer), true),
                            None => Err(anyhow::anyhow!("No active batch")),
                        }
                    }
                    "boil_read_symbol" => {
                        let layer = args.get("layer").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let symbol = args.get("symbol").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let id = args.get("id").and_then(|v| v.as_u64()).map(|i| i as usize);
                        match (&state.batch, &state.index, &state.symbol_map) {
                            (Some(b), Some(idx), Some(sym_map)) => {
                                commands::find_expand::run_expand_with_index(b, idx, sym_map, &symbol, Some(layer), id, true)
                            }
                            _ => Err(anyhow::anyhow!("No active batch or index not loaded")),
                        }
                    }
                    "boil_write" => {
                        let file = args.get("file").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let line = args.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        let content = args.get("content").and_then(|v| v.as_str()).map(|s| s.to_string());
                        match &state.batch {
                            Some(b) => {
                                match commands::edit::run_write(b, file, line, content) {
                                    Ok(_) => {
                                        state.reload_batch();
                                        Ok(serde_json::to_string(&json!({ "status": "success", "message": "File updated successfully." })).unwrap())
                                    }
                                    Err(e) => Err(e),
                                }
                            }
                            None => Err(anyhow::anyhow!("No active batch")),
                        }
                    }
                    "boil_delete" => {
                        let file = args.get("file").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let line = args.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        match &state.batch {
                            Some(b) => {
                                match commands::edit::run_delete(b, file, line) {
                                    Ok(_) => {
                                        state.reload_batch();
                                        Ok(serde_json::to_string(&json!({ "status": "success", "message": "Line deleted successfully." })).unwrap())
                                    }
                                    Err(e) => Err(e),
                                }
                            }
                            None => Err(anyhow::anyhow!("No active batch")),
                        }
                    }
                    _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
                };

                match output_result {
                    Ok(text) => {
                        let result = json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": text
                                }
                            ]
                        });
                        send_response(id, result);
                    }
                    Err(e) => {
                        let result = json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": format!("Error: {}", e)
                                }
                            ],
                            "isError": true
                        });
                        send_response(id, result);
                    }
                }
            }
        } else {
            if let Some(id) = req.id {
                send_error(Some(id), -32601, "Method not found".to_string());
            }
        }
    }

    Ok(())
}
