use anyhow::Result;
use boil::adapters::{
    build::CompositeBuildProvider,
    clustering::LeidenClusteringProvider,
    documentation::MarkdownProvider,
    embeddings::FastEmbedProvider,
    export::{DotExporter, GraphMlExporter, JsonExporter},
    input::{
        BuildModule, DocumentationModule, OwnershipModule, ProvenanceModule, RuntimeModule,
        SyntaxModule,
    },
    output::KnowledgeExportModule,
    ownership::CodeownersProvider,
    reasoning::{ArchitectureAnalyzer, SemanticsModule},
    runtime::JsonTraceProvider,
    syntax::TreeSitterProvider,
    temporal::GitTemporalProvider,
    vcs::GitProvider,
};
use boil::core::{Engine, EngineConfig};
use boil::ports::temporal::TemporalProvider;
use globset::GlobSetBuilder;
use std::fs;
use tempfile::TempDir;

fn build_test_engine() -> Engine {
    Engine::new()
        // 6 Input Modules
        .register_input(Box::new(SyntaxModule::new(Box::new(
            TreeSitterProvider::new(),
        ))))
        .register_input(Box::new(BuildModule::new(Box::new(
            CompositeBuildProvider::new(),
        ))))
        .register_input(Box::new(ProvenanceModule::new(Box::new(
            GitProvider::new(),
        ))))
        .register_input(Box::new(RuntimeModule::new(Box::new(JsonTraceProvider))))
        .register_input(Box::new(OwnershipModule::new(Box::new(
            CodeownersProvider,
        ))))
        .register_input(Box::new(DocumentationModule::new(Box::new(
            MarkdownProvider,
        ))))
        // 2 Reasoning Modules
        .register_reasoning(Box::new(ArchitectureAnalyzer::new(Box::new(
            LeidenClusteringProvider,
        ))))
        .register_reasoning(Box::new(SemanticsModule::new(Box::new(
            FastEmbedProvider::new(),
        ))))
        // 1 Output Module (with 3 format exporters)
        .register_output(Box::new(KnowledgeExportModule::new(vec![
            Box::new(JsonExporter),
            Box::new(DotExporter),
            Box::new(GraphMlExporter),
        ])))
}

#[test]
fn test_massive_comprehensive_pipeline() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    // 1. Create a rich Dummy Repository
    fs::create_dir_all(repo_path.join("src"))?;
    fs::create_dir_all(repo_path.join(".github"))?;
    fs::create_dir_all(repo_path.join("docs"))?;

    // Build: Cargo.toml
    fs::write(
        repo_path.join("Cargo.toml"),
        r#"[package]
name = "dummy"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
"#,
    )?;

    // Syntax: src/main.rs and src/utils.rs
    fs::write(
        repo_path.join("src/main.rs"),
        r#"
mod utils;
fn main() {
    let sum = utils::add(5, 10);
    println!("Sum: {}", sum);
}
"#,
    )?;

    fs::write(
        repo_path.join("src/utils.rs"),
        r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )?;

    // Documentation: README and ADR
    fs::write(
        repo_path.join("README.md"),
        "# Dummy Project\nThis is a massive test.",
    )?;
    fs::write(
        repo_path.join("docs/ADR-1-architecture.md"),
        "# ADR 1\nWe use Rust.",
    )?;

    // Ownership: CODEOWNERS
    fs::write(
        repo_path.join(".github/CODEOWNERS"),
        "* @backend-team\n/docs/ @docs-team\n",
    )?;

    // Runtime: trace.json
    fs::write(
        repo_path.join("trace.json"),
        r#"[
    { "file": "src/utils.rs", "symbol": "add", "count": 42 }
]"#,
    )?;

    // 2. Initialize Git and create history for Provenance and Temporal diffing
    let repo = git2::Repository::init(repo_path)?;
    let mut index = repo.index()?;

    // Commit 1 (Initial setup)
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let oid1 = index.write_tree()?;
    let sig1 = git2::Signature::now("Alice", "alice@example.com")?;
    let tree1 = repo.find_tree(oid1)?;
    let commit1 = repo.commit(Some("HEAD"), &sig1, &sig1, "Initial commit", &tree1, &[])?;

    // Commit 2 (Modify a file to create blame history and test Temporal)
    fs::write(
        repo_path.join("src/utils.rs"),
        r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}
