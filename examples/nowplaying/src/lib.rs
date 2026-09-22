//! Rust Now Playing — a custom-GUI example for vdj-plugin.
//!
//! A Sound Effect whose panel shows the track loaded on the left and right
//! decks, live. The GUI is the `VDJINTERFACE_SKIN` path: the plugin hands
//! VirtualDJ skin XML plus a PNG, and the *skin engine* does all the data
//! binding — the backtick expressions in `format=""` (`` `deck left
//! get_title` ``) re-evaluate continuously, so the Rust side never polls the
//! decks at all. Audio passes through untouched.
//!
//! Install: `./package.sh nowplaying --install`, restart VirtualDJ, select
//! "RustNowPlaying" in a deck's effects list and open its GUI.

use vdj_plugin::{DspContext, DspPlugin, Host, PluginInfo, UserInterface, VdjPlugin};

struct NowPlaying;

impl VdjPlugin for NowPlaying {
    fn new() -> Self {
        NowPlaying
    }

    fn info(&self) -> PluginInfo {
        PluginInfo {
            name: "Rust Now Playing".into(),
            author: "vdj-plugin".into(),
            description: "Shows the current track on the left and right decks (vdj-plugin GUI example)"
                .into(),
            version: env!("CARGO_PKG_VERSION").into(),
            flags: 0,
        }
    }

    fn user_interface(&mut self, _host: &Host) -> Option<UserInterface> {
        // Rebuilt on every panel open (the host re-asks each time); both
        // buffers are compiled into the binary.
        Some(UserInterface::Skin {
            xml: include_str!("../assets/skin.xml").to_string(),
            image: include_bytes!("../assets/skin.png").to_vec(),
        })
    }
}

impl DspPlugin for NowPlaying {
    fn process_samples(&mut self, _buffer: &mut [f32], _ctx: &DspContext) {
        // Pass-through: this effect only exists to own a GUI surface.
    }
}

vdj_plugin::export_dsp_plugin!(NowPlaying);
