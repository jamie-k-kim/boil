![version](https://img.shields.io/badge/version-0.1.0-blue)
![msrv](https://img.shields.io/badge/msrv-1.85.0-blue)
![license](https://img.shields.io/badge/code%20license-MIT-blue.svg)

### [**Features**](#features)&ensp;|&ensp;[**Installation**](#installation)&ensp;|&ensp;[**Tutorial**](#tutorial)&ensp;|&ensp;[**Custom Plugins**](https://github.com/jamie-k-kim/boil/wiki/WASM-Plugin-API-Reference)&ensp;

**Boil** is an engine library, CLI, and MCP server that generates various graph representations of a repository, and then merges them into one heterogeneous knowledge graph called a "canon." This canon serves as a truth source for the repository's structure, semantics, and evolution.

Boil then uses this canon to distill the codebase into multiple levels of compression. It then provides navigation and symbolic retrieval tools so the user (a human or an AI agent) can traverse all of the layers as one zoomable object instead of parsing the entire codebase. Boil can also directly edit the source code while automatically regenerating the canon and the distilled layers.

**Supported languages:**&ensp;_C, C++, C#, Go, Java, JavaScript, Kotlin, Python, Ruby, Rust, Swift, TypeScript_

<div style="height: 12px;"></div>

# **Features**

<h3 style="margin-bottom: 6px;">Modular Architecture</h3>

Boil uses a hexagonal (ports-and-adapters) architecture, where input / reasoning modules contribute data to build the canon. You can swap or detach any of the modules without breaking the system. When you install Boil, its ports are already attached to their respective defaults ([tree-sitter](https://github.com/tree-sitter/tree-sitter) for the Syntax Module, [fastembed](https://github.com/Anush008/fastembed-rs) for the Semantics Module, [git2](https://github.com/rust-lang/git2-rs) for the Provenance Module, etc.). But Boil has adapters for other popular tools as well (you can swap git2 for [mercurial](), fastembed for [OpenAI](https://developers.openai.com/api/reference/overview) / [Ollama](https://docs.ollama.com/capabilities/embeddings), etc.). And of course, it's easy to [create your own adapters using any language](https://github.com/jamie-k-kim/boil/wiki).

<h3 style="margin-bottom: 6px;">Scales to Large Codebases</h3>

Most heterogeneous graph engines today are written in Python or TypeScript, which bottleneck as you scale. TypeScript, in particular, compiles to single-threaded JavaScript and requires worker threads for parallel parsing, and Node.js/V8's default heap limit can trigger OOMs on monorepos with millions of nodes/edges. Boil manages memory at the byte level with no garbage collector and can index large codebases with a tiny RAM footprint. It also uses lightweight OS threads (via crates like [Rayon](https://github.com/rayon-rs/rayon)) to parse thousands of files concurrently, utilizing all CPU cores with practically no scheduling overhead.

<h3 style="margin-bottom: 6px;">Native MCP Support</h3>

Boil natively supports the Model Context Protocol (MCP) so AI agents can connect and use its tools.

<h3 style="margin-bottom: 6px;">Zero External Dependencies</h3>

Boil is portable and runs out-of-the-box on minimal systems. By default, it dynamically loads dependencies at runtime so it compiles faster during development. However, you can use static release mode (`--features static`) to compile it into a single, self-contained binary.

<div style="height: 16px;"></div>

# **Installation**

### Use Precompiled Binary (macOS / Linux / Windows)

1. Go to the [GitHub Releases](https://github.com/jamie-k-kim/boil/releases) page.

2. Download the archive matching your OS and architecture.

3. Extract the archive to retrieve the `boil` (or `boil.exe`) executable.

4. Move the executable to a directory in your system's `PATH` (such as `/usr/local/bin` on Unix systems).

### Install via Homebrew (macOS / Linux)

This automatically installs all required dependencies (including `onnxruntime`, which is required for `fastembeds`):

```
brew install https://raw.githubusercontent.com/jamie-k-kim/boil/main/packaging/boil.rb
```

### Build from Source (macOS / Linux / Windows)

To build from source, download the project and unzip. Inside the project directory, run this command:

```
cargo build --release --features static
```

That will give you a binary with zero external runtime dependencies. It will take significantly longer to build, but it will give you a better user experience. If you're not interested in that, run this command instead:

```
cargo build --release
```

After it compiles, find the executable file:

- **Linux / macOS**: `target/release/boil`
- **Windows**: `target\release\boil.exe`

Move this file into a directory in your system's `PATH` (such as `/usr/local/bin` on Unix systems).

<div style="height: 16px;"></div>

# **Tutorial**

## 1. Generate a Configuration File

Generate a template `boil.toml` configuration file:
```
boil -i
```

This will create a file called `boil.toml` in your home directory.

Mac: `/Users/<username>/boil.toml`

Linux: `/home/<username>/boil.toml`

Windows: `C:\Users\<username>\boil.toml`

Boil loads its settings globally from this single file.

## 2. Edit the Configuration File

The `boil.toml` file uses "profiles" to group configurations together. When you run the `boil` command to generate a canon, it will always use the `[default]` profile unless you explicitly choose another one using the `-p` flag.

Each profile is a TOML table, and you can configure:
* **ignore:** A list of glob patterns specifying files and directories to exclude from the canon
* **silent:** Indicates if console outputs should be suppressed

```
ignore = ["**/target/**", "**/node_modules/**", "**/.git/**"]
silent = false
```

Within profiles, you can also configure individual modules in this format: `[<profile_name>.modules.<module_name>]`

### Syntax
Configures the AST parser engine.

* **provider:** `"treesitter"`, `"wasm"`, or `"none"`
* **plugin_path:** Required if the provider is set to `"wasm"`

```toml
[default.modules.syntax]
provider = "treesitter"
```

### Semantics (Embeddings)
Configures semantic vector generation for codebase concepts.

* **provider:** `"fastembed"`, `"openai"`, `"cohere"`, `"voyageai"`, `"ollama"`, `"wasm"`, or `"none"`
* **plugin_path:** Required if the provider is set to `"wasm"`

```toml
[default.modules.semantics]
provider = "openai"
```

### Architecture (Clustering)
Groups modules and source files into logical subsystems.

* **provider:** `"leiden"`, `"wasm"`, or `"none"`
* **plugin_path:** Required if the provider is set to `"wasm"`

```toml
[default.modules.architecture]
provider = "leiden"
```

### Build
Configures build configuration parsing.

* **provider:** `"composite"`, `"cargo"`, `"npm"`, `"python"`, `"bazel"`, `"gradle"`, `"wasm"`, or `"none"`
* **plugin_path:** Required if the provider is set to `"wasm"`

```toml
[default.modules.build]
provider = "cargo"
```

### Runtime
Ingests system tracing and hotspot profiles.

* *provider:* `"json"`, `"opentelemetry"`, `"wasm"`, or `"none"`
* **plugin_path:** Required if the provider is set to `"wasm"`

```toml
[default.modules.runtime]
provider = "json"
```

### Provenance
Tracks author identity and version history metrics.

* **provider:** `"git2"`, `"mercurial"`, `"wasm"`, or `"none"`
* **plugin_path:** Required if the provider is set to `"wasm"`

```toml
[default.modules.provenance]
provider = "git2"
```

### Ownership
Resolves user/team ownership mappings.

* **provider:** `"codeowners"`, `"jira"`, `"github_teams"`, `"wasm"`, or `"none"`
* **plugin_path:** Required if the provider is set to `"wasm"`

```toml
[default.modules.ownership]
provider = "codeowners"
```

### Documentation
Extracts context from documentation files.

* **provider:** `"markdown"`, `"notion"`, `"confluence"`, `"wasm"`, or `"none"`
* **plugin_path:** Required if the provider is set to `"wasm"`

```toml
[default.modules.documentation]
provider = "markdown"
```

### Temporal
Tracks versioned graph changes and performs topological diffing between commits and codebase states.

* **provider:** `"git"` or `"none"`

```toml
[default.modules.temporal]
provider = "git"
```

### Export
Specifies formats used when running boil canon export.

* **formats:** Array of strings including `"json"`, `"dot"`, `"graphml"`, and `"neo4j"`

```toml
[default.modules.export]
formats = ["json", "dot"]
```

### Complete Example

```toml
# Default Profile
[default]
ignore = [
  "**/target/**",
  "**/node_modules/**",
  "**/.git/**"
]
silent = false
[default.modules.semantics]
provider = "openai"
[default.modules.temporal]
provider = "git"
[default.modules.export]
formats = ["json", "dot"]

# Custom Profile
[my_profile]
ignore = [
  "**/target/**",
  "**/node_modules/**",
  "**/.git/**",
  "**/tests/**"
]
silent = true
[my_profile.modules.semantics]
provider = "none"
[my_profile.modules.export]
formats = ["graphml"]
```

## 3. Create a Canon

This will use your default profile:

```
boil repo/ output/
```

To use a custom profile, use the `-p` flag followed by the profile's name:

```
boil repo/ output/ -p my_profile
```

This will generate a timestamped folder inside the output (e.g., `output/canon_2026-06-08_00-01-38/`), and it will contain a binary canon (.bin) as well as any formats you included in the configuration file.

## 4. Build Your Own Custom WASM Plugins

You can find the full documentation for Boil [here](https://github.com/jamie-k-kim/boil/wiki).

## 5. Distill a Codebase

Boil can operate using a pre-generated binary canon graph (`canon.bin`) or on the fly.

* **Distillation**: Compress a repository focusing on specific hotspots and matching a compression target:
  ```
  boil distill repo/ output/ path/to/canon.bin -f "src/lib.rs" -t "50%"
  ```

* **Batch**: Generate three distilled fidelity layers (`L0_partial`, `L1_skeletal`, `L2_architectural`) inside your output folder:
  ```
  boil batch repo/ output/ path/to/canon.bin
  ```

```
calculator/      <--- This is our project, the source repository
└── main.cpp
└── helper.cpp
└── util.cpp

output/          <--- This is where we store all of our distillations
│
├── dstl_2026-06-20_12-00-00/     <--- Output of a "boil distill" execution
│   ├── dstl_manifest.toml        <--- Useful information about the distillation
│   └── calculator/               <--- Compressed version of the source repository
│        └── main.cpp.dstl
│        └── helper.cpp.dstl
│        └── util.cpp.dstl
│
└── batch_2026-06-20_13-00-00/    <--- Output of a "boil batch" execution
    ├── batch_manifest.toml       <--- Useful information about the entire batch
    └── layers/
        ├── L0_partial/             <--- Each layer is treated like a distillation...
        │   ├── dstl_manifest.toml  <--- ...so it has its own manifest
        │   └── calculator/
        │       └── main.cpp.dstl
        │       └── helper.cpp.dstl
        │       └── util.cpp.dstl
        ├── L1_skeletal/
        │   └── ...
        └── L2_architectural/     <--- Most compressed layer
            └── ...
```

## 6. Latch onto a Batch
Boil remembers your current active batch in `~/boil.toml`, the same place you configured your modules. You can edit the file, or you can set / reset the batch using commands like below.

* **Set Batch**: Tell `boil` which distilled batch to use:
  ```
  boil setbatch path/to/batch_2026-06-01_00-00-00/
  ```
* **Status**: View information about your active batch (source repo, layers, created time):
  ```
  boil status
  ```
* **Reset**: Clear active batch path:
  ```
  boil reset
  ```

## 7. Navigate the Codebase
Explore the codebase at different fidelity layers (`L0`, `L1`, `L2`, or `src`). File paths are relative to the repository's root.
* **List Directory**:
  ```
  boil ls L2 src/
  ```
* **Find Symbol**: Search for classes, functions, or structures:
  ```
  boil find helper
  ```
* **Read File**: View file contents at a given fidelity level (you can omit ".dstl" if you wish):
  ```
  boil read file L2 src/lib.rs
  ```
* **Read Symbol**: Expand a specific symbol definition:
  ```
  boil read symbol L2_architectural helper
  ```
  *(Add `--id <ID>` to disambiguate if multiple symbols match the same name)*

## 8. Edit Code
You can directly edit source files, while Boil dynamically patches the canon and the distilled layers:
* **Write/Insert Code**: Inserts a new line at `<LINE>` and pushes everything afterwards down by 1 line:
  ```
  boil write src/lib.rs 2 '  println!(\"Hello!\");'
  ```
* **Delete Code**: Deletes the specified line:
  ```
  boil delete src/lib.rs 3
  ```

## 9. JSON Option
Pass `--json` as a global flag to output results in machine-readable JSON format:
```
boil --json status
boil --json ls L2_architectural
```

<div style="height: 16px;"></div>

## 10. MCP Server

### Run the Server

To start the MCP server natively via standard input/output (stdio):
```bash
boil mcp
```

### Run via MCP Inspector

You can test the server using the MCP Inspector:
```bash
npx -y @modelcontextprotocol/inspector boil mcp
```
This launches a UI in your browser, where you can invoke and verify all `boil` tools.

### Configure for Claude Desktop

To use this server with Claude Desktop, add it to your `claude_desktop_config.json` (usually located in `~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "boil": {
      "command": "/absolute/path/to/boil",
      "args": ["mcp"]
    }
  }
}
```
*(Adjust the absolute path to match your local setup.)*