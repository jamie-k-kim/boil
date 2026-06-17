use anyhow::Result;
use clap::Parser as ClapParser;
use std::path::PathBuf;

mod plugin_generator;

#[derive(ClapParser, Debug)]
#[command(
    name = "boil",
    version = "0.1.0",
    arg_required_else_help = true,
    disable_version_flag = true,
    override_usage = "boil [REPO_PATH] [OUTPUT_PATH] [OPTIONS]\n       boil diff <ARG1> <ARG2> [REPO_PATH] [OPTIONS]"
)]
struct Cli {
    /// Initialize the global boil configuration file (boil.toml) in the home directory
    #[arg(short = 'i', long = "init")]
    init: bool,

    /// Path to the repository to analyze
    #[arg(value_name = "REPO_PATH")]
    input_dir: Option<String>,

    /// Path to the output directory
    #[arg(value_name = "OUTPUT_PATH")]
    output_dir: Option<String>,

    /// Profile to use from the configuration file
    #[arg(short = 'p', long = "profile")]
    profile: Option<String>,

    /// Run in silent mode (suppress console output)
    #[arg(short = 's', long = "silent")]
    silent: bool,

    /// Print version
    #[arg(short = 'v', long = "version")]
    version: bool,

    #[command(subcommand)]
    subcommand: Option<Subcommands>,
}

#[derive(clap::Subcommand, Debug, Clone)]
enum Subcommands {
    /// Diff two versioned canons or git commits
    Diff {
        /// First target (file path or git commit/branch)
        arg1: String,
        /// Second target (file path or git commit/branch)
        arg2: String,
        /// Path to repository (required only for commit diff if not in current directory)
        repo_path: Option<String>,
    },
    /// Scaffold a new WebAssembly plugin
    CreatePlugin {
        /// Name of the plugin (e.g., "my-plugin")
        name: String,
        /// Module type to target (e.g., "temporal", "ownership", "syntax", "embeddings", "clustering", "documentation", "build", "runtime", "provenance")
        #[arg(short = 'm', long = "module")]
        module: String,
    },
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RawProfile {
    pub ignore: Option<Vec<String>>,
    pub silent: Option<bool>,
    pub modules: Option<std::collections::HashMap<String, toml::Value>>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.version {
        println!("boil {}", env!("CARGO_PKG_VERSION"));
        Ok(())
    } else if cli.init {
        run_init()
    } else if let Some(sub) = cli.subcommand {
        match sub {
            Subcommands::Diff {
                arg1,
                arg2,
                repo_path,
            } => run_diff(arg1, arg2, repo_path, cli.profile),
            Subcommands::CreatePlugin { name, module } => {
                plugin_generator::run_create_plugin(&name, &module)
            }
        }
    } else {
        let input_dir = cli.input_dir.ok_or_else(|| anyhow::anyhow!(
            "error: The following required arguments were not provided:\n  <REPO_PATH>\n  <OUTPUT_PATH>\n\nUsage: boil [REPO_PATH] [OUTPUT_PATH]"
        ))?;
        let output_dir = cli.output_dir.ok_or_else(|| anyhow::anyhow!(
            "error: The following required arguments were not provided:\n  <OUTPUT_PATH>\n\nUsage: boil [REPO_PATH] [OUTPUT_PATH]"
        ))?;
        run_canon_export(input_dir, output_dir, cli.profile, cli.silent)
    }
}

fn run_init() -> Result<()> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let boil_toml_path = home_dir.join("boil.toml");

    if boil_toml_path.exists() {
        println!(
            "{} boil.toml already exists at {:?}",
            console::style("Notice:").yellow(),
            boil_toml_path
        );
        return Ok(());
    }

    let template = r#"[default]
ignore = ["**/target/**", "**/node_modules/**", "**/.git/**"]

[default.modules.semantics]
provider = "openai"

[default.modules.export]
formats = ["json", "dot"]
"#;

    std::fs::write(&boil_toml_path, template)?;
    println!(
        "{} Created global boil.toml template at {:?}",
        console::style("Success!").green().bold(),
        boil_toml_path
    );

    Ok(())
}

