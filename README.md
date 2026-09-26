# demoparser maintained Rust branch

This branch of [unicbm/demoparser](https://github.com/unicbm/demoparser)
contains the maintained Rust parsing core used by CS2 DemoTracer. The repository
name remains `demoparser`; the Cargo crates remain `parser` and `csgoproto`.
It contains the current minimal maintained snapshot, not the upstream Python or
JavaScript distributions. See [provenance](PROVENANCE.md), the preserved
[upstream README](README.upstream.md), and [maintenance notes](README.vendor.md).

## Build and check

Install stable Rust with rustfmt and the native C/C++ toolchain required by
`mimalloc`. From this repository root:

```powershell
cargo build --workspace --release --locked
pwsh -NoProfile -File tools/check.ps1
```

The root workspace and root `Cargo.lock` are authoritative. Generated protobuf,
message and lookup-map sources are checked in. Normal builds do not clone a
GameTracking repository, invoke `protoc`, or regenerate source.

The check script runs formatting checks, checks all workspace targets, runs the
self-contained tests, and compiles the external-demo regressions. Formatting
covers crate entrypoints, build scripts and the modified fixture harness;
preserved upstream/generated modules are not globally reformatted.

## Test data boundary

Default `cargo test --workspace --release --locked` runs the synthetic unit and
regression tests embedded in the parser modules. They require no demo download.
The original `e2e_test` golden assertions remain in source behind the
`external-demo-tests` feature and carry an explicit ignored-test annotation.
Their expected values describe the original upstream `test_demo.dem`; an
arbitrary demo is not a substitute. No demo fixture or fixture digest was
included in the imported minimal snapshot.

To run that separate lane with a local copy of the original fixture:

```powershell
pwsh -NoProfile -File tools/check.ps1 -FixtureTests -DemoPath path/to/test_demo.dem
```

Or set `DEMOPARSER_TEST_DEMO` to its absolute path and run:

```powershell
cargo test -p parser --lib --release --locked --features external-demo-tests e2e_test:: -- --ignored
```

The explicit lane fails if the fixture is missing. A default green check means
self-contained regressions passed and fixture assertions compiled; it does not
claim that the external golden tests ran. Keep local demos out of Git.

## License

The upstream MIT [LICENSE](LICENSE) is preserved verbatim. This maintained
branch retains upstream source and attribution; see [PROVENANCE.md](PROVENANCE.md).
