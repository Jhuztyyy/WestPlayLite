use std::future::IntoFuture;
use std::pin::pin;
use std::sync::{Mutex, OnceLock};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::{Duration, Instant};

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

pub enum Transport {
    PlayPause,
    Next,
    Previous,
}

fn noop_waker() -> Waker {
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    fn wake(_: *const ()) {}
    fn wake_by_ref(_: *const ()) {}
    fn drop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

fn block_on_short<F: IntoFuture>(fut: F) -> Option<F::Output> {
    let mut fut = pin!(fut.into_future());
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let deadline = Instant::now() + Duration::from_millis(250);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(out) => return Some(out),
            Poll::Pending => {
                if Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
}

pub fn send_transport(cmd: Transport) {
    let Some(Ok(manager)) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .ok()
        .and_then(block_on_short)
    else {
        return;
    };
    let Ok(session) = manager.GetCurrentSession() else {
        return;
    };
    match cmd {
        Transport::PlayPause => {
            if let Ok(op) = session.TryTogglePlayPauseAsync() {
                let _ = block_on_short(op);
            }
        }
        Transport::Next => {
            if let Ok(op) = session.TrySkipNextAsync() {
                let _ = block_on_short(op);
            }
        }
        Transport::Previous => {
            if let Ok(op) = session.TrySkipPreviousAsync() {
                let _ = block_on_short(op);
            }
        }
    }
}

struct Snap {
    label: String,
    playing: bool,
    shuffle: bool,
}

static SNAP: Mutex<Option<Snap>> = Mutex::new(None);
static STARTED: OnceLock<()> = OnceLock::new();

fn refresh() -> Option<Snap> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .ok()
        .and_then(block_on_short)?
        .ok()?;
    let session = manager.GetCurrentSession().ok()?;
    let props = session
        .TryGetMediaPropertiesAsync()
        .ok()
        .and_then(block_on_short)?
        .ok()?;
    let info = session.GetPlaybackInfo().ok()?;
    let status = info.PlaybackStatus().ok()?;
    let playing = status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
    let paused = status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused;
    if !playing && !paused {
        return None;
    }

    let title = props.Title().map(|s| s.to_string()).unwrap_or_default();
    let artist = props.Artist().map(|s| s.to_string()).unwrap_or_default();
    let label = if artist.is_empty() {
        title
    } else {
        format!("{artist} - {title}")
    };
    if label.is_empty() {
        return None;
    }

    Some(Snap {
        label,
        playing,
        shuffle: info
            .IsShuffleActive()
            .ok()
            .and_then(|v| v.Value().ok())
            .unwrap_or(false),
    })
}

fn ensure_thread() {
    STARTED.get_or_init(|| {
        std::thread::spawn(|| loop {
            let snap = refresh();
            if let Ok(mut slot) = SNAP.lock() {
                *slot = snap;
            }
            std::thread::sleep(Duration::from_millis(750));
        });
    });
}

pub fn current_playback(music_player_active: bool) -> Option<Box<crate::music_folder::Playback>> {
    if music_player_active {
        return None;
    }
    ensure_thread();
    let guard = SNAP.lock().ok()?;
    let s = guard.as_ref()?;
    Some(Box::new(crate::music_folder::Playback {
        file: s.label.clone(),
        playing: s.playing,
        loading: false,
        position: 0.0,
        duration: 0.0,
        shuffle: s.shuffle,
        repeat: crate::music_folder::Repeat::Off,
        volume: None,
        index: 0,
        count: 0,
    }))
}