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

## File comparison preparation follow-up

The November 11, 2026 follow-up measures the previous implementation at `589a353` against the
optimized comparison paths. Both measurements use release builds on macOS with an Apple M1 Max. The
preparation probe runs three times after syntax initialization and reports medians. Generated Rust
fragments contain one changed line unless the scenario explicitly uses identical content.

| Scenario                    | Input bytes | Total before |  Total after | Comparison stage before | Comparison stage after |
| --------------------------- | ----------: | -----------: | -----------: | ----------------------: | ---------------------: |
| 8,000 Rust lines            |     349,777 | 1,807.203 ms |   917.551 ms |              903.268 ms |              13.040 ms |
| 30,000 Rust lines           |   1,357,777 | 6,934.256 ms | 3,490.675 ms |            3,463.632 ms |              29.433 ms |
| 30,000 identical Rust lines |   1,357,780 | 7,112.353 ms | 3,562.836 ms |            3,605.879 ms |              16.682 ms |
| 100,000 short text lines    |     200,000 |    61.181 ms |    25.543 ms |               34.803 ms |              17.203 ms |

Total preparation includes current-version syntax preparation and comparison assembly. It excludes
initial plain rendering, file reads, network access, and application message delivery. The
comparison stage includes previous-version syntax preparation. Each column reports its own median;
total preparation uses the median sum of both stages. The 30,000-line fixtures fit server comparison
limits but exceed the Git preview byte limit. These generated fixtures do not predict every real
file or language.

Comparison assembly computes line changes before preparing the previous version. It reuses unchanged
highlighted blocks when their complete parser and highlighting states match. Checkpoints occur every
64 source lines during initial highlighting. Text equality alone cannot authorize reuse after a
multiline comment or string changes. Identical snapshots reuse all prepared syntax directly. Server
responses reuse the prepared local snapshot only when its complete text still matches. Initial
syntax preparation remains proportional to source complexity and runs on a worker. The diff
algorithm deadline does not include initial syntax preparation.

Shared immutable source lines and syntax spans replace deep document copies. Copying an 8,000-line
prepared document decreased from 1.404 ms to 0.060 ms. Copying a 30,000-line prepared document
decreased from 6.168 ms to 0.223 ms. Prepared tab expansion also removes displayed-string
allocations during row selection construction.

The short-line comparison's estimated retained allocation decreased from 27,523,754 bytes to
13,001,102 bytes, approximately 53%. Small repeated text lines share storage, and comparison cells
use compact boxed emphasis ranges. The estimate counts shared allocations once within each document
and conservatively across documents. It includes shared-allocation counters but excludes allocator
metadata, renderer resources, and total process overhead. Syntax snapshots reserve an additional 4
KiB per checkpoint because Syntect's internal stack capacities are opaque. This reserve estimates
cache cost; it does not impose a hard limit on parser stacks. Ordinary Rust comparison allocations
remained approximately unchanged: 7.34 MB to 7.43 MB for the 8,000-line fixture.

Source reads now also enforce a 100,000-line limit, independently of byte limits. Denser local and
historical files use existing bounded page navigation. Server previews and comparisons reject files
above that line count through the existing large-file error. Comparison documents can contain lines
from both bounded versions.

The shared word-span builder advances through sorted emphasis ranges instead of restarting each
search. A synthetic 8,192-byte line with 4,096 syntax spans and emphasis ranges improved from 6.880
ms to 0.156 ms. These values average 20 builder calls and exclude text layout and drawing. Unicode
boundaries, syntax colors, and ranges crossing span boundaries retain regression coverage.

Git refreshes reuse the latest status snapshot and canonical repository roots. A single bounded
baseline cache uses repository path, HEAD revision, and original filename as its key. HEAD
resolution remains active, so commits and branch switches invalidate the baseline. Renames,
deletions, nested repositories, and nested project scopes retain regression coverage. A five-run
temporary-repository fixture measured uncached refresh at 54.287 ms and cached refresh at 6.970 ms.
Both paths ran against the same fixture; the common status poll remains outside these timings.

The existing GPU scroll test still passes in both comparison views. The following figures include
interface construction and layout, excluding drawing and GPU capture.

| View         | Median before | Median after | p95 before | p95 after |
| ------------ | ------------: | -----------: | ---------: | --------: |
| Unified      |       0.21 ms |      0.21 ms |    0.53 ms |   1.23 ms |
| Side-by-side |       0.25 ms |      0.23 ms |    0.90 ms |   0.83 ms |

Unified tail latency increased in this sample despite an unchanged median. These short runs vary
with system activity and do not establish a native presentation rate. GPU capture includes
synchronous screenshot readback and is reported separately by the test.

Run the retained preparation and refresh probes with:

```sh
IWE_PROFILE_COMPARISON=1 cargo test --locked --release comparison_audit_probe -- --nocapture --test-threads=1
IWE_PROFILE_COMPARISON=1 cargo test --locked --release cached_baselines_keep_nested_repository_ownership -- --nocapture --test-threads=1
IWE_PROFILE_BACKEND=wgpu cargo test --locked --release source_scroll_frames -- --nocapture --test-threads=1
```

PowerShell users can set the corresponding `$env:IWE_PROFILE_COMPARISON` or
`$env:IWE_PROFILE_BACKEND` value before Cargo. The preparation probe runs only when its profiling
variable is present. Correctness regressions run independently during ordinary checks, without
timing thresholds. Live server latency and Windows/Linux performance were not measured.
