use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod config;
mod batch;
mod commands;
mod mcp;

#[derive(Parser)]
#[command(name = "boil", version = "0.1.0")]
struct Cli {
    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Generate a binary canon (.bin) from a repository")]
    Canon {
        #[arg(help = "The source code repository to index")]
        repo: PathBuf,
        #[arg(help = "The destination path where the output index will be saved")]
        output: PathBuf,
    },
    #[command(about = "Start the native MCP JSON-RPC server on stdin/stdout")]
    Mcp,
    #[command(about = "Latch onto a specific batch for Steampipe to use")]
    Setbatch { path: PathBuf },
    #[command(about = "Show the status of the currently active batch")]
    Status,
    #[command(about = "Reset your active batch configuration")]
    Reset,
    #[command(about = "List files and directories inside a specific layer")]
    Ls { layer: String, path: Option<String> },
    #[command(about = "Search for a specific symbol across the batch")]
    Find { symbol: String },
    #[command(about = "Write or overwrite code at a specific line in a file")]
    Write { 
        file: String, 
        line: usize, 
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        content: Vec<String> 
    },
    #[command(about = "Delete a specific line of code from a file")]
    Delete { file: String, line: usize },
    #[command(about = "Read the contents of a file or symbol")]
    Read {
        #[command(subcommand)]
        sub: ReadSub,
    },
    #[command(about = "Distill a repository into a single focused distillation")]
    Distill {
        #[arg(help = "The source code repository to distill")]
        repo: PathBuf,
        #[arg(help = "The destination path where the output will go")]
        output: PathBuf,
        #[arg(help = "Optional pre-generated binary canon (canon.bin) from Boil. If omitted, the repository will be indexed on-the-fly.")]
        canon: Option<PathBuf>,
        #[arg(short, long, help = "Optional list of files to focus on during distillation")]
        focus: Option<Vec<String>>,
        #[arg(short = 't', long, help = "Optional compression target (e.g., 50% or 1000b)")]
        target: Option<String>,
        #[arg(short = 'e', long, default_value = "none", help = "Embedding provider to use for canon patching (none, openai, ollama, fastembed)")]
        embedding_provider: String,
    },
    #[command(about = "Generate a batch of distilled fidelity layers (L0, L1, L2)")]
    Batch {
        #[arg(help = "The source code repository to distill")]
        repo: PathBuf,
        #[arg(help = "The destination path where the output will go")]
        output: PathBuf,
        #[arg(help = "Optional pre-generated binary canon (canon.bin) from Boil. If omitted, the repository will be indexed on-the-fly.")]
        canon: Option<PathBuf>,
        #[arg(short, long, help = "Optional list of files to focus on during distillation")]
        focus: Option<Vec<String>>,
        #[arg(short = 't', long, help = "Optional compression target (e.g., 50% or 1000b)")]
        target: Option<String>,
        #[arg(short = 'e', long, default_value = "none", help = "Embedding provider to use for canon patching (none, openai, ollama, fastembed)")]
        embedding_provider: String,
    },
}

#[derive(Subcommand)]
enum ReadSub {
    File {
        layer: String,
        file: String,
    },
    Symbol {
        layer: String,
        symbol: String,
        #[arg(long)]
        id: Option<usize>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    let mut config = config::load_config()?;

    // 1. Handle commands that don't need a batch
    match &cli.command {
        Commands::Canon { repo, output } => {
            boil_engine::indexer::run_canon_export(
                repo.to_string_lossy().to_string(),
                output.to_string_lossy().to_string(),
                None,
                false,
            )?;
            return Ok(());
        }
        Commands::Mcp => {
            return mcp::run_mcp_server();
        }
        Commands::Setbatch { path } => {
            batch::Batch::load(path.clone())?;
            config.last_batch = Some(path.clone());
            config::save_config(&config)?;
            println!("Batch path set to: {}", path.display());
            return Ok(());
        }
        Commands::Reset => {
            config.last_batch = None;
            config::save_config(&config)?;
            println!("Batch configuration reset.");
            return Ok(());
        }
        Commands::Distill { repo, output, canon, focus, target, embedding_provider } => {
            commands::distill::run_distill(
                repo.clone(),
                output.clone(),
                canon.clone(),
                focus.clone(),
                target.clone(),
                embedding_provider.clone(),
            )?;
            return Ok(());
        }
        Commands::Batch { repo, output, canon, focus, target, embedding_provider } => {
            commands::batch::run_batch(
                repo.clone(),
                output.clone(),
                canon.clone(),
                focus.clone(),
                target.clone(),
                embedding_provider.clone(),
            )?;
            return Ok(());
        }
        _ => {}
    }

    // 2. Ensure Batch is set for other commands
    let batch_path = config.last_batch.ok_or_else(|| anyhow::anyhow!("No batch path set. Use boil setbatch <PATH>."))?;
    let batch = batch::Batch::load(batch_path)?;

    // 3. Dispatch remaining commands
    match cli.command {
        Commands::Status => {
            let out = commands::run_status(&batch, cli.json)?;
            print!("{}", out);
        }
        Commands::Ls { layer, path } => {
            let out = commands::ls_show::run_ls(&batch, path, Some(layer), cli.json)?;
            print!("{}", out);
        }
        Commands::Find { symbol } => {
            let out = commands::find_expand::run_find(&batch, symbol, cli.json)?;
            print!("{}", out);
        }
        Commands::Write { file, line, content } => {
            let content_str = if content.is_empty() { None } else { Some(content.join(" ")) };
            commands::edit::run_write(&batch, file, line, content_str)?;
            if cli.json {
                println!("{}", serde_json::to_string(&serde_json::json!({ "status": "success", "message": "File updated successfully." }))?);
            } else {
                println!("File updated successfully.");
            }
        }
        Commands::Delete { file, line } => {
            commands::edit::run_delete(&batch, file, line)?;
            if cli.json {
                println!("{}", serde_json::to_string(&serde_json::json!({ "status": "success", "message": "Line deleted successfully." }))?);
            } else {
                println!("Line deleted successfully.");
            }
        }
        Commands::Read { sub } => match sub {
            ReadSub::File { layer, file } => {
                let out = commands::ls_show::run_show(&batch, file, Some(layer), cli.json)?;
                print!("{}", out);
            }
            ReadSub::Symbol { layer, symbol, id } => {
                let out = commands::find_expand::run_expand(&batch, symbol, Some(layer), id, cli.json)?;
                print!("{}", out);
            }
        },
        Commands::Canon { .. } => unreachable!(),
        Commands::Distill { .. } => unreachable!(),
        Commands::Batch { .. } => unreachable!(),
        _ => unreachable!(),
    }

    Ok(())
}
