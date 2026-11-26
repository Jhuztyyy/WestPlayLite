fn main() {
    // The app bundles the built UI (`ui/dist`, tauri.conf.json's resources), which `tauri build` makes first
    // (`npm run build:release`) and which isn't in the repository. Without it, `cargo check` or `cargo test`
    // on a fresh clone would fail here: an empty folder stands in until the UI is built.
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/dist");
    if !dist.exists() {
        std::fs::create_dir_all(&dist).expect("creating ui/dist");
    }
    // Every command the companion calls, named here so each gets an `allow-<command>` permission that
    // capabilities/default.json grants (to the local page and to the tauri.localhost / dev-server origins).
    // Keep this list equal to `generate_handler!` in src/main.rs: a command missing here is refused with
    // "Command <name> not allowed by ACL".
    const COMMANDS: &[&str] = &[
        "companion_ready",
        "window_in_front",
        "room_held",
        "set_westplay_open",
        "youtube_sign_in",
        "youtube_sign_out",
        "youtube_check",
        "spotify_sign_in",
        "spotify_sign_out",
        "spotify_check",
        "apple_music_sign_in",
        "apple_music_sign_out",
        "apple_music_check",
        "open_support_page",
        "open_social",
        "app_log_place",
        "customize_list",
        "add_wallpapers",
        "add_icon_set",
        "remove_wallpaper",
        "remove_icon_set",
        "update_offer",
        "look_for_update",
        "install_update",
        "get_settings",
        "update_settings",
        "key_name",
        "setup_state",
        "game_running",
        "prepare_maps_again",
        "choose_game_folder",
        "check_game",
        "install_into_game",
        "finish_setup",
        "uninstall_from_game",
        "remove_data",
        "leave_pc",
    ];
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