pub(crate) fn load_profile(
    _input_path: &PathBuf,
    profile_name: &Option<String>,
) -> Result<Option<RawProfile>> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    // Check for both ~/boil.toml and ~/.boil.toml
    let mut profile_file = home_dir.join("boil.toml");
    if !profile_file.exists() {
        let hidden_file = home_dir.join(".boil.toml");
        if hidden_file.exists() {
            profile_file = hidden_file;
        }
    }

    if profile_file.exists() {
        let content = std::fs::read_to_string(&profile_file)?;
        let profiles: std::collections::HashMap<String, RawProfile> = toml::from_str(&content)
            .map_err(|e| {
                anyhow::anyhow!("Failed to parse boil.toml at {:?}: {}", profile_file, e)
            })?;

        if let Some(name) = profile_name {
            return Ok(Some(
                profiles
                    .get(name)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Profile '{}' not found in boil.toml at {:?}",
                            name,
                            profile_file
                        )
                    })?
                    .clone(),
            ));
        }

        if let Some(default_profile) = profiles.get("default") {
            return Ok(Some(default_profile.clone()));
        }

        Ok(None)
    } else if profile_name.is_some() {
        Err(anyhow::anyhow!(
            "Profile requested but boil.toml not found in your home directory"
        ))
    } else {
        Ok(None)
    }
}

pub(crate) fn resolve_vcs_provider(
    profile_data: &Option<RawProfile>,
) -> Box<dyn boil::ports::VcsProvider> {
    if let Some(modules) = profile_data.as_ref().and_then(|p| p.modules.as_ref()) {
        if let Some(prov_config) = modules.get("provenance") {
            let provider_name = prov_config
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("git2");
            match provider_name {
                "git2" => Box::new(boil::adapters::vcs::GitProvider::new()),
                "mercurial" => Box::new(boil::adapters::vcs::mercurial::MercurialProvider::new()),
                "wasm" => {
                    let plugin_path = prov_config
                        .get("plugin_path")
                        .and_then(|v| v.as_str())
                        .expect("plugin_path must be specified when using the Wasm provider");
                    Box::new(boil::adapters::wasm::vcs::WasmVcsProvider::new(plugin_path).unwrap())
                }
                "none" => Box::new(boil::adapters::null::NullVcsProvider),
                _ => Box::new(boil::adapters::vcs::GitProvider::new()), // Default
            }
        } else {
            Box::new(boil::adapters::vcs::GitProvider::new())
        }
    } else {
        Box::new(boil::adapters::vcs::GitProvider::new())
    }
}

pub(crate) fn resolve_syntax_provider(
    profile_data: &Option<RawProfile>,
) -> Box<dyn boil::ports::SyntaxProvider> {
    if let Some(modules) = profile_data.as_ref().and_then(|p| p.modules.as_ref()) {
        if let Some(syntax_config) = modules.get("syntax") {
            let provider_name = syntax_config
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("treesitter");
            match provider_name {
                "treesitter" => Box::new(boil::adapters::syntax::TreeSitterProvider::new()),
                "wasm" => {
                    let plugin_path = syntax_config
                        .get("plugin_path")
                        .and_then(|v| v.as_str())
                        .expect("plugin_path must be specified when using the Wasm provider");
                    Box::new(
                        boil::adapters::wasm::syntax::WasmSyntaxProvider::new(plugin_path).unwrap(),
                    )
                }
                "none" => Box::new(boil::adapters::null::NullSyntaxProvider),
                _ => Box::new(boil::adapters::syntax::TreeSitterProvider::new()), // Default
            }
        } else {
            Box::new(boil::adapters::syntax::TreeSitterProvider::new())
        }
    } else {
        Box::new(boil::adapters::syntax::TreeSitterProvider::new())
    }
}

