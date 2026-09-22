# vdj-plugin — write VirtualDJ plugins in Rust

A Rust interface to the Atomix Plugin SDK 8, built on the evidence in the
[virtualdj-api-reference](https://github.com/monomadic/virtualdj-api-reference)
repo's [Plugin SDK.md](https://github.com/monomadic/virtualdj-api-reference/blob/master/docs/Plugin%20SDK.md). The SDK is a C++ class
hierarchy, so the crate's core is a hand-laid Itanium-ABI vtable layer with a
safe trait API on top — the same architecture `vst3-sys`/`nih-plug` use for
VST3.

## Status

**Live-verified 2026-09-22 on VirtualDJ 2026 (bundle `18.0.9644`, macOS 26.6.2
arm64).** The RustTremolo bundle was loaded by the real host and driven end to
end (`~/…/VirtualDJ/rust-tremolo.log` is the capture):

- `DllGetClassObject` negotiation → listed as `RustTremolo` in the effects
  catalog (`get_effect_title 'RustTremolo'` → `RustTremolo - Deck 1`).
- `OnGetPluginInfo` + `OnLoad` on `effect_select` — and from inside `OnLoad`,
  both typed query channels answered through the Rust vtable with the correct
  split: `GetStringInfo("get_version")` → `S_OK, "2026"`;
  `GetInfo("get_version")` → `E_INVALIDARG` with `0.0` written anyway, exactly
  the HRESULT-is-the-answer behavior the reference repo documents.
- `OnStart`/`OnStop` (vtable slots 8/9) via `effect_active` toggles.
- `OnProcessSamples` (slot 10) with real audio: a 138 BPM track playing on
  deck 1 read a mean VU of ~0.65 (samples 0.33-0.91) dry, and ~0.40 (0.17-0.66)
  with the tremolo active - matching the effect's 0.35-1.0 gain sweep
  (predicted mean ~0.44) and dipping well below the dry minimum. The same
  slot is also driven offline by [fakehost/fakehost.cpp](fakehost/fakehost.cpp),
  which `dlopen`s the built bundle and drives it through compiler-generated
  vtable calls from the actual SDK headers, both teardown paths included.

- `OnGetUserInterface` / `VDJINTERFACE_SKIN` (2026-09-22, same build): the
  [nowplaying example](examples/nowplaying/) returned skin XML + a PNG from
  Rust-owned buffers; VirtualDJ rendered the panel, bound the backtick
  expressions live (`deck left get_artist`/`get_title`), honored
  `visibility=""` gating on `loaded` and `play`, and re-opened the panel
  cleanly (the host re-asks on every open, buffers replaced each call).

- The **companion-window architecture** (2026-09-22, same build): the
  [companion example](examples/companion/) is a *basic* plugin in `AutoStart/`
  — the verified headless lifecycle — that spawns its own poll thread with a
  `SharedHost`, creates a native floating `NSWindow` on the main thread, and
  binds its visibility to the VDJScript variable `$nowplaying`. Live-verified:
  the window pops up when VirtualDJ starts, shows both decks' artist/title
  with a PLAYING badge, and `toggle '$nowplaying'` — the action any custom
  button or pad can carry — hides and re-shows it, with the close button
  writing the variable back so button LEDs stay truthful.

Two host behaviors worth knowing, observed during the live run:

- **Effect bundles load lazily.** VirtualDJ lists a new bundle in the effects
  catalog without loading it (no `OnLoad`, no auto-generated `.ini`);
  `DllGetClassObject` runs on first `effect_select`, and the host instantiated
  the plugin twice. The plugin's catalog identity is the **bundle filename**,
  not the declared `PluginName`.
- **The first launch after installing a new bundle binary wedges the plugin
  layer.** Observed twice, once per freshly built bundle: VirtualDJ starts
  and runs normally but with the plugin layer inert — no Network Control HTTP
  listener, no plugin loads. A plain second relaunch (bundle still installed)
  comes up clean in seconds; nothing needs to be removed. Cause unconfirmed
  (first-seen-binary system assessment fits the shape). Practical rule:
  install, restart VirtualDJ **twice**, then verify the HTTP interface
  answers.

## MVP scope

| Piece | Where |
| --- | --- |
| Raw ABI (`HRESULT`, LP64 `GUID`, callback vtable, info structs, flags) | [vdj-plugin/src/sys.rs](vdj-plugin/src/sys.rs) |
| Object layout + plugin vtables (basic and DSP), panic-safe shims | [vdj-plugin/src/wrapper.rs](vdj-plugin/src/wrapper.rs) |
| Safe traits `VdjPlugin` / `DspPlugin`, `Host`, `SharedHost` | [vdj-plugin/src/lib.rs](vdj-plugin/src/lib.rs), [host.rs](vdj-plugin/src/host.rs) |
| Entry-point macros `export_basic_plugin!` / `export_dsp_plugin!` | [vdj-plugin/src/export.rs](vdj-plugin/src/export.rs) |
| Layout harness (C++ reference diffed against the Rust `#[repr(C)]` types) | [vdj-plugin/tests/layout.rs](vdj-plugin/tests/layout.rs) |
| Example: beat-synced tremolo Sound Effect | [examples/tremolo/](examples/tremolo/) |
| Example: custom skin GUI showing the left/right deck tracks | [examples/nowplaying/](examples/nowplaying/) |
| Example: standalone floating now-playing window (AutoStart + `NSWindow`) | [examples/companion/](examples/companion/) |
| Bundle packaging + install | [package.sh](package.sh) |

macOS only (arm64 exercised; the layouts are Itanium-ABI shapes that hold for
x86_64 macOS too, unverified). Windows would need the MSVC vtable variants.

## Build and test

```sh
cargo test          # unit offset tests + the C++ layout harness
./package.sh tremolo           # build bundle/RustTremolo.bundle
./package.sh tremolo --install # …and install; restart VirtualDJ, select "RustTremolo" in a deck's effects
```

The fake host drives a built bundle exactly as VirtualDJ does (real C++ vtable
calls, host-poked members, both destructor paths), with a hang watchdog:

```sh
SDK="$(find vendor -name vdjDsp8.h -print -quit)"; SDK="${SDK%/*}"
clang++ -std=c++17 -arch arm64 -O0 -g -I "$SDK" fakehost/fakehost.cpp -o target/fakehost
HOME=/tmp/fakehome target/fakehost bundle/RustTremolo.bundle
```

The harness needs the Atomix SDK headers, which are **not** in this repo (no
license grant — see [Plugin SDK.md](https://github.com/monomadic/virtualdj-api-reference/blob/master/docs/Plugin%20SDK.md) §Provenance). Put
them under `vendor/` (download from the official
[PluginSDK8 page](https://www.virtualdj.com/wiki/PluginSDK8.html)) or set
`VDJ_SDK=<dir>`; without them the harness skips and says so.

## Why the layout layer looks the way it does

Facts this code is built on, each verified in the reference repo:

- The host calls the plugin through a real C++ vtable **and writes data members
  directly** (`cb`, `hInstance`, and the DSP fields at offsets 24/28/32) — no
  setter virtuals exist. Hence the `#[repr(C)]` instance prefix in
  `wrapper.rs`.
- `IVdjPlugin8`'s virtual destructor takes **two** vtable slots on Itanium, so
  `OnParameter` is slot 5. The harness proves this from the compiler itself by
  decoding pointers-to-member-functions.
- The SDK's `GUID` uses `unsigned long Data1` → **24 bytes on LP64**, not the
  16-byte Windows GUID.
- Type negotiation: the host probes one export with a sequence of IIDs and
  stops at the first acceptance, so `CLASS_E_CLASSNOTAVAILABLE` (-1, the
  header's macOS value) is the normal decline path.
- A basic plugin in `AutoStart/` is a **headless service** (`OnGetPluginInfo` +
  `OnLoad`, then nothing) — background work starts in `on_load`, on its own
  thread, via `SharedHost` (cross-thread host calls: one recorded session of
  evidence, see the type's docs).
- `OnLoad` fires early in startup; `E_INVALIDARG` then means "not available
  yet", not "no such query".

## Roadmap (post-MVP)

1. Parameter declaration (`DeclareParameter*` — needs pinned storage the host
   can write into) and the default parameter UI.
2. `IVdjPluginStartStop8` and buffer/position DSP variants (slot positions for
   StartStop already captured by the harness).
3. A generated, typed verb layer from the repo's verb store
   (`docs/vdjscript-verbs.json` + plugin-channel HRESULT captures): per-verb
   methods choosing `GetInfo` vs `GetStringInfo` and surfacing `HRESULT`s as
   `Result` — the thing the C++ SDK never had.
4. Video / online-source interfaces (the latter blocked on the second-loading-
   path question in [Plugin SDK.md](https://github.com/monomadic/virtualdj-api-reference/blob/master/docs/Plugin%20SDK.md) §Loading).
