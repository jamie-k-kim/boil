use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::Result;
use boil_core::canon::{FileInfo, ProjectGraph, Symbol, SymbolKind, state::CanonStateRef};
use boil_core::canon::graph::{NodeData, EdgeData};
use boil_core::language::Language;

fn create_dummy_canon(repo_root: &Path, canon_path: &Path) -> Result<()> {
    let mut graph = ProjectGraph::empty();
    
    // Add Repository node
    let repo_idx = graph.graph.add_node(NodeData::Repository {
        path: repo_root.to_path_buf(),
        metadata: HashMap::new(),
    });
    graph.node_index.insert(format!("repo:{}", repo_root.display()), repo_idx);
    graph.node_index.insert("repo:root".to_string(), repo_idx);

    // Add File node
    let file_path = repo_root.join("src/lib.rs");
    let file_idx = graph.graph.add_node(NodeData::File {
        path: file_path.clone(),
        language: "Rust".to_string(),
        metadata: HashMap::new(),
    });
    graph.node_index.insert(format!("file:{}", file_path.display()), file_idx);
    graph.graph.add_edge(repo_idx, file_idx, EdgeData::Contains);

    // Add Symbol node
    let sym_idx = graph.graph.add_node(NodeData::Symbol {
        name: "helper".to_string(),
        kind: SymbolKind::Function,
        exported: true,
        references: 1,
        metadata: HashMap::new(),
    });
    graph.node_index.insert("symbol:helper".to_string(), sym_idx);
    graph.graph.add_edge(file_idx, sym_idx, EdgeData::Contains);

    let file_infos = vec![FileInfo {
        path: file_path,
        language: Language::Rust,
        symbols: vec![Symbol {
            name: "helper".to_string(),
            kind: SymbolKind::Function,
            byte_start: 0,
            byte_end: 25,
            line_start: 1,
            line_end: 3,
            exported: true,
            signature: Some("pub fn helper()".to_string()),
            references: 1,
        }],
        imports: vec![],
        references: vec![],
        original_tokens: 10,
    }];

    let canon_ref = CanonStateRef::new(&file_infos, &graph);
    canon_ref.save(canon_path)?;
    Ok(())
}

