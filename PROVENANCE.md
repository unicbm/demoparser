# Source and maintenance provenance

- Original project: [LaihoE/demoparser](https://github.com/LaihoE/demoparser).
- Maintained fork: [unicbm/demoparser](https://github.com/unicbm/demoparser).
- Imported maintained snapshot: `third_party/demoparser` from
  [unicbm/demotracer](https://github.com/unicbm/demotracer), commit
  `012d978ecdce8a949306fdcec39e4dcd9bf7624e`.
- License: upstream MIT `LICENSE`, retained verbatim including its original
  copyright text. `README.upstream.md` preserves the imported upstream README.

The snapshot contains only the Rust `parser` and `csgoproto` crates, generated
protocol/map source, and their existing tests and maintenance notes. It excludes
upstream language bindings, demo datasets and full GameTracking source. The
snapshot does not identify a precise original upstream commit; the maintained
source commit above identifies the actual imported tree without inventing one.

`README.vendor.md` records the downstream parsing, timing, movement, usercmd,
projection and cosmetic fixes already present at import. Extraction adds a root
workspace and lockfile, independent build checks, and an explicit fixture-test
lane. It preserves the crate names and parser API used by the converter.

Protocol/map regeneration utilities remain maintenance tools. Normal builds use
the committed generated files; regeneration requires the matching external Valve
data and a separate source review. Do not run those utilities as a normal build
step or introduce generated-source network downloads into build scripts.
