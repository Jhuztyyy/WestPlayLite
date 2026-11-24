use std::future::IntoFuture;
use std::pin::pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::{Duration, Instant};

use base64::Engine as _;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession,
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Storage::Streams::{DataReader, IRandomAccessStreamWithContentType};

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

pub fn current_playback(music_player_active: bool) -> Option<Box<crate::music_folder::Playback>> {
    if music_player_active {
        return None;
    }
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .ok()
        .and_then(block_on_short)?
        .ok()?;
    let session: GlobalSystemMediaTransportControlsSession = manager.GetCurrentSession().ok()?;
    let props = session.TryGetMediaPropertiesAsync().ok().and_then(block_on_short)?.ok()?;
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

    let _cover_data_url = props.Thumbnail().ok().and_then(|stream_ref| {
        let stream: IRandomAccessStreamWithContentType =
            stream_ref.OpenReadAsync().ok().and_then(block_on_short)?.ok()?;
        let size = stream.Size().ok()? as u32;
        if size == 0 || size > 2_000_000 {
            return None;
        }
        let input = stream.GetInputStreamAt(0).ok()?;
        let reader = DataReader::CreateDataReader(&input).ok()?;
        let _ = block_on_short(reader.LoadAsync(size).ok()?)?.ok()?;
        let mut buf = vec![0u8; size as usize];
        reader.ReadBytes(&mut buf).ok()?;
        Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(buf)
        ))
    });

    Some(Box::new(crate::music_folder::Playback {
        file: label,
        playing,
        loading: false,
        position: 0.0,
        duration: 0.0,
        shuffle: info.IsShuffleActive().ok().and_then(|v| v.Value().ok()).unwrap_or(false),
        repeat: crate::music_folder::Repeat::Off,
        volume: None,
        index: 0,
        count: 0,
    }))
}