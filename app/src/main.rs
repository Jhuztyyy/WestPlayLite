//! WestPlay app: the companion window (Tauri) plus the in-cabin UI host on its own thread.

// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod customize;
mod maps;
mod setup;
mod update;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use setup::SetupStore;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use westplay_host::maps::SetUpGame;
use westplay_host::settings::{Settings, SettingsFile, SharedSettings};
use westplay_host::{
    AccountAction, AccountRequests, BrowserGone, Config, Content, OpenRequest, SCREEN_HEIGHT, SCREEN_WIDTH, Status,
};

struct Host {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// Requests from the companion window to open or close WestPlay on the truck's screen (the cabin page owns
/// that state, which it also changes itself: its sidebar's truck button, its WestPlay button, the interaction
/// key and W; the host reports it back in its status).
struct WestplayOpen(Arc<OpenRequest>);

/// The companion's sign-ins and sign-outs (YouTube, Spotify, Apple Music), for the host.
struct Accounts(Arc<AccountRequests>);

/// The player's settings: kept on disk, applied live by the host thread, changed from the companion window.
struct SettingsStore {
    file: SettingsFile,
    shared: Arc<SharedSettings>,
}

/// The companion window starts hidden and shows once its page has drawn its first screen
/// (`companion_ready`), so it never shows blank while the page loads; after this long it shows anyway.
const SHOW_ANYWAY: Duration = Duration::from_secs(8);

/// The companion's page has drawn its first screen: the window shows.
#[tauri::command]
fn companion_ready(window: WebviewWindow) {
    if !window.is_visible().unwrap_or(false) {
        westplay_host::startup::mark("app: window shown");
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Whether the companion's window is the one in front. Windows' word: the page's own `document.hasFocus()` says
/// yes while the window comes up behind another.
#[tauri::command]
fn window_in_front(window: WebviewWindow) -> bool {
    window.hwnd().is_ok_and(|window| westplay_host::in_front(window.0 as isize))
}

/// The companion's page held its room still, or set it going again: for a log to tell what the room cost.
#[tauri::command]
fn room_held(held: bool) {
    westplay_host::log!("companion: {}", if held { "behind, its room held still" } else { "in front, its room going" });
}

#[tauri::command]
fn set_westplay_open(open: bool, request: State<WestplayOpen>) {
    request.0.request(open);
}

/// Opens Google's sign-in for YouTube in a window on the desktop (the status then says when it's done).
#[tauri::command]
fn youtube_sign_in(accounts: State<Accounts>) {
    accounts.0.youtube.request(AccountAction::SignIn);
}

/// Signs YouTube out.
#[tauri::command]
fn youtube_sign_out(accounts: State<Accounts>) {
    accounts.0.youtube.request(AccountAction::SignOut);
}

/// Runs YouTube (hidden), so the status says whether it's signed in.
#[tauri::command]
fn youtube_check(accounts: State<Accounts>) {
    accounts.0.youtube.request(AccountAction::Check);
}

/// Opens Spotify's sign-in in a window on the desktop (the status then says when it's done).
#[tauri::command]
fn spotify_sign_in(accounts: State<Accounts>) {
    accounts.0.spotify.request(AccountAction::SignIn);
}

/// Signs Spotify out.
#[tauri::command]
fn spotify_sign_out(accounts: State<Accounts>) {
    accounts.0.spotify.request(AccountAction::SignOut);
}

/// Runs Spotify's web player (hidden), so the status says whether it's signed in.
#[tauri::command]
fn spotify_check(accounts: State<Accounts>) {
    accounts.0.spotify.request(AccountAction::Check);
}

/// Opens Apple's sign-in for Apple Music in a window on the desktop (the status then says when it's done).
#[tauri::command]
fn apple_music_sign_in(accounts: State<Accounts>) {
    accounts.0.apple_music.request(AccountAction::SignIn);
}

/// Signs Apple Music out.
#[tauri::command]
fn apple_music_sign_out(accounts: State<Accounts>) {
    accounts.0.apple_music.request(AccountAction::SignOut);
}

/// Runs Apple Music's web player (hidden), so the status says whether it's signed in.
#[tauri::command]
fn apple_music_check(accounts: State<Accounts>) {
    accounts.0.apple_music.request(AccountAction::Check);
}

/// The maker's Kreosus page, where players support WestPlay's development.
const SUPPORT_PAGE: &str = "https://kreosus.com/enes-less";

/// Opens the support page (Support me) in the player's browser. The address is fixed here, so the companion
/// can't open any other.
#[tauri::command]
fn open_support_page() {
    westplay_host::browser::open(SUPPORT_PAGE);
}

/// WestPlay's pages elsewhere, at the foot of the companion's list.
#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum Social {
    YouTube,
    Discord,
    Patreon,
}

/// Opens one of WestPlay's pages elsewhere (its YouTube channel, its Discord, its Patreon) in the player's browser.
/// The addresses are fixed here, as the support page's.
#[tauri::command]
fn open_social(site: Social) {
    westplay_host::browser::open(match site {
        Social::YouTube => "https://www.youtube.com/@tyraness",
        Social::Discord => "https://discord.com/invite/Jt34SmwU6",
        Social::Patreon => "https://www.patreon.com/c/westmods",
    });
}

/// Where this run's `app.log` is (the About page): Documents\WestPlay\logs, or WestPlay's own data folder where
/// Documents refused it; `None` if it couldn't be made.
#[tauri::command]
fn app_log_place() -> Option<String> {
    westplay_host::log::place().map(|path| path.display().to_string())
}

/// A keyboard key's name for the player's keyboard, by its virtual-key code (the keybinds' labels); `None`
/// for a mouse button or a key without one.
#[tauri::command]
fn key_name(key: u32) -> Option<String> {
    westplay_host::settings::KeyCode::name(key)
}

#[tauri::command]
fn get_settings(store: State<SettingsStore>) -> Settings {
    store.shared.get()
}

/// Saves new settings and applies them. Values are corrected on the way in (see `westplay_host::settings`);
/// the settings as saved are returned and announced to every window (`settings-changed`).
#[tauri::command]
fn update_settings(settings: Settings, store: State<SettingsStore>, app: AppHandle) -> Result<Settings, String> {
    store.file.save(&settings).map_err(|e| format!("Couldn't save the settings: {e}"))?;
    westplay_host::graphics::set_game_first(settings.graphics.game_first);
    set_cast(&app, settings.graphics.youtube_cast);
    store.shared.set(settings.clone());
    let _ = app.emit("settings-changed", &settings);
    Ok(settings)
}

/// WestPlay Lite's cast target (westplay-cast), while it's on.
#[derive(Default)]
struct CastState(std::sync::Mutex<Option<westplay_cast::Handle>>);

/// Turns the cast target on or off, as the setting says (a change only: on again when it's on already does nothing).
fn set_cast(app: &AppHandle, on: bool) {
    let Some(state) = app.try_state::<CastState>() else { return };
    let Ok(mut cast) = state.0.lock() else { return };
    if on == cast.is_some() {
        return;
    }
    if !on {
        *cast = None;
        westplay_host::log!("[cast] off");
        return;
    }
    let handle = app.clone();
    let report = move |event: westplay_cast::Event| {
        // Searches repeat every few seconds from every TV on the network: not logged.
        if !matches!(event, westplay_cast::Event::Search { .. }) {
            westplay_host::log!("[cast] {event:?}");
        }
        match &event {
            westplay_cast::Event::Launch { body, .. } => open_cast_window(&handle, body),
            westplay_cast::Event::Stop { .. } => {
                if let Some(window) = handle.get_webview_window("cast") {
                    let _ = window.close();
                }
            }
            _ => {}
        }
    };
    match westplay_cast::start("WestPlay", report) {
        Ok(handle) => {
            *cast = Some(handle);
            westplay_host::log!("[cast] on: WestPlay shows in a phone's cast list");
        }
        Err(error) => westplay_host::log!("[cast] couldn't start: {error}"),
    }
}

/// Whether the cast window is shown as a window (to see what the TV page does). Not shown, it's a window the system
/// still counts as in sight, so the TV page keeps running and keeps its phone connected, but one nobody sees: no frame,
/// not on the taskbar, behind everything, never focused (the game stays in front), its corner the only pixel on the
/// screen. A hidden window never paints, and a TV page that doesn't paint can stall.
const CAST_WINDOW_SHOWN: bool = false;

/// Pauses the cast window's video for the game's pause (marking the one it paused) and lets it play on after.
const CAST_HOLD: &str = "document.querySelectorAll('video').forEach(function (v) { if (!v.paused) { v.muted = true; v.pause(); v.dataset.wpHeld = '1'; } });";
const CAST_RELEASE: &str = "document.querySelectorAll('video').forEach(function (v) { if (v.dataset.wpHeld === '1') { delete v.dataset.wpHeld; v.muted = true; var p = v.play(); if (p && p.catch) p.catch(function () {}); } });";

/// The user agent of a smart TV (Samsung's Tizen): YouTube's TV page is for these.
const CAST_USER_AGENT: &str = "Mozilla/5.0 (SMART-TV; Linux; Tizen 6.0) AppleWebKit/537.36 (KHTML, like Gecko) 76.0.3809.146/6.0 TV Safari/537.36";

/// Run before YouTube's own scripts in the cast window. VP9 and AV1 are said not to be supported, so YouTube sends
/// H.264, which a weak PC's GPU decodes (as the h264ify extension does). And the window is silent: the truck's
/// screen plays the video, this window only receives the phone's commands.
const CAST_SCRIPT: &str = r#"(function () {
  var avoid = /vp0?9|av0?1/i;
  try {
    var media = window.MediaSource;
    if (media && media.isTypeSupported) {
      var supported = media.isTypeSupported.bind(media);
      media.isTypeSupported = function (type) { return avoid.test(String(type)) ? false : supported(type); };
    }
    var proto = window.HTMLMediaElement && window.HTMLMediaElement.prototype;
    if (proto && proto.canPlayType) {
      var canPlay = proto.canPlayType;
      proto.canPlayType = function (type) { return avoid.test(String(type)) ? '' : canPlay.call(this, type); };
    }
  } catch (error) {}
  // The page is told it's in sight, hidden or not: a TV page that sees itself hidden may pause what it plays.
  try {
    Object.defineProperty(document, 'hidden', { configurable: true, get: function () { return false; } });
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: function () { return 'visible'; } });
    document.addEventListener('visibilitychange', function (event) { event.stopImmediatePropagation(); }, true);
  } catch (error) {}
  // The name the phone shows once connected: the TV page names itself after the TV it takes itself for (Samsung's
  // Tizen: "Samsung SmartTV", as a name, or as a brand and a model) when it tells YouTube's lounge, and the lounge
  // tells the phone. It goes out as WestPlay Lite. What the lounge was told is on the address (`wpdbg`), for the log.
  var seen = [];
  function named(text) {
    if (typeof text !== 'string') return text;
    return text
      .replace(/Samsung(?:\+|%20| )SmartTV/gi, 'WestPlay+Lite')
      .replace(/Samsung/g, 'WestPlay')
      .replace(/SmartTV/g, 'Lite');
  }
  function lounge(url) { return typeof url === 'string' && url.indexOf('/lounge/') >= 0; }
  function peek(url, body) {
    try {
      var keep = /(?:^|[&?])(name|device|deviceInfo|app|brand|model)=([^&]*)/g;
      var found = [], match, text = String(url).split('?')[1] + '&' + String(body);
      while ((match = keep.exec(text)) && found.length < 6) found.push(match[1] + '=' + match[2].slice(0, 120));
      var line = found.join(' ');
      if (line && seen.indexOf(line) < 0 && seen.length < 3) seen.push(line);
    } catch (error) {}
  }
  try {
    var open = XMLHttpRequest.prototype.open;
    var send = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function (method, url) {
      this.westplayLounge = lounge(String(url));
      if (this.westplayLounge) { this.westplayUrl = named(String(url)); arguments[1] = this.westplayUrl; }
      return open.apply(this, arguments);
    };
    XMLHttpRequest.prototype.send = function (body) {
      if (!this.westplayLounge) return send.call(this, body);
      var renamed = named(body);
      peek(this.westplayUrl, renamed);
      return send.call(this, renamed);
    };
    var fetched = window.fetch;
    if (fetched) {
      window.fetch = function (input, init) {
        var url = typeof input === 'string' ? input : input && input.url;
        if (lounge(url)) {
          if (typeof input === 'string') input = named(input);
          if (init && typeof init.body === 'string') init = Object.assign({}, init, { body: named(init.body) });
          peek(typeof input === 'string' ? input : url, init && init.body);
        }
        return fetched.call(this, input, init);
      };
    }
  } catch (error) {}
  document.addEventListener('play', function (event) { if (event.target) event.target.muted = true; }, true);
  // The video's state on the address (`?wp=<1 paused or 0 playing>.<seconds>`, no page load): WestPlay sees nothing
  // else of this window, and the phone's pausing, playing and seeking reach the truck's player by it.
  setInterval(function () {
    try {
      var video = document.querySelector('video');
      var address = new URL(location.href);
      var changed = false;
      if (video) {
        var mark = (video.paused ? '1' : '0') + '.' + (Math.round(video.currentTime * 10) / 10);
        if (address.searchParams.get('wp') !== mark) { address.searchParams.set('wp', mark); changed = true; }
      }
      var debug = seen.join(' | ');
      if (debug && address.searchParams.get('wpdbg') !== debug) { address.searchParams.set('wpdbg', debug); changed = true; }
      if (changed) history.replaceState(history.state, '', address.pathname + address.search + address.hash);
    } catch (error) {}
  }, 500);
})();"#;

