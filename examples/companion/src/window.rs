//! The floating now-playing window: plain AppKit, main-thread only.
//!
//! The poll thread calls [`apply`] with each snapshot; the UI work is
//! marshaled onto the main queue with libdispatch. AppKit objects live in a
//! main-thread `thread_local`, created lazily on the first snapshot (which is
//! also naturally after the app has finished launching — `OnLoad` itself is
//! too early to build windows).

use std::cell::RefCell;

use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSFont, NSTextAlignment, NSTextField, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use vdj_plugin::SharedHost;

use crate::{DeckNow, Snapshot};

struct Row {
    badge: Retained<NSTextField>,
    text: Retained<NSTextField>,
}

struct Ui {
    window: Retained<NSWindow>,
    left: Row,
    right: Row,
    /// Whether the last order we gave the window was "be visible" — used to
    /// tell a user-clicked close button apart from our own orderOut.
    ordered_visible: bool,
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
}

/// Marshal one snapshot onto the main thread and render it.
pub fn apply(snap: Snapshot, host: SharedHost) {
    DispatchQueue::main().exec_async(move || {
        let Some(mtm) = MainThreadMarker::new() else { return };
        UI.with(|cell| {
            let mut ui = cell.borrow_mut();
            let ui = ui.get_or_insert_with(|| build(mtm));
            render(ui, &snap, &host);
        });
    });
}

fn label(
    mtm: MainThreadMarker,
    window: &NSWindow,
    text: &str,
    size: f64,
    bold: bool,
    frame: NSRect,
) -> Retained<NSTextField> {
    let l = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    l.setFrame(frame);
    let font = if bold {
        NSFont::boldSystemFontOfSize(size)
    } else {
        NSFont::systemFontOfSize(size)
    };
    l.setFont(Some(&font));
    if let Some(content) = window.contentView() {
        content.addSubview(&l);
    }
    l
}

fn build(mtm: MainThreadMarker) -> Ui {
    let rect = NSRect::new(NSPoint::new(80.0, 140.0), NSSize::new(420.0, 168.0));
    let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable;
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            rect,
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe {
        // We hold the only strong reference; the close button must not
        // dealloc the window out from under it.
        window.setReleasedWhenClosed(false);
        window.setLevel(3); // NSFloatingWindowLevel: stay above the VirtualDJ window
    }
    window.setTitle(&NSString::from_str("Now Playing"));

    let header = |y: f64| NSRect::new(NSPoint::new(18.0, y), NSSize::new(120.0, 16.0));
    let badge = |y: f64| NSRect::new(NSPoint::new(280.0, y), NSSize::new(122.0, 16.0));
    let line = |y: f64| NSRect::new(NSPoint::new(18.0, y), NSSize::new(384.0, 22.0));

    let mk_header = |y: f64, name: &str| {
        let l = label(mtm, &window, name, 11.0, true, header(y));
        l.setTextColor(Some(&NSColor::secondaryLabelColor()));
        l
    };
    let mk_badge = |y: f64| {
        let l = label(mtm, &window, "PLAYING", 11.0, true, badge(y));
        l.setTextColor(Some(&NSColor::systemGreenColor()));
        l.setAlignment(NSTextAlignment::Right);
        l.setHidden(true);
        l
    };

    mk_header(134.0, "LEFT DECK");
    mk_header(62.0, "RIGHT DECK");
    let ui = Ui {
        left: Row {
            badge: mk_badge(134.0),
            text: label(mtm, &window, "", 16.0, true, line(106.0)),
        },
        right: Row {
            badge: mk_badge(62.0),
            text: label(mtm, &window, "", 16.0, true, line(34.0)),
        },
        window,
        ordered_visible: false,
    };
    render_row(&ui.left, &DeckNow::default());
    render_row(&ui.right, &DeckNow::default());
    ui
}

fn render_row(row: &Row, deck: &DeckNow) {
    let line = if deck.loaded {
        if deck.artist.is_empty() {
            deck.title.clone()
        } else {
            format!("{} - {}", deck.artist, deck.title)
        }
    } else {
        "no track loaded".to_string()
    };
    let color = if deck.loaded {
        NSColor::labelColor()
    } else {
        NSColor::tertiaryLabelColor()
    };
    row.text.setStringValue(&NSString::from_str(&line));
    row.text.setTextColor(Some(&color));
    row.badge.setHidden(!deck.playing);
}

fn render(ui: &mut Ui, snap: &Snapshot, host: &SharedHost) {
    render_row(&ui.left, &snap.left);
    render_row(&ui.right, &snap.right);

    let actually_visible = ui.window.isVisible();
    if ui.ordered_visible && snap.visible && !actually_visible {
        // We showed it, the variable still says visible, but the window is
        // gone: the user clicked the close button. Write the state back so
        // `toggle '$nowplaying'` buttons (and their LEDs) stay truthful.
        let _ = host.send_command("set '$nowplaying' 0");
        ui.ordered_visible = false;
    } else if snap.visible && !actually_visible {
        ui.window.orderFront(None);
        ui.ordered_visible = true;
    } else if !snap.visible && actually_visible {
        ui.window.orderOut(None);
        ui.ordered_visible = false;
    } else {
        ui.ordered_visible = actually_visible;
    }
}
