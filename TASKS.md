# Tasks

Findings and suggestions from a repository review on 2026-09-22. Ordered by
priority within each section. Check items off as they land.

## Fixes

- [x] **Make the layout-harness skip loud and opt-in.**
  `vdj-plugin/tests/layout.rs` prints `SKIP` to stderr when the Atomix SDK
  headers are absent, which `cargo test` swallows, so the test reports green on
  any machine without `vendor/` or `VDJ_SDK`. The README's claim that
  `cargo test` runs the C++ cross-check is therefore false by default. Print the
  skip to stdout, and fail unless the skip is explicitly allowed (e.g.
  `VDJ_SDK_OPTIONAL=1`, or a default-on `sdk-harness` cargo feature that CI
  disables).

- [x] **Replace stale monorepo paths in comments.** These date from when the
  crate lived inside the reference repo:
  - `package.sh` lines 4-5 (`rust/package.sh`, `rust/bundle/`) and line 7
    (`tools/plugin/build.sh`).
  - `examples/tremolo/src/lib.rs` line 9 (`rust/package.sh`).
  - `docs/Plugin SDK.md` cited as if local in `vdj-plugin/src/lib.rs`,
    `src/sys.rs`, `src/host.rs`, and `tests/layout.rs`.
  - `docs/VDJScript Local Test Tracker.md` cited in `src/host.rs` and
    `src/wrapper.rs`.
  Point them at the `virtualdj-api-reference` URLs the README already uses.

- [x] **Invalidate `SharedHost` when its instance is dropped.**
  `SharedHost` (`vdj-plugin/src/host.rs`) copies a raw `IVdjCallbacks8`
  pointer with no lifetime tie to the plugin instance. The companion example's
  `Drop` only sets a stop flag and never joins its poll thread, so after plugin
  unload the thread can still call into the host through a freed object. Back
  `SharedHost` with an `Arc<AtomicPtr<IVdjCallbacks8>>` that the instance nulls
  in `Drop`, so late calls return `E_FAIL` instead of dereferencing garbage.
  Then have the companion example join its thread on drop.

- [x] **Parameterize the target arch in `package.sh`.** It hardcodes
  `-arch arm64`. The README says the x86_64 macOS layouts should hold but are
  unverified; an `ARCH` variable (or a universal build once both Rust targets
  are installed) makes that cheap to check.

## Features

- [ ] **Parameter declaration (`DeclareParameter`).** The biggest functional
  gap: a Sound Effect without sliders or a default parameter UI is a demo, not
  a shippable effect. Add a `Param` type the plugin struct owns (an
  `AtomicF32`-style cell). Because the plugin lives inside the boxed
  `Instance`, its address is stable, so
  `Host::declare_slider(&Param, id, name, short_name, default)` can hand the
  host a pointer safely. This unlocks the already-wired but dead
  `on_parameter`, `parameter_string`, and default-UI paths. Extend the tremolo
  example with a depth slider to live-verify it.

- [ ] **Typed verb layer.** Generate per-verb methods from the reference repo's
  verb store (`docs/vdjscript-verbs.json` plus the plugin-channel HRESULT
  captures): `deck(Left).title()` style calls that pick `GetInfo` vs
  `GetStringInfo` and surface HRESULTs as `Result`. This is the crate's real
  differentiator over the C++ SDK.

- [ ] **`IVdjPluginStartStop8` wrapper.** Nearly free: it is the Basic vtable
  plus `OnStart`/`OnStop`, and the harness already asserts those slot
  positions (`ss_slot_onstart`, `ss_slot_onstop`).

- [ ] **Buffer/position DSP variants** (`IVdjPluginBuffer8` and friends), after
  parameters land.

- [ ] **Windows support** via the MSVC vtable variants and the 16-byte GUID.
  Large; the layout harness approach carries over directly.

## Tooling and hygiene

- [ ] **CI workflow.** A GitHub Actions macOS job running `cargo fmt --check`,
  `cargo clippy --workspace --all-targets`, `cargo test --workspace`, and
  `./package.sh` for all three examples. The C++ harness cannot run there
  without the headers, which is why the skip must be loud (see Fixes).

- [ ] **Crate metadata for publishing.** Add `repository`, `readme`,
  `keywords`, and `categories` to `vdj-plugin/Cargo.toml`; drop
  `publish = false` once parameters land.

- [ ] **Compile-check the lib doc example.** `vdj-plugin/src/lib.rs` marks its
  example ` ```ignore `; ` ```no_run ` would at least compile it.

- [ ] **`package.sh --uninstall`** to remove an installed bundle from
  `PluginsMacArm/<subdir>/`.

- [ ] **Shared file-logging helper** in the crate (behind a feature), so
  examples stop reimplementing the tremolo's `log_line`.

- [ ] **Plist nit.** `LSMinimumSystemVersion` is `10.14` in an arm64-only
  bundle; `11.0` is the honest floor.
