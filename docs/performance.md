# Source scroll performance

The November 10, 2026 audit reproduced delays with `src/i18n/mod.rs` on macOS. The audit covered
plain source, unified comparisons, and side-by-side comparisons.

## Findings

- Generic monospace requests repeatedly searched fallback fonts during text preparation.
- Selection prepared a second text layout for every new row, even without pointer interaction.
- Selection copied syntax spans during scrolling without an active selection.
- Source blocks introduced 32 new rows together, concentrating layout work into individual frames.
- Fixed gutters clipped and drew rows twice during ordinary vertical scrolling.

The viewer now selects Menlo on macOS and Consolas on Windows. Linux retains its existing monospace
family. Selection prepares pointer layout only for clicks and selection drags. Unselected spans
retain borrowed text. Source blocks advance eight rows at a time. Vertical scrolling uses one
drawing pass per source row. Horizontal scrolling preserves fixed gutters and their input
boundaries.

## Measurement

The test uses the current `src/i18n/mod.rs`, a 1440 × 920 viewport, and 44 wheel events. It excludes
four initial frames from reported timings. GPU captures use a scale factor of two. Each event
scrolls 48 pixels. Comparisons use a generated previous version of the same source.

The same test measured both source versions with the wgpu renderer in development builds. The
comparison test applies synchronization tasks before each following frame.

| View         | Median before | Median after | 95th percentile before | 95th percentile after |
| ------------ | ------------: | -----------: | ---------------------: | --------------------: |
| Plain        |       1.18 ms |      0.74 ms |              134.53 ms |               3.75 ms |
| Unified      |       1.57 ms |      1.02 ms |              141.75 ms |               4.39 ms |
| Side-by-side |       2.24 ms |      1.31 ms |              272.43 ms |               6.88 ms |

These values measure interface construction and layout. They exclude drawing, GPU work, and
screenshot readback. The plain-source 95th percentile improved approximately 36 times in this run.

Run the frame test with the software renderer:

```sh
cargo test --locked source_scroll_frames -- --nocapture --test-threads=1
```

Include GPU rendering and screenshot readback on a machine with a supported graphics device:

```sh
IWE_PROFILE_BACKEND=wgpu cargo test --locked source_scroll_frames -- --nocapture --test-threads=1
```

Use `--release` after `--locked` to measure an optimized build. PowerShell users can set
`$env:IWE_PROFILE_BACKEND = "wgpu"` before the Cargo command.

The report separates interface construction, event processing and drawing, and GPU capture. GPU
capture includes synchronous screenshot readback. Its total is not a native presentation rate. Do
not compare software-renderer timings with GPU timings or development builds with release builds.
System load and font availability affect results. The test checks scrolling behavior without fragile
wall-clock assertions.

The regression suite also verifies bounded row preparation, Unicode selection, and fixed gutter
pixels. Native checks must cover scrolling, selection, and both comparison views. macOS measurements
do not establish Windows or Linux performance.

## Verification limits

The local macOS application launched successfully with an isolated workspace. Automated coordinate
input did not open the file in that window. Native scrolling therefore remains unverified. GPU frame
tests exercised scrolling in all three source views. Windows and Linux runs were unavailable.

The first audit's `make check` passed 385 tests. Three subprocess fixtures remain excluded from
direct execution because their parent tests start them explicitly. This repository has no separate
integration-test directory. `make build` also passed.

The dependency audit completed with ten maintenance warnings and one existing `lru` memory-safety
warning. The audit reviewed `RUSTSEC-2026-0253` against the installed dependency sources. The
dependency chain is Iced → iced_wgpu → cryoglyph 0.1.0 → lru 0.16.4. The advisory requires a stored
key's destructor to panic during `LruCache::pop()`. The reviewed renderer uses `Copy` glyph keys
without destructors and calls `pop_lru()` instead. The documented trigger is absent from this
caller. The upstream fix starts at lru 0.18.2. Dependency versions and advisory exclusions remain
unchanged by this audit.

## Memory and redraw follow-up

The follow-up limits prepared documents in closed tabs to 16 MiB. The latest 20 closures retain
their identities and navigation state. Recently closed documents receive priority within the budget.
Evicted documents reopen from their original file or Git revision on a background worker. The
application does not create additional source copies on disk. Directory metadata already has a
separate persistent cache.

The budget measures owned buffer capacities, including comparison rows and word highlights. It
excludes allocator overhead, active tabs, and renderer resources. Preparation computes these sizes
off the interface thread. Eviction also releases folding projections. Reopening restores expanded
blocks only when the document fingerprint matches. Large-file cursors remain available, and changed
large files restart at the first part.

A regression scenario closes 20 distinct documents containing 8,000 lines each. Their retained
document allocations decrease from 46,912,000 bytes to 16,419,200 bytes, approximately 65%. These
figures measure document allocations, not total process memory.

Icon widgets now cache their prepared geometry. Identity, appearance, color, and dimensions
invalidate the relevant cache. Comparison assembly also moves prepared lines instead of cloning
their text and syntax spans. Plain document fingerprinting no longer clones the complete source.

A second test redraws 96 icons across 44 frames. It discards four initial frames and uses wgpu.
Development-build drawing time decreases from 3.270 ms to 0.090 ms at the median. Its 95th
percentile decreases from 4.071 ms to 0.125 ms. These timings exclude screenshot readback and
measure only icon drawing. Pixel comparisons verify folder expansion, icon replacement, and theme
changes against fresh drawing.

Run the additional regressions with:

```sh
cargo test --locked closed_documents -- --nocapture
cargo test --locked suspended_ -- --test-threads=1
IWE_PROFILE_BACKEND=wgpu cargo test --locked icon_redraws -- --nocapture --test-threads=1
```

`make build` and `make run` use the development profile. Use `make release` for an optimized
executable, or `cargo run --locked --release` to build and start it. Performance comparisons must
use the same build profile and renderer. Moving active text or renderer caches to disk would add
reads during interaction, so these caches remain in memory.

## Terminal and directory-index follow-up

This pass reviews source code without creating application or test executables. Its findings are
structural; it reports no new timing or process-memory measurements.

- The terminal previously rebuilt every visible cell during unrelated redraws. It now retains
  drawing geometry until its screen revision or presentation changes. The terminal also shares the
  source viewer's platform font.
- The index previously counted every cached entry before each directory read. It now maintains that
  count during insertions, replacements, and subtree removals. Pruning visits only the refreshed
  subtree.
- Cache writes previously cloned directory entries and allocated the complete JSON document.
  Serialization now borrows entries and streams through a bounded buffer into an atomic temporary
  file. The existing 32 MiB limit and cache format remain unchanged.
- A full output queue previously blocked index cancellation. Queue waits now check cancellation, and
  a closed receiver cancels further traversal. A dedicated session thread replaces the long-lived
  task in the asynchronous runtime's blocking pool.

Regression tests cover terminal drawing reuse, visual invalidation, scrollback revisions, subtree
counts, cache compatibility, oversized writes, and cancellation with a full queue. These tests
require execution in a later verification run. This pass checks their types without linking test
executables. Native checks and Windows/Linux execution remain unavailable in this pass.