pub(crate) fn resolve_build_provider(
    profile_data: &Option<RawProfile>,
) -> Box<dyn boil::ports::BuildProvider> {
    if let Some(modules) = profile_data.as_ref().and_then(|p| p.modules.as_ref()) {
        if let Some(build_config) = modules.get("build") {
            let provider_name = build_config
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("composite");
            match provider_name {
                "cargo" => Box::new(boil::adapters::build::cargo::CargoProvider),
                "npm" => Box::new(boil::adapters::build::npm::NpmProvider),
                "python" => Box::new(boil::adapters::build::python::PythonProvider),
                "bazel" => Box::new(boil::adapters::build::bazel::BazelProvider),
                "gradle" => Box::new(boil::adapters::build::gradle::GradleProvider),
                "wasm" => {
                    let plugin_path = build_config
                        .get("plugin_path")
                        .and_then(|v| v.as_str())
                        .expect("plugin_path must be specified when using the Wasm provider");
                    Box::new(
                        boil::adapters::wasm::build::WasmBuildProvider::new(plugin_path).unwrap(),
                    )
                }
                "none" => Box::new(boil::adapters::null::NullBuildProvider),
                _ => Box::new(boil::adapters::build::CompositeBuildProvider::new()), // Default
            }
        } else {
            Box::new(boil::adapters::build::CompositeBuildProvider::new())
        }
    } else {
        Box::new(boil::adapters::build::CompositeBuildProvider::new())
    }
}

pub(crate) fn resolve_runtime_provider(
    profile_data: &Option<RawProfile>,
) -> Box<dyn boil::ports::RuntimeProvider> {
    if let Some(modules) = profile_data.as_ref().and_then(|p| p.modules.as_ref()) {
        if let Some(runtime_config) = modules.get("runtime") {
            let provider_name = runtime_config
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("json");
            match provider_name {
                "json" => Box::new(boil::adapters::runtime::JsonTraceProvider),
                "opentelemetry" => {
                    Box::new(boil::adapters::runtime::opentelemetry::OpentelemetryProvider::new())
                }
                "wasm" => {
                    let plugin_path = runtime_config
                        .get("plugin_path")
                        .and_then(|v| v.as_str())
                        .expect("plugin_path must be specified when using the Wasm provider");
                    Box::new(
                        boil::adapters::wasm::runtime::WasmRuntimeProvider::new(plugin_path)
                            .unwrap(),
                    )
                }
                "none" => Box::new(boil::adapters::null::NullRuntimeProvider),
                _ => Box::new(boil::adapters::runtime::JsonTraceProvider), // Default
            }
        } else {
            Box::new(boil::adapters::runtime::JsonTraceProvider)
        }
    } else {
        Box::new(boil::adapters::runtime::JsonTraceProvider)
    }
}

pub(crate) fn resolve_ownership_provider(
    profile_data: &Option<RawProfile>,
) -> Box<dyn boil::ports::OwnershipProvider> {
    if let Some(modules) = profile_data.as_ref().and_then(|p| p.modules.as_ref()) {
        if let Some(own_config) = modules.get("ownership") {
            let provider_name = own_config
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("codeowners");
            match provider_name {
                "codeowners" => Box::new(boil::adapters::ownership::CodeownersProvider),
                "jira" => Box::new(boil::adapters::ownership::jira::JiraProvider::new()),
                "github_teams" => {
                    Box::new(boil::adapters::ownership::github_teams::GithubTeamsProvider::new())
                }
                "wasm" => {
                    let plugin_path = own_config
                        .get("plugin_path")
                        .and_then(|v| v.as_str())
                        .expect("plugin_path must be specified when using the Wasm provider");
                    Box::new(
                        boil::adapters::wasm::ownership::WasmOwnershipProvider::new(plugin_path)
                            .unwrap(),
                    )
                }
                "none" => Box::new(boil::adapters::null::NullOwnershipProvider),
                _ => Box::new(boil::adapters::ownership::CodeownersProvider), // Default
            }
        } else {
            Box::new(boil::adapters::ownership::CodeownersProvider)
        }
    } else {
        Box::new(boil::adapters::ownership::CodeownersProvider)
    }
}

pub(crate) fn resolve_documentation_provider(
    profile_data: &Option<RawProfile>,
) -> Box<dyn boil::ports::DocumentationProvider> {
    if let Some(modules) = profile_data.as_ref().and_then(|p| p.modules.as_ref()) {
        if let Some(doc_config) = modules.get("documentation") {
            let provider_name = doc_config
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("markdown");
            match provider_name {
                "markdown" => Box::new(boil::adapters::documentation::MarkdownProvider),
                "notion" => Box::new(boil::adapters::documentation::notion::NotionProvider::new()),
                "confluence" => {
                    Box::new(boil::adapters::documentation::confluence::ConfluenceProvider::new())
                }
                "wasm" => {
                    let plugin_path = doc_config
                        .get("plugin_path")
                        .and_then(|v| v.as_str())
                        .expect("plugin_path must be specified when using the Wasm provider");
                    Box::new(
                        boil::adapters::wasm::documentation::WasmDocumentationProvider::new(
                            plugin_path,
                        )
                        .unwrap(),
                    )
                }
                "none" => Box::new(boil::adapters::null::NullDocumentationProvider),
                _ => Box::new(boil::adapters::documentation::MarkdownProvider), // Default
            }
        } else {
            Box::new(boil::adapters::documentation::MarkdownProvider)
        }
    } else {
        Box::new(boil::adapters::documentation::MarkdownProvider)
    }
}