/// The state the cast window's script marks on its own address (`?wp=<1 paused or 0 playing>.<seconds>`, set with
/// `history.replaceState`): the one way the page's state reaches WestPlay, as its address is all WestPlay can see.
fn cast_mark(url: &tauri::Url) -> Option<(bool, f64)> {
    let value = url.query_pairs().find(|(name, _)| name == "wp")?.1.into_owned();
    let (paused, time) = value.split_once('.')?;
    let time = time.parse::<f64>().ok().filter(|time| time.is_finite() && *time >= 0.0)?;
    Some((paused == "1", time))
}

/// The address as the TV page itself has it: without the mark above.
fn without_cast_mark(url: &tauri::Url) -> String {
    let mut plain = url.clone();
    let kept: Vec<(String, String)> =
        url.query_pairs().filter(|(name, _)| name != "wp").map(|(name, value)| (name.into_owned(), value.into_owned())).collect();
    if kept.is_empty() {
        plain.set_query(None);
    } else {
        plain.query_pairs_mut().clear().extend_pairs(kept.iter().map(|(name, value)| (name.as_str(), value.as_str())));
    }
    plain.to_string()
}

/// What the phone does to the video, as the cast window shows it, turned into what the truck's player is to do.
#[derive(Default)]
struct CastMirror {
    paused: Option<bool>,
    time: f64,
    at: Option<std::time::Instant>,
}