#[test]
fn test_comprehensive_cli_flow() {
    // 1. Build the boil binary to ensure it's up to date
    let build_status = Command::new("cargo")
        .arg("build")
        .status()
        .expect("Failed to run cargo build");
    assert!(build_status.success(), "Cargo build failed");

    let mut boil_path = PathBuf::from("target/debug/boil");
    if !boil_path.exists() {
        boil_path = PathBuf::from("../target/debug/boil");
    }
    assert!(boil_path.exists(), "boil executable not found");

    // 2. Set up temporary test directory under private/ for testing
    let private_dir = Path::new("private");
    if !private_dir.exists() {
        fs::create_dir(private_dir).unwrap();
    }
    
    let temp_dir = tempfile::Builder::new()
        .prefix("comp_test_")
        .tempdir_in(private_dir)
        .unwrap();

    let repo_root = temp_dir.path().join("mock_repo");
    fs::create_dir_all(repo_root.join("src")).unwrap();

    // Write initial source file
    let lib_rs = repo_root.join("src/lib.rs");
    fs::write(&lib_rs, "pub fn helper() {\n    println!(\"helper!\");\n}\n").unwrap();

    let canon_dir = temp_dir.path().join("canon");
    fs::create_dir(&canon_dir).unwrap();
    let canon_bin = canon_dir.join("canon.bin");

    // Create dummy canon.bin
    create_dummy_canon(&repo_root, &canon_bin).unwrap();

    let output_dir = temp_dir.path().join("output");
    fs::create_dir(&output_dir).unwrap();

    // 3. Test Batch Command
    let batch_status = Command::new(&boil_path)
        .arg("batch")
        .arg(&repo_root)
        .arg(&output_dir)
        .arg(&canon_bin)
        .status()
        .unwrap();
    assert!(batch_status.success(), "boil batch command failed");

    // Test Batch Command without canon.bin (indexing on-the-fly)
    let output_dir_no_canon = temp_dir.path().join("output_no_canon");
    fs::create_dir(&output_dir_no_canon).unwrap();

    let batch_status_no_canon = Command::new(&boil_path)
        .arg("batch")
        .arg(&repo_root)
        .arg(&output_dir_no_canon)
        // Omit canon_bin
        .status()
        .unwrap();
    assert!(batch_status_no_canon.success(), "boil batch command without canon.bin failed");

    let batch_root_no_canon = fs::read_dir(&output_dir_no_canon)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with("batch_"))
        .unwrap()
        .path();

    let layers_dir_no_canon = batch_root_no_canon.join("layers");
    assert!(layers_dir_no_canon.join("L0_partial").exists(), "Missing L0_partial for no-canon");
    assert!(layers_dir_no_canon.join("L1_skeletal").exists(), "Missing L1_skeletal for no-canon");
    assert!(layers_dir_no_canon.join("L2_architectural").exists(), "Missing L2_architectural for no-canon");

    // Find the actual batch path generated (since it contains timestamp prefix)
    let batch_root = fs::read_dir(&output_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with("batch_"))
        .unwrap()
        .path();

    // Verify L0, L1, L2 layers were generated
    let layers_dir = batch_root.join("layers");
    assert!(layers_dir.join("L0_partial").exists(), "Missing L0_partial");
    assert!(layers_dir.join("L1_skeletal").exists(), "Missing L1_skeletal");
    assert!(layers_dir.join("L2_architectural").exists(), "Missing L2_architectural");

    // 4. Test Setbatch Command
    let setbatch_status = Command::new(&boil_path)
        .arg("setbatch")
        .arg(&batch_root)
        .status()
        .unwrap();
    assert!(setbatch_status.success(), "boil setbatch command failed");

    // 5. Test Status Command
    let status_output = Command::new(&boil_path)
        .arg("status")
        .output()
        .unwrap();
    assert!(status_output.status.success());
    let status_str = String::from_utf8(status_output.stdout).unwrap();
    assert!(status_str.contains("Batch Status"));

    // 6. Test Ls Command
    let ls_output = Command::new(&boil_path)
        .arg("ls")
        .arg("L1_skeletal")
        .arg("src")
        .output()
        .unwrap();
    assert!(ls_output.status.success());
    let ls_str = String::from_utf8(ls_output.stdout).unwrap();
    assert!(ls_str.contains("lib.rs"));

    // 7. Test Find Command
    let find_output = Command::new(&boil_path)
        .arg("find")
        .arg("helper")
        .output()
        .unwrap();
    assert!(find_output.status.success());
    let find_str = String::from_utf8(find_output.stdout).unwrap();
    assert!(find_str.contains("helper"));

    // 8. Test Read File Command
    let show_output = Command::new(&boil_path)
        .arg("read")
        .arg("file")
        .arg("L1_skeletal")
        .arg("src/lib.rs")
        .output()
        .unwrap();
    assert!(show_output.status.success());
    let show_str = String::from_utf8(show_output.stdout).unwrap();
    assert!(show_str.contains("pub fn helper()"));

    // 9. Test Write Command (Insert at line 1)
    let write_status = Command::new(&boil_path)
        .arg("write")
        .arg("src/lib.rs")
        .arg("1")
        .arg("// first line!")
        .status()
        .unwrap();
    assert!(write_status.success());

    // Verify insertion
    let updated_content = fs::read_to_string(&lib_rs).unwrap();
    assert!(updated_content.starts_with("// first line!\npub fn helper()"));

    // 10. Test Delete Command (Delete line 1)
    let delete_status = Command::new(&boil_path)
        .arg("delete")
        .arg("src/lib.rs")
        .arg("1")
        .status()
        .unwrap();
    assert!(delete_status.success());

    // Verify deletion
    let post_delete_content = fs::read_to_string(&lib_rs).unwrap();
    assert!(post_delete_content.starts_with("pub fn helper()"));
}