pub(crate) fn resolve_clustering_provider(
    profile_data: &Option<RawProfile>,
) -> Box<dyn boil::ports::ClusteringProvider> {
    if let Some(modules) = profile_data.as_ref().and_then(|p| p.modules.as_ref()) {
        if let Some(arch_config) = modules.get("architecture") {
            let provider_name = arch_config
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("leiden");
            match provider_name {
                "leiden" => Box::new(boil::adapters::clustering::LeidenClusteringProvider),
                "wasm" => {
                    let plugin_path = arch_config
                        .get("plugin_path")
                        .and_then(|v| v.as_str())
                        .expect("plugin_path must be specified when using the Wasm provider");
                    Box::new(
                        boil::adapters::wasm::clustering::WasmClusteringProvider::new(plugin_path)
                            .unwrap(),
                    )
                }
                "none" => Box::new(boil::adapters::null::NullClusteringProvider),
                _ => Box::new(boil::adapters::clustering::LeidenClusteringProvider), // Default
            }
        } else {
            Box::new(boil::adapters::clustering::LeidenClusteringProvider)
        }
    } else {
        Box::new(boil::adapters::clustering::LeidenClusteringProvider)
    }
}

pub(crate) fn resolve_embedding_provider(
    profile_data: &Option<RawProfile>,
) -> Box<dyn boil::ports::EmbeddingProvider> {
    if let Some(modules) = profile_data.as_ref().and_then(|p| p.modules.as_ref()) {
        if let Some(sem_config) = modules.get("semantics") {
            let provider_name = sem_config
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("fastembed");
            match provider_name {
                "fastembed" => Box::new(boil::adapters::embeddings::FastEmbedProvider::new()),
                "openai" => Box::new(boil::adapters::embeddings::openai::OpenAiProvider::new()),
                "cohere" => Box::new(boil::adapters::embeddings::cohere::CohereProvider::new()),
                "voyageai" => {
                    Box::new(boil::adapters::embeddings::voyageai::VoyageAiProvider::new())
                }
                "ollama" => Box::new(boil::adapters::embeddings::ollama::OllamaProvider::new()),
                "wasm" => {
                    let plugin_path = sem_config
                        .get("plugin_path")
                        .and_then(|v| v.as_str())
                        .expect("plugin_path must be specified when using the Wasm provider");
                    Box::new(
                        boil::adapters::wasm::embeddings::WasmEmbeddingProvider::new(plugin_path)
                            .unwrap(),
                    )
                }
                "none" => Box::new(boil::adapters::null::NullEmbeddingProvider),
                _ => Box::new(boil::adapters::embeddings::FastEmbedProvider::new()), // Default
            }
        } else {
            Box::new(boil::adapters::embeddings::FastEmbedProvider::new())
        }
    } else {
        Box::new(boil::adapters::embeddings::FastEmbedProvider::new())
    }
}

pub(crate) fn resolve_exporters(
    profile_data: &Option<RawProfile>,
) -> Vec<Box<dyn boil::ports::ExportProvider>> {
    let mut exporters: Vec<Box<dyn boil::ports::ExportProvider>> = Vec::new();

    if let Some(formats) = profile_data
        .as_ref()
        .and_then(|p| p.modules.as_ref())
        .and_then(|m| m.get("export"))
        .and_then(|e| e.get("formats"))
        .and_then(|v| v.as_array())
    {
        for format in formats {
            if let Some(f_str) = format.as_str() {
                match f_str {
                    "json" => exporters.push(Box::new(boil::adapters::export::JsonExporter)),
                    "dot" => exporters.push(Box::new(boil::adapters::export::DotExporter)),
                    "graphml" => exporters.push(Box::new(boil::adapters::export::GraphMlExporter)),
                    "neo4j" => exporters
                        .push(Box::new(boil::adapters::export::neo4j::Neo4jProvider::new())),
                    _ => {}
                }
            }
        }
    }

    if exporters.is_empty() {
        exporters.push(Box::new(boil::adapters::export::JsonExporter));
    }

    exporters
}

