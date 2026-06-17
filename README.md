<img src="assets/boil.png" width="150">

![version](https://img.shields.io/badge/version-0.1.0-blue)
![msrv](https://img.shields.io/badge/msrv-1.85.0-blue)
![license](https://img.shields.io/badge/code%20license-MIT-blue.svg)

### [**Features**](#features)&ensp;|&ensp;[**Installation**](#installation)&ensp;|&ensp;[**Quickstart**](#quickstart)&ensp;|&ensp;[**Documentation**](https://github.com/jamie-k-kim/boil/wiki)&ensp;

Boil is an engine library and CLI that merges graph representations of a repository into one heterogeneous knowledge graph, called a "canon." This canon serves as a truth source for the repository's structure, semantics, and evolution. It doesn't answer questions about the codebase, but rather, it's an alternative representation from which answers are easier to derive.

**Supported languages:**&ensp;_C, C++, C#, Go, Java, JavaScript, Kotlin, Python, Ruby, Rust, Swift, TypeScript_

<div style="height: 12px;"></div>

# **Features**

<h3 style="margin-bottom: 6px;">Modular Architecture</h3>

Boil uses a hexagonal (ports-and-adapters) architecture, where input / reasoning modules contribute data to build the canon. You can swap or detach any of the modules without breaking the system. When you install Boil, its ports are already attached to their respective defaults ([tree-sitter](https://github.com/tree-sitter/tree-sitter) for the Syntax Module, [fastembed](https://github.com/Anush008/fastembed-rs) for the Semantics Module, [git2](https://github.com/rust-lang/git2-rs) for the Provenance Module, etc.). But Boil has adapters for other popular tools as well (you can swap git2 for [mercurial](), fastembed for [OpenAI](https://developers.openai.com/api/reference/overview) / [Ollama](https://docs.ollama.com/capabilities/embeddings), etc.). And of course, it's easy to [create your own adapters using any language](https://github.com/jamie-k-kim/boil/wiki).

<h3 style="margin-bottom: 6px;">Scales to Large Codebases</h3>

Most heterogeneous graph engines today are written in Python or TypeScript, which bottleneck as you scale. TypeScript, in particular, compiles to single-threaded JavaScript and requires worker threads for parallel parsing, and Node.js/V8's default heap limit can trigger OOMs on monorepos with millions of nodes/edges. Boil manages memory at the byte level with no garbage collector and can index large codebases with a tiny RAM footprint. It also uses lightweight OS threads (via crates like [Rayon](https://github.com/rayon-rs/rayon)) to parse thousands of files concurrently, utilizing all CPU cores with practically no scheduling overhead.

<h3 style="margin-bottom: 6px;">No External Runtime Dependencies</h3>

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

# **Quickstart**

## 1. Generate a Configuration File

Generate a template `boil.toml` configuration file at your repository's root:
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

<div style="height: 16px;"></div>

# **Documentation**

You can find the full documentation for Boil [here](https://github.com/jamie-k-kim/boil/wiki).