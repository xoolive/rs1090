# Agent development guide

This guide provides comprehensive instructions for AI agents working on the jet1090/rs1090 project.

## Project overview

- Real-time enriched trajectory data serving
- Cross-platform export formats (JSON, gRPC, Arrow)
- Inspired by [pyModeS](https://github.com/junzis/pyModeS) library design
- Uses [deku](https://github.com/sharksforarms/deku) for declarative binary data decoding

## Project structure

```
jet1090/
├── crates/
│   ├── rs1090/          # Core decoding library (Mode S, ADS-B, FLARM)
│   ├── jet1090/         # Live decoding application with TUI and web server
│   ├── decode1090/      # Companion CLI decoding tool
│   └── rs1090-wasm/     # WebAssembly bindings for browser usage
├── python/              # Python bindings (PyO3/maturin)
├── docs/                # MkDocs documentation (deployed to mode-s.org/jet1090)
├── samples/             # Real flight trajectory data for testing (private)
├── references/          # ADS-B/Mode S specification PDFs (ICAO standards, private)
└── container/           # Docker/Podman container definitions
```

### Crate responsibilities

- **rs1090**: Core library with decoding logic, CPR algorithms, data sources (RTL-SDR, SeRo, SSH, Beast)
- **jet1090**: Full-featured application with TUI, web server, snapshot management, deduplication
- **decode1090**: Lightweight CLI tool for batch message decoding
- **rs1090-wasm**: Browser-compatible WebAssembly bindings
- **python/**: Python bindings exposing `decode()` and `flarm()` functions

## Setup and build

### Initial build

```sh
cargo build --release --all-features
```

### Building specific components

```sh
# Core library only
cargo build -p rs1090 --release

# jet1090 application
cargo build -p jet1090 --release

# Python bindings (requires uv)
cd python
uv sync --all-extras --dev
maturin develop

# WebAssembly bindings
cd crates/rs1090-wasm
wasm-pack build --target web
```

### Nix platform

```sh
nix develop              # Enter development environment
nix build                # Build jet1090 (default package)
nix run                  # Run jet1090 directly
nix profile install      # Install to PATH
```

## Testing

### Rust tests

```sh
# Run all tests (workspace-wide)
cargo test --workspace --all-features --all-targets

# Run tests for specific crate
cargo test -p rs1090 --all-features

# Run specific test
cargo test test_name -- --nocapture

# Run with Nix
nix run .#checks.test-check  # Uses cargo-nextest
```

### Benchmarks

```sh
# Run Rust benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench long_flight

# Python benchmarks
cd python/examples
python benchmark.py
```

### Python tests

```sh
cd python
uv run pytest                # Run all tests
uv run pytest tests/test_adsb.py  # Specific test file
uv run pytest -v             # Verbose output
```

### WebAssembly tests

```sh
cd crates/rs1090-wasm/tests
npm install
npm test
```

## Code quality and style

### Rust

**Linting:**

```sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**Formatting:**

```sh
cargo fmt --all              # Format all code
cargo fmt --all --check      # Check without modifying
```

**Documentation:**

```sh
cargo doc --all-features --no-deps        # Build docs
cargo doc --all-features --no-deps --open # Build and open in browser

# Check for documentation issues
RUSTDOCFLAGS="-D rustdoc::all -A rustdoc::private-doc-tests" cargo doc --all-features --no-deps
```

### Python

```sh
cd python
uv run ruff check            # Linting
uv run ruff format           # Formatting
uv run ruff format --check   # Check formatting without modifying

# Type checking (run both)
uvx ty check
uv run mypy
```

### Markdown

- Use `prettier` for formatting documentation and markdown files
- Follow CommonMark specification

### Code conventions

**Rust:**

- Use descriptive variable names (e.g., `icao24`, `latitude_cpr`, `groundspeed`)
- Prefer declarative deku attributes for binary decoding
- Document public APIs with `///` doc comments
- Use `tracing` for logging, not `println!`
- Handle errors with `Result<T, E>`, avoid unwrap in library code
- Use `#[must_use]` for important return values

**Python:**

- Follow PEP 8 style guide (enforced by ruff)
- Use type hints for all public functions
- Docstrings for all public functions and classes

## Documentation

### Building documentation

**MkDocs site (jet1090 user docs):**

```sh
uvx --with "mkdocs-material[imaging]" mkdocs serve  # Local preview
uvx --with "mkdocs-material[imaging]" mkdocs build -d site  # Build static site
```

Site deploys automatically to https://mode-s.org/jet1090 on push to master.

**Rust API docs:**

```sh
cargo doc --all-features --no-deps --open
```

Published automatically to https://docs.rs/rs1090

### Documentation structure

- `docs/`: MkDocs markdown files (installation, configuration, usage guides)
- `crates/rs1090/src/`: Inline Rust documentation (extracted by rustdoc)
- `readme.md`: Main repository README with quickstart examples
- `changelog.md`: Version history and release notes

## Decoding specifications and test data

### Reference materials

The `references/` directory contains official ICAO specifications:

- `DO-260B.pdf`: ADS-B specification
- `icao_doc_9871_*.pdf`: Mode S technical provisions
- `icao_annex_10_*.pdf`: Aeronautical telecommunications standards

**Extracting information:**

```sh
pdftotext references/DO-260B.pdf - | less  # Convert to text for searching
```

### Test samples

The `samples/` directory contains real-world trajectory data in JSONL format:

- Format: `YYYYMMDD_GUFI_ORIGIN_DEST.jsonl[.7z]`
- Contains timestamped Mode S messages from actual flights
- Useful for regression testing, debugging, and performance benchmarking

Do not commit files there, they must remain private.
You may only extract individual or cherry-picked sequences of messages for testing and/or statistical purposes.

## Git workflow and commits

### Branching strategy

- `master`: Main development branch (protected)
- Feature branches: `feature/description` or `fix/issue-number`
- Always create PRs for review, never push directly to master

### Commit guidelines

**IMPORTANT:**

- **Never commit without explicit user approval**
- Always ask for confirmation before creating commits
- If fixing a GitHub issue, create a dedicated branch and PR

**Commit message format:**

```
type: brief description (imperative mood)

Optional longer explanation of what changed and why.

Fixes #123
```

**Types:** `feat`, `fix`, `refactor`, `docs`, `test`, `perf`, `chore`

**Examples:**

```
feat: add support for ADS-B version 2 messages
fix: correct altitude decoding for odd CPR frames (#456)
refactor: simplify Beast protocol parser
docs: update installation instructions for Nix users
```

### GitHub issues and PRs

**Opening issues:**

```sh
# Never open issues without user acknowledgement
gh issue create --title "Title" --body "Description"
```

**Analyzing issues:**

```sh
# Always read ALL comments before planning
gh issue view 123
gh issue view 123 --comments
```

**Creating pull requests:**

```sh
# After user approves commits
gh pr create --title "Title" --body "Description"

# Link to issue
gh pr create --title "Fix altitude bug" --body "Fixes #123"
```

Update changelog after fixing issues

## Task planning

### Using plan.md

- **Always** use `plan.md` to track complex tasks
- Update frequently as you work through tasks
- Structure:

  ```markdown
  ## Current task: [Brief description]

  - [ ] Step 1
  - [ ] Step 2
  - [x] Completed step

  ## Next:

  - Future tasks
  ```

- Prune completed tasks after commits are merged

### Task breakdown approach

1. **Understand the requirement** - Read issue, analyze code context
2. **Plan steps** - Break into discrete, testable units
3. **Execute incrementally** - Small commits, test frequently
4. **Verify** - Run tests, check lints, update docs
5. **Review** - Self-review changes before proposing to user

## Support and contributions

- Test thoroughly before proposing changes
- Document breaking changes clearly in PRs