pub(crate) fn resolve_temporal_provider(
    profile_data: &Option<RawProfile>,
) -> Box<dyn boil::ports::temporal::TemporalProvider> {
    if let Some(modules) = profile_data.as_ref().and_then(|p| p.modules.as_ref()) {
        if let Some(temp_config) = modules.get("temporal") {
            let provider_name = temp_config
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("git");
            match provider_name {
                "git" => Box::new(boil::adapters::temporal::GitTemporalProvider::new()),
                "wasm" => {
                    let plugin_path = temp_config
                        .get("plugin_path")
                        .and_then(|v| v.as_str())
                        .expect("plugin_path must be specified when using the Wasm provider");
                    Box::new(
                        boil::adapters::wasm::temporal::WasmTemporalProvider::new(plugin_path)
                            .unwrap(),
                    )
                }
                "none" => Box::new(boil::adapters::null::NullTemporalProvider),
                _ => Box::new(boil::adapters::temporal::GitTemporalProvider::new()), // Default
            }
        } else {
            Box::new(boil::adapters::temporal::GitTemporalProvider::new())
        }
    } else {
        Box::new(boil::adapters::temporal::GitTemporalProvider::new())
    }
}

pub(crate) fn build_engine(profile_data: &Option<RawProfile>) -> boil::core::Engine {
    let vcs_provider = resolve_vcs_provider(profile_data);
    let syntax_provider = resolve_syntax_provider(profile_data);
    let build_provider = resolve_build_provider(profile_data);
    let runtime_provider = resolve_runtime_provider(profile_data);
    let ownership_provider = resolve_ownership_provider(profile_data);
    let documentation_provider = resolve_documentation_provider(profile_data);
    let clustering_provider = resolve_clustering_provider(profile_data);
    let embedding_provider = resolve_embedding_provider(profile_data);
    let exporters = resolve_exporters(profile_data);

    boil::core::Engine::new()
        .register_input(Box::new(boil::adapters::input::SyntaxModule::new(
            syntax_provider,
        )))
        .register_input(Box::new(boil::adapters::input::BuildModule::new(
            build_provider,
        )))
        .register_input(Box::new(boil::adapters::input::ProvenanceModule::new(
            vcs_provider,
        )))
        .register_input(Box::new(boil::adapters::input::RuntimeModule::new(
            runtime_provider,
        )))
        .register_input(Box::new(boil::adapters::input::OwnershipModule::new(
            ownership_provider,
        )))
        .register_input(Box::new(boil::adapters::input::DocumentationModule::new(
            documentation_provider,
        )))
        .register_reasoning(Box::new(
            boil::adapters::reasoning::ArchitectureAnalyzer::new(clustering_provider),
        ))
        .register_reasoning(Box::new(boil::adapters::reasoning::SemanticsModule::new(
            embedding_provider,
        )))
        .register_output(Box::new(
            boil::adapters::output::KnowledgeExportModule::new(exporters),
        ))
}

pub(crate) fn build_config(
    profile_data: &Option<RawProfile>,
    silent: bool,
) -> Result<boil::core::EngineConfig> {
    let is_silent = silent
        || profile_data
            .as_ref()
            .and_then(|p| p.silent)
            .unwrap_or(false);
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();

    let mut ignore_patterns = profile_data
        .as_ref()
        .and_then(|p| p.ignore.clone())
        .unwrap_or_default();
    ignore_patterns.push("boil.toml".to_string());

    Ok(boil::core::EngineConfig {
        ignore: build_globset(&ignore_patterns)?,
        ignore_patterns,
        force_timestamp: Some(timestamp),
        silent: is_silent,
    })
}