"#,
    )?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let oid2 = index.write_tree()?;
    let sig2 = git2::Signature::now("Bob", "bob@example.com")?;
    let tree2 = repo.find_tree(oid2)?;
    let parent_commit1 = repo.find_commit(commit1)?;
    let _commit2 = repo.commit(
        Some("HEAD"),
        &sig2,
        &sig2,
        "Added subtract function",
        &tree2,
        &[&parent_commit1],
    )?;

    // 3. Configure and Run Engine on HEAD
    let config = EngineConfig {
        ignore: GlobSetBuilder::new().build()?,
        ignore_patterns: vec![],
        force_timestamp: Some("massive_test".to_string()),
        silent: true,
    };
    let engine = build_test_engine();
    let output_dir = temp_dir.path().join("output");
    fs::create_dir_all(&output_dir)?;

    let (_, graph) = engine.run(repo_path, &output_dir, &config)?;

    // 4. Verify Output Formats
    let boil_out = output_dir.join("canon_massive_test");
    assert!(boil_out.join("canon.json").exists(), "JSON should exist");
    assert!(boil_out.join("canon.dot").exists(), "DOT should exist");
    assert!(boil_out.join("canon.graphml").exists(), "GraphML should exist");

    // 5. Exhaustively Assert Canon Graph in Memory
    println!("--- GRAPH NODES ---");
    for k in graph.node_index.keys() {
        println!("{}", k);
    }
    println!("-------------------");

    let repo_str = repo_path.to_string_lossy();
    let main_sym = format!("symbol:{}/src/main.rs:main", repo_str);
    let add_sym = format!("symbol:{}/src/utils.rs:add", repo_str);
    let sub_sym = format!("symbol:{}/src/utils.rs:subtract", repo_str);
    let utils_file = format!("file:{}/src/utils.rs", repo_str);
    let readme_doc = format!("document:{}/README.md", repo_str);
    let adr_doc = format!("document:{}/docs/ADR-1-architecture.md", repo_str);

    // Syntax asserts
    assert!(graph.node_index.contains_key(&main_sym), "main fn missing");
    let add_fn_idx = *graph.node_index.get(&add_sym).expect("add fn missing");
    assert!(graph.node_index.contains_key(&sub_sym), "subtract fn missing");

    // Build (dependencies)
    assert!(graph.node_index.contains_key("package:serde"), "serde package missing");

    // Documentation
    assert!(graph.node_index.contains_key(&readme_doc), "README missing");
    assert!(graph.node_index.contains_key(&adr_doc), "ADR missing");

    // Detailed Node Field Assertions
    let utils_file_idx = *graph.node_index.get(&utils_file).expect("utils.rs missing");

    let add_node = &graph.graph[add_fn_idx];
    let utils_file_node = &graph.graph[utils_file_idx];

    let add_metadata = match add_node {
        boil::core::canon::graph::NodeData::Symbol { metadata, .. } => metadata,
        _ => panic!("Expected Symbol node"),
    };

    let file_metadata = match utils_file_node {
        boil::core::canon::graph::NodeData::File { metadata, .. } => metadata,
        _ => panic!("Expected File node"),
    };

    // Semantics (Embeddings)
    assert!(add_metadata.contains_key("embedding"), "Node lacks embedding");
    let embed_str = add_metadata.get("embedding").unwrap();
    let embed_vec: Vec<f32> = serde_json::from_str(embed_str).unwrap();
    assert_eq!(embed_vec.len(), 384, "FastEmbed should produce 384-dimensional vectors");

    // Architecture (Clustering)
    assert!(
        graph.node_index.keys().any(|k| k.starts_with("subsystem:")),
        "No subsystem nodes created by Leiden"
    );

    // Runtime Trace
    // In our dummy `trace.json`, `src/utils.rs:add` was executed 42 times.
    assert_eq!(add_metadata.get("runtime_executions").unwrap(), "42", "Runtime trace not mapped");

    // Ownership
    // `src/utils.rs` matches `* @backend-team`
    assert!(graph.node_index.contains_key("owner:@backend-team"), "owner node missing");

    // Provenance (Git blame/log)
    assert!(file_metadata.contains_key("created"));
    assert!(file_metadata.contains_key("modified"));
    
    // Since Alice and Bob both touched utils.rs, both should be authors.
    assert!(graph.node_index.contains_key("author:alice@example.com"), "Alice missing");
    assert!(graph.node_index.contains_key("author:bob@example.com"), "Bob missing");

    // 6. Test Temporal Diffing Engine
    // Diff HEAD~1 (before subtract was added) with HEAD (current)
    let temporal_provider = GitTemporalProvider::new();
    let base_graph = temporal_provider.build_graph_from_commit(&engine, &config, repo_path, "HEAD~1")?;
    let head_graph = temporal_provider.build_graph_from_commit(&engine, &config, repo_path, "HEAD")?;

    let diff_report = temporal_provider.compare_graphs(&base_graph, &head_graph)?;
    
    // We added `subtract` in HEAD, so it should be in added_symbols
    assert!(
        diff_report.added_symbols.contains(&"subtract".to_string()),
        "Temporal diff should detect 'subtract' as added"
    );

    Ok(())
}
