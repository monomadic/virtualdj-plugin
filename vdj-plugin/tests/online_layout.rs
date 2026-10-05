use std::collections::HashMap;
use std::mem::{offset_of, size_of};
use std::path::PathBuf;
use std::process::Command;
use vdj_plugin::wrapper::{OnlineFields, OnlineSlots};

#[test]
fn online_source_matches_sdk() {
    let sdk = std::env::var_os("VDJ_SDK")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../virtualdj-beetport/sdk")
        });
    if !sdk.join("vdjOnlineSource.h").exists() {
        assert_eq!(
            std::env::var("VDJ_SDK_OPTIONAL").as_deref(),
            Ok("1"),
            "OnlineSource SDK header missing"
        );
        eprintln!("SKIP: OnlineSource SDK header missing");
        return;
    }
    let exe = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("online_layout_ref");
    let out = Command::new("clang++")
        .args(["-std=c++17", "-Wno-invalid-offsetof", "-I"])
        .arg(&sdk)
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/online_layout_ref.cpp"))
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = Command::new(exe).output().unwrap();
    assert!(out.status.success());
    let values: HashMap<_, _> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| {
            let (k, v) = line.split_once(' ').unwrap();
            (k.to_owned(), v.parse::<usize>().unwrap())
        })
        .collect();
    let get = |k: &str| *values.get(k).unwrap();
    assert_eq!(get("size"), 32);
    assert_eq!(get("oauth_offset"), 24);
    assert_eq!(size_of::<OnlineFields>(), 8);
    for (name, offset) in [
        ("is_logged", offset_of!(OnlineSlots, is_logged)),
        ("on_oauth", offset_of!(OnlineSlots, on_oauth)),
        ("on_search", offset_of!(OnlineSlots, on_search)),
        (
            "on_search_cancel",
            offset_of!(OnlineSlots, on_search_cancel),
        ),
        ("get_stream_url", offset_of!(OnlineSlots, get_stream_url)),
        ("get_folder_list", offset_of!(OnlineSlots, get_folder_list)),
        ("get_folder", offset_of!(OnlineSlots, get_folder)),
        (
            "get_context_menu",
            offset_of!(OnlineSlots, get_context_menu),
        ),
        ("on_context_menu", offset_of!(OnlineSlots, on_context_menu)),
        (
            "get_folder_context_menu",
            offset_of!(OnlineSlots, get_folder_context_menu),
        ),
        (
            "on_folder_context_menu",
            offset_of!(OnlineSlots, on_folder_context_menu),
        ),
    ] {
        assert_eq!(get(name), offset / 8, "{name}");
    }
    assert_eq!(get("tracks_add"), 0);
    assert_eq!(get("tracks_finish"), 1);
    assert_eq!(get("string_assign"), 0);
}