fn run_canon_export(
    input_dir: String,
    output_dir: String,
    profile: Option<String>,
    silent: bool,
) -> Result<()> {
    let input_path = PathBuf::from(&input_dir);
    let output_path = PathBuf::from(&output_dir);
    let profile_data = load_profile(&input_path, &profile)?;
    let config = build_config(&profile_data, silent)?;

    if !config.silent {
        println!(
            "{} Generating Canon Graph...",
            console::style("--- Canon Export ---").bold().cyan()
        );
    }

    let engine = build_engine(&profile_data);
    engine.run(&input_path, &output_path, &config)?;

    if !config.silent {
        println!(
            "\n{} Canon export complete at {:?}",
            console::style("Success!").green().bold(),
            output_path
        );
    }

    Ok(())
}

fn build_globset(patterns: &[String]) -> Result<globset::GlobSet> {
    let mut builder = globset::GlobSetBuilder::new();

    for p in patterns {
        let glob = globset::Glob::new(p)?;
        builder.add(glob);
    }

    Ok(builder.build()?)
}

fn run_diff(
    arg1: String,
    arg2: String,
    repo_path: Option<String>,
    profile: Option<String>,
) -> Result<()> {
    let path1 = PathBuf::from(&arg1);
    let path2 = PathBuf::from(&arg2);

    let repo_dir = repo_path
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let profile_data = load_profile(&repo_dir, &profile).unwrap_or(None);
    let provider = resolve_temporal_provider(&profile_data);
    let engine = build_engine(&profile_data);
    let config = build_config(&profile_data, true)?;

    let diff_report = if path1.is_file() && path2.is_file() {
        // Pattern A: File Diff (Load pre-built CanonState)
        println!(
            "{} Comparing binary canons: {:?} vs {:?}",
            console::style("--- Diffing Files ---").bold().cyan(),
            path1,
            path2
        );
        let base_state = boil::core::canon::CanonState::load(&path1)?;
        let head_state = boil::core::canon::CanonState::load(&path2)?;
        provider.compare_graphs(&base_state.graph, &head_state.graph)?
    } else {
        // Pattern B: Git Commit Diff
        println!(
            "{} Comparing Git revisions: {} vs {} in {:?}",
            console::style("--- Diffing Git Revisions ---")
                .bold()
                .cyan(),
            arg1,
            arg2,
            repo_dir
        );
        let base_graph = provider.build_graph_from_commit(&engine, &config, &repo_dir, &arg1)?;
        let head_graph = provider.build_graph_from_commit(&engine, &config, &repo_dir, &arg2)?;
        provider.compare_graphs(&base_graph, &head_graph)?
    };

    print_diff_report(&diff_report);

    Ok(())
}

fn print_diff_report(report: &boil::ports::temporal::DiffReport) {
    println!(
        "\n{}",
        console::style("=== Diff Report ===")
            .bold()
            .underlined()
            .yellow()
    );

    // Subsystems
    if !report.added_subsystems.is_empty() || !report.removed_subsystems.is_empty() {
        println!("\n{}", console::style("[Subsystems]").bold().green());
        for sub in &report.added_subsystems {
            println!("  {} {}", console::style("+ Added:").green(), sub);
        }
        for sub in &report.removed_subsystems {
            println!("  {} {}", console::style("- Removed:").red(), sub);
        }
    } else {
        println!("\n[Subsystems] No changes.");
    }

    // Symbols
    if !report.added_symbols.is_empty()
        || !report.removed_symbols.is_empty()
        || !report.moved_symbols.is_empty()
    {
        println!("\n{}", console::style("[Symbols]").bold().green());
        if !report.added_symbols.is_empty() {
            println!("  Added Symbols ({}):", report.added_symbols.len());
            for sym in &report.added_symbols {
                println!("    - {}", sym);
            }
        }
        if !report.removed_symbols.is_empty() {
            println!("  Removed Symbols ({}):", report.removed_symbols.len());
            for sym in &report.removed_symbols {
                println!("    - {}", sym);
            }
        }
        if !report.moved_symbols.is_empty() {
            println!("  Moved Symbols ({}):", report.moved_symbols.len());
            for (name, path_diff) in &report.moved_symbols {
                println!("    - {} ({})", name, path_diff);
            }
        }
    } else {
        println!("\n[Symbols] No changes.");
    }

    // Topology
    println!("\n{}", console::style("[Topology]").bold().green());
    if report.new_edges > 0 {
        println!(
            "  • {} new dependency edges added to the graph",
            report.new_edges
        );
    } else {
        println!("  • No new dependency edges added");
    }
    println!();
}