impl CastMirror {
    /// `Some((paused, seconds, seek))` when the truck's player is to pause or play, or (`seek`) to move to `seconds`:
    /// the phone paused or played, or moved within the video. Not the video's own running on.
    fn update(&mut self, paused: bool, time: f64) -> Option<(bool, f64, bool)> {
        let now = std::time::Instant::now();
        // A video that hasn't started (paused at its start) says nothing: the truck's player is already loading it.
        if self.paused.is_none() && paused {
            self.paused = Some(true);
            self.time = time;
            self.at = Some(now);
            return None;
        }
        let expected = match (self.paused, self.at) {
            (Some(false), Some(at)) => self.time + now.duration_since(at).as_secs_f64(),
            (Some(true), _) => self.time,
            _ => time,
        };
        let changed = self.paused != Some(paused);
        let jumped = self.paused.is_some() && (time - expected).abs() > 2.5;
        self.paused = Some(paused);
        self.time = time;
        self.at = Some(now);
        (changed || jumped).then_some((paused, time, jumped))
    }
}

/// The video the TV page plays, from its address (`https://www.youtube.com/tv#/watch?...&v=<id>`): `None` before one
/// plays. An id is 11 letters, digits, `-` and `_`: nothing else passes (it goes into a message to the page).
fn cast_video_id(url: &tauri::Url) -> Option<String> {
    let query = url.fragment()?.split_once('?')?.1;
    let video = query.split('&').find_map(|pair| pair.strip_prefix("v="))?;
    (video.len() == 11 && video.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')).then(|| video.to_string())
}

/// Follows the cast window while it's open: the address it's on goes to the log when it changes (the TV page puts
/// what it plays in its address, `#/watch?...&v=<id>`), the video it plays goes to the truck's screen, and the phone's
/// pausing, playing and seeking (marked on the address by the window's script) go to the truck's player.
fn watch_cast_address(app: AppHandle) {
    let _ = std::thread::Builder::new().name("westplay-cast-address".into()).spawn(move || {
        let mut last = String::new();
        let mut video: Option<String> = None;
        let mut mirror = CastMirror::default();
        let mut game_paused = false;
        loop {
            let Some(window) = app.get_webview_window("cast") else { break };
            // The game paused: the video in the cast window pauses too (or it plays on to its end and brings the
            // phone's next video while the game is stopped), and plays on with the game, if it was playing.
            let paused = westplay_host::GAME_PAUSED.load(std::sync::atomic::Ordering::Relaxed);
            if paused != game_paused {
                game_paused = paused;
                let _ = window.eval(if paused { CAST_HOLD } else { CAST_RELEASE });
            }
            if let Ok(url) = window.url() {
                let plain = without_cast_mark(&url);
                if plain != last {
                    westplay_host::log!("[cast] page: {plain}");
                    last = plain;
                }
                if let Some(id) = cast_video_id(&url)
                    && video.as_deref() != Some(id.as_str())
                {
                    westplay_host::log!("[cast] the phone chose a video: {id}");
                    video = Some(id.clone());
                    mirror = CastMirror::default();
                    if let Ok(mut slot) = westplay_host::CAST_VIDEO.lock() {
                        *slot = Some(id);
                    }
                    if let Some(open) = app.try_state::<WestplayOpen>() {
                        open.0.request(true);
                    }
                }
                if video.is_some()
                    && let Some((paused, time)) = cast_mark(&url)
                    && let Some(change) = mirror.update(paused, time)
                    && let Ok(mut slot) = westplay_host::CAST_STATE.lock()
                {
                    *slot = Some(change);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });
}

/// A phone cast to WestPlay (`pairing`: what it sent, `pairingCode=...&theme=cl&dialLaunch=watch`): YouTube's TV page
/// opens with it in a window of its own, as it does on a TV, and the phone pairs with it. A test: it plays in this
/// window, not yet on the truck's screen.
fn open_cast_window(app: &AppHandle, pairing: &str) {
    // The phone's words go into an address: only the characters a pairing request has are let through.
    if pairing.is_empty() || !pairing.chars().all(|c| c.is_ascii_alphanumeric() || "=&_%.-".contains(c)) {
        westplay_host::log!("[cast] a launch that isn't a pairing request, not opened");
        return;
    }
    let address = format!("https://www.youtube.com/tv?{pairing}");
    if let Some(window) = app.get_webview_window("cast") {
        let _ = window.eval(&format!("window.location.replace('{address}')"));
        if CAST_WINDOW_SHOWN {
            let _ = window.show();
            let _ = window.set_focus();
        }
        return;
    }
    let Ok(url) = address.parse::<tauri::Url>() else { return };
    match WebviewWindowBuilder::new(app, "cast", WebviewUrl::External(url))
        .title("WestPlay cast")
        .decorations(CAST_WINDOW_SHOWN)
        .skip_taskbar(!CAST_WINDOW_SHOWN)
        .always_on_bottom(!CAST_WINDOW_SHOWN)
        .focused(CAST_WINDOW_SHOWN)
        .resizable(CAST_WINDOW_SHOWN)
        .shadow(CAST_WINDOW_SHOWN)
        // Small, and so is the video YouTube sends it: the truck's screen plays it, this window only listens.
        .inner_size(256.0, 144.0)
        .user_agent(CAST_USER_AGENT)
        .initialization_script(CAST_SCRIPT)
        .build()
    {
        Ok(window) => {
            // Its sound stops at the browser, not by a script: nothing the page does brings it back.
            let _ = window.with_webview(|webview| westplay_host::mute_webview(&webview.controller()));
            if !CAST_WINDOW_SHOWN
                && let Ok(Some(monitor)) = window.primary_monitor()
            {
                // Off the screen but for its one corner pixel.
                let (at, size) = (monitor.position(), monitor.size());
                let _ = window.set_position(tauri::PhysicalPosition::new(
                    at.x + size.width as i32 - 1,
                    at.y + size.height as i32 - 1,
                ));
            }
            westplay_host::log!("[cast] YouTube's TV page opened for the phone");
            watch_cast_address(app.clone());
        }
        Err(error) => westplay_host::log!("[cast] couldn't open YouTube's TV page: {error}"),
    }
}

fn main() {
    // WestPlay Lite: only smooth scrolling off. (--renderer-process-limit and --enable-low-end-device-mode applied to
    // every WebView2 of the process, the host's hidden cab, Spotify and YouTube pages included.)
    if std::env::var_os("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").is_none() {
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_var(
                "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
                "--disable-smooth-scrolling",
            );
        }
    }
    // Started as administrator only to copy the plugin into the game (setup asked for it): that, and done.
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = westplay_setup::plugin::run_as_administrator_copy(&args) {
        std::process::exit(code);
    }
    // One WestPlay at a time: a second only says so (held until the app ends). Before the log starts: a second
    // making the log anew would empty the running one's.
    let Some(_instance) = westplay_host::Instance::claim() else {
        westplay_host::tell_already_running();
        return;
    };
    westplay_host::log::init();
    westplay_host::startup::mark("app: started");
    let open_request = Arc::new(OpenRequest::default());
    let accounts = Arc::new(AccountRequests::default());
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(update::Updates::default())
        .manage(WestplayOpen(open_request.clone()))
        .manage(Accounts(accounts.clone()))
        .register_asynchronous_uri_scheme_protocol(maps::SCHEME, maps::serve)
        .register_asynchronous_uri_scheme_protocol(maps::CUSTOMIZE_SCHEME, maps::serve_customize)
        .invoke_handler(tauri::generate_handler![
            companion_ready,
            window_in_front,
            room_held,
            set_westplay_open,
            youtube_sign_in,
            youtube_sign_out,
            youtube_check,
            spotify_sign_in,
            spotify_sign_out,
            spotify_check,
            apple_music_sign_in,
            apple_music_sign_out,
            apple_music_check,
            open_support_page,
            open_social,
            app_log_place,
            customize::customize_list,
            customize::add_wallpapers,
            customize::add_icon_set,
            customize::remove_wallpaper,
            customize::remove_icon_set,
            update::update_offer,
            update::look_for_update,
            update::install_update,
            get_settings,
            update_settings,
            key_name,
            setup::setup_state,
            setup::game_running,
            setup::choose_game_folder,
            setup::check_game,
            setup::install_into_game,
            setup::finish_setup,
            setup::uninstall_from_game,
            setup::remove_data,
            setup::leave_pc
        ])
        .setup(move |app| {
            westplay_host::startup::mark("app: window set up");
            // A page that never says it's ready (it failed to load) still shows, rather than nothing at all.
            if let Some(window) = app.get_webview_window("main") {
                std::thread::spawn(move || {
                    std::thread::sleep(SHOW_ANYWAY);
                    if !window.is_visible().unwrap_or(true) {
                        westplay_host::startup::mark("app: window shown without its page ready");
                        let _ = window.show();
                    }
                });
            }
            let config_dir = app.path().app_config_dir()?;
            let file = SettingsFile::in_dir(&config_dir);
            let loaded = file.load();
            // WestPlay Lite: before the host's browsers start, which take the priority from here.
            westplay_host::graphics::set_game_first(loaded.graphics.game_first);
            app.manage(CastState::default());
            set_cast(app.handle(), loaded.graphics.youtube_cast);
            let shared = Arc::new(SharedSettings::new(loaded));
            let setup = SetupStore::load(&config_dir, app.handle());
            setup::watch_installed_mods(&setup);
            let customize = customize::Customize {
                dir: app.path().app_local_data_dir()?.join(westplay_host::customize::FOLDER),
                changed: Arc::default(),
            };
            let host = start_host(
                app.handle(),
                open_request.clone(),
                accounts.clone(),
                shared.clone(),
                setup.game(),
                Looks { config_dir: config_dir.clone(), customize_changed: customize.changed.clone() },
            )?;
            app.manage(customize);
            app.manage(SettingsStore { file, shared });
            app.manage(setup);
            app.manage(std::sync::Mutex::new(host));
            update::watch(app.handle());
            // WestPlay Lite: the companion's X puts it in the tray (the icons by the clock) and the host goes on.
            let open = MenuItem::with_id(app, "open", "Open WestPlay", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit WestPlay", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("WestPlay")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_companion(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_companion(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build the WestPlay app");

    app.run(|app, event| match event {
        // The X of the companion window hides it to the tray; "Quit WestPlay" there ends the app.
        RunEvent::WindowEvent { label, event: WindowEvent::CloseRequested { api, .. }, .. } if label == "main" => {
            api.prevent_close();
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        RunEvent::Exit => stop_host(app),
        _ => {}
    });
}

/// Brings the companion window back from the tray.
fn show_companion(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Stops the host, so its WebView2 shuts down cleanly with the app (it quitting, or an update's installer
/// about to replace it). Once: a stopped host has nothing left to stop.
fn stop_host(app: &AppHandle) {
    if let Some(host) = app.try_state::<std::sync::Mutex<Host>>() {
        let mut host = host.lock().unwrap_or_else(|e| e.into_inner());
        host.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = host.thread.take() {
            let _ = thread.join();
        }
    }
}

/// What the host needs for the screen's look: where the settings are kept (the cab changes its wallpaper
/// itself), and the count of changes to the player's wallpapers and icon sets.
struct Looks {
    config_dir: PathBuf,
    customize_changed: Arc<std::sync::atomic::AtomicU64>,
}

/// How a run of the host ended.
enum Ended {
    Stopped,
    BrowserGone,
    /// To draw the cab on another GPU (westplay_host::graphics).
    GpuChanged,
    Failed(String),
}

/// The host is started again after its browser fails at most this often within the window, after a pause.
const MAX_RESTARTS: usize = 5;
const RESTART_WINDOW: Duration = Duration::from_secs(60);
const RESTART_DELAY: Duration = Duration::from_secs(1);
/// How long WebView2's browsers are waited for to go before the host starts on another GPU.
const BROWSERS_GOING: Duration = Duration::from_secs(10);

fn start_host(
    app: &AppHandle,
    open_request: Arc<OpenRequest>,
    accounts: Arc<AccountRequests>,
    settings: Arc<SharedSettings>,
    game: Arc<SetUpGame>,
    looks: Looks,
) -> Result<Host, Box<dyn std::error::Error>> {
    let content = cabin_content(app);
    let data_dir = app.path().app_local_data_dir()?;
    let map_builder = setup::map_builder_file(app);
    westplay_host::log!(
        "maps: {}",
        map_builder
            .as_ref()
            .map_or("no map builder: built in WestPlay itself".into(), |file| format!("built by {}", file.display()))
    );
    let stop = Arc::new(AtomicBool::new(false));

    let events = app.clone();
    let thread_stop = stop.clone();
    // The host runs on a thread of its own, started again (on a new thread) if WebView2's browser process
    // exits under it (westplay_host::BrowserGone): the cabin page comes back rather than staying blank. Not
    // endlessly: a browser failing again and again is reported instead.
    let thread = std::thread::Builder::new().name("westplay-host-supervisor".into()).spawn(move || {
        let mut restarts: Vec<Instant> = Vec::new();
        loop {
            let (content, data_dir) = (content.clone(), data_dir.clone());
            let (open_request, settings, stop) = (open_request.clone(), settings.clone(), thread_stop.clone());
            let (accounts, game, map_builder) = (accounts.clone(), game.clone(), map_builder.clone());
            let status_events = events.clone();
            let (settings_events, settings_dir) = (events.clone(), looks.config_dir.clone());
            let customize_changed = looks.customize_changed.clone();
            let run = std::thread::Builder::new().name("westplay-host".into()).spawn(move || {
                let config = Config {
                    width: SCREEN_WIDTH,
                    height: SCREEN_HEIGHT,
                    content,
                    data_dir,
                    open_request,
                    accounts,
                    settings,
                    game,
                    map_builder,
                    on_status: Box::new(move |status| {
                        let _ = status_events.emit("host-status", status);
                    }),
                    // The cab's own Settings changed one (its wallpaper): kept, and the companion told.
                    on_settings: Box::new(move |settings| {
                        if let Err(error) = SettingsFile::in_dir(&settings_dir).save(settings) {
                            westplay_host::log!("[settings] couldn't save the cab's change: {error}");
                        }
                        let _ = settings_events.emit("settings-changed", settings);
                    }),
                    customize_changed,
                };
                match westplay_host::run(config, stop) {
                    Ok(()) => Ended::Stopped,
                    Err(e) if e.is::<BrowserGone>() => Ended::BrowserGone,
                    Err(e) if e.is::<westplay_host::graphics::GpuChanged>() => Ended::GpuChanged,
                    Err(e) => Ended::Failed(e.to_string()),
                }
            });
            let ended = match run.map(|thread| thread.join()) {
                Ok(Ok(ended)) => ended,
                Ok(Err(_)) => Ended::Failed("it crashed".into()),
                Err(e) => Ended::Failed(e.to_string()),
            };
            let error = match ended {
                Ended::Stopped => return,
                Ended::BrowserGone | Ended::GpuChanged if thread_stop.load(Ordering::Relaxed) => return,
                // Started again on the other GPU once the browsers have gone (one still running would refuse
                // the new arguments). Not counted as a failure.
                Ended::GpuChanged => {
                    westplay_host::wait_for_browsers(BROWSERS_GOING);
                    continue;
                }
                Ended::BrowserGone => {
                    let now = Instant::now();
                    restarts.retain(|&at| now.duration_since(at) < RESTART_WINDOW);
                    restarts.push(now);
                    if restarts.len() <= MAX_RESTARTS {
                        std::thread::sleep(RESTART_DELAY);
                        continue;
                    }
                    BrowserGone.to_string()
                }
                Ended::Failed(error) => error,
            };
            // In the log too: shown only, a player's said nothing of it (2026-11-11).
            westplay_host::log!("WestPlay engine stopped: {error}");
            let status = Status { error: Some(format!("WestPlay engine stopped: {error}")), ..Default::default() };
            let _ = events.emit("host-status", &status);
            return;
        }
    })?;
    Ok(Host { stop, thread: Some(thread) })
}

/// The cabin page: from the Vite dev server while developing, from the bundled UI otherwise.
fn cabin_content(app: &AppHandle) -> Content {
    if cfg!(debug_assertions)
        && let Some(dev_url) = &app.config().build.dev_url
    {
        return Content::Url(format!("{}cabin.html", dev_url.as_str()));
    }
    let bundled =
        app.path().resource_dir().map(|dir| dir.join("ui")).ok().filter(|dir| dir.join("cabin.html").exists());
    // Unbundled release builds (`cargo build --release`) use the UI build next to the sources.
    let root = bundled.unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/dist"));
    Content::Folder { root, page: "cabin.html".into() }
}
