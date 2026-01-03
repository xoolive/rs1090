# Contribute to the project

`jet1090` is an open-source project distributed under the MIT license.

Contributions in any form are welcome, but please follow these minimal guidelines:

- for all contributions, open a pull request on GitHub
- if you fix a typo in the code or in the documentation, thank you!
- if you fix a bug with your contribution, please file an issue first and describe the problem
- if you want to implement a new functionality, or improve an existing one, briefly explain the motivation in the pull request.

!!! tip "New functionalities"

    - In particular, any functionality supported by an existing tool but not by `jet1090` is welcome.
    - I have a particular interest for supporting more SDR devices, but I only own the cheap RTL-SDR ones.

## Testing Guidelines

### Running Tests

**Rust tests:**

```bash
# Run all workspace tests
cargo test --workspace --all-features --all-targets

# Run tests for specific crate
cargo test -p rs1090 --all-features

# Run specific test module
cargo test -p rs1090 --lib bds06  # BDS 0,6 tests
cargo test -p rs1090 --lib bds09  # BDS 0,9 tests

# Run with Nix (uses cargo-nextest)
nix run .#checks.test-check
```

**Python tests:**

```bash
cd python
uv run pytest                      # All tests
uv run pytest tests/test_adsb.py   # Specific file
uv run pytest -v                   # Verbose output
```

**Benchmarks:**

```bash
cargo bench                        # Rust benchmarks
cd python/examples && python benchmark.py  # Python benchmarks
```

### Writing Tests

#### Test Coverage Expectations

- **Bug fixes:** Every bug fix should include a test that would have caught the bug
- **New features:** New functionality must include comprehensive tests
- **Edge cases:** Tests should cover corner cases, boundary conditions, and error paths
- **Real data:** Prefer real ADS-B messages over synthetic data when possible

#### Test Organization

**Rust:**

- Place tests in the same file as the code being tested (bottom of file)
- Use `#[cfg(test)]` module
- Group related tests together
- Use descriptive test names: `test_<feature>_<condition>_<expected_result>`

**Python:**

- Create test files in `python/tests/` directory
- Name files `test_<module>.py`
- Use classes to group related tests: `class TestBDS06SurfacePosition`
- Mirror Rust test structure for consistency

#### Testing Corner Cases

Focus on these common edge cases:

1. **Boundary values:** 0, max, min, -1, +1 around thresholds
2. **Sign bits:** Positive, negative, zero values
3. **Status bits:** Valid (status=1) and unavailable (status=0) combinations
4. **Reserved fields:** Ensure reserved bits are validated or ignored appropriately
5. **Cross-validation:** Check consistency between related fields

**Example corner cases for movement codes:**

```rust
// Code 0: No information available
test_movement_no_info()

// Code 1: Stopped (0.0 kt)
test_movement_stopped()

// Code 13-38: Verify 0.5 kt steps (validates bug fix)
test_movement_2_15kt_range()

// Code 124: Maximum (175+ kt)
test_movement_175kt_plus()
```

### Continuous Integration

All tests run automatically on GitHub Actions:

- `rust.yml`: Rust tests, clippy, formatting
- `python.yml`: Python tests, type checking, linting
- `wasm.yml`: WebAssembly tests

Ensure all tests pass locally before opening a pull request:

```bash
# Rust quality checks
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check

# Python quality checks
cd python
uv run pytest
uv run ruff check
uv run ruff format --check
```
