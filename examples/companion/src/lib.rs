//! Rust Companion — a standalone now-playing window for VirtualDJ.
//!
//! The design this crate exists for: a **basic plugin in `AutoStart/`**, which
//! VirtualDJ loads at startup and then leaves completely alone (the verified
//! headless lifecycle — `OnGetPluginInfo` + `OnLoad`, nothing else). Everything
//! after that is the plugin's own doing:
//!
//! - `OnLoad` grabs a [`SharedHost`] and spawns a poll thread — host queries
//!   from a non-main thread are the live-tested pattern.
//! - The poll thread reads both decks (artist, title, loaded, play) ~3x/sec
//!   and marshals a snapshot to the main thread, where a floating `NSWindow`
//!   with plain AppKit labels shows it.
//! - Visibility is driven by the VDJScript variable `$nowplaying`, so any
//!   custom button or pad can toggle the window:
//!
//!   ```vdjscript
//!   toggle '$nowplaying'
//!   ```
//!
//!   The plugin sets `$nowplaying 1` once at startup (the window pops up when
//!   VirtualDJ loads); a button flips it thereafter, and closing the window
//!   with its close button writes the variable back to 0 so button LEDs stay
//!   truthful.
//!
//! Install: `./package.sh companion --install`, then restart VirtualDJ twice
//! (first launch of a new bundle binary starts with the plugin layer inert —
//! see README). Remove by deleting
//! `~/Library/Application Support/VirtualDJ/PluginsMacArm/AutoStart/RustCompanion.bundle`.

mod window;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use vdj_plugin::{Host, PluginInfo, Result, SharedHost, VdjPlugin};

/// One deck's snapshot, as read over the typed query channels.
#[derive(Clone, Default, PartialEq)]
pub struct DeckNow {
    pub loaded: bool,
    pub playing: bool,
    pub artist: String,
    pub title: String,
}

#[derive(Clone, Default, PartialEq)]
pub struct Snapshot {
    pub visible: bool,
    pub left: DeckNow,
    pub right: DeckNow,
}

fn read_deck(host: &SharedHost, side: &str) -> DeckNow {
    let q = |verb: &str| format!("deck {side} {verb}");
    let num = |verb: &str| host.get_info(&q(verb)).unwrap_or(0.0) != 0.0;
    let loaded = num("loaded");
    DeckNow {
        loaded,
        playing: num("play"),
        // Empty decks answer placeholder prose for get_title, so only ask
        // when loaded.
        artist: if loaded { host.get_string_info(&q("get_artist")).unwrap_or_default() } else { String::new() },
        title: if loaded { host.get_string_info(&q("get_title")).unwrap_or_default() } else { String::new() },
    }
}

struct Companion {
    stop: Arc<AtomicBool>,
}

impl Drop for Companion {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl VdjPlugin for Companion {
    fn new() -> Self {
        Companion { stop: Arc::new(AtomicBool::new(false)) }
    }

    fn info(&self) -> PluginInfo {
        PluginInfo {
            name: "Rust Companion".into(),
            author: "vdj-plugin".into(),
            description: "Floating now-playing window for both decks; toggle with `toggle '$nowplaying'`"
                .into(),
            version: env!("CARGO_PKG_VERSION").into(),
            flags: 0,
        }
    }

    fn on_load(&mut self, host: &Host) -> Result<()> {
        let shared = host.shared();
        let stop = self.stop.clone();
        // OnLoad fires early in startup; everything below waits for the host
        // to become answerable, off the loader's thread.
        std::thread::Builder::new()
            .name("rust-companion-poll".into())
            .spawn(move || poll_loop(shared, stop))
            .ok();
        Ok(())
    }
}

fn poll_loop(host: SharedHost, stop: Arc<AtomicBool>) {
    // Wait until the engine answers queries (early-startup reads fail with
    // E_INVALIDARG per the verified timing), then announce the window.
    let mut announced = false;
    while !stop.load(Ordering::Relaxed) {
        if !announced {
            if host.get_info("get_version").map(|_| true).unwrap_or(false)
                || host.get_string_info("get_version").map(|_| true).unwrap_or(false)
            {
                // Pop up on load, and make the state button-visible.
                let _ = host.send_command("set '$nowplaying' 1");
                announced = true;
            } else {
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
        }
        let snap = Snapshot {
            visible: host.get_info("get_var '$nowplaying'").unwrap_or(1.0) != 0.0,
            left: read_deck(&host, "left"),
            right: read_deck(&host, "right"),
        };
        // Sent every tick (not only on change): the window also has to notice
        // state the snapshot can't carry, like the user closing it.
        window::apply(snap, host);
        std::thread::sleep(Duration::from_millis(300));
    }
}

vdj_plugin::export_basic_plugin!(Companion);
