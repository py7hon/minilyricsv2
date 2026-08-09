use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

#[derive(Debug, Clone, Default)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub is_playing: bool,
}

fn get_current_windows_ticks() -> i64 {
    let now = std::time::SystemTime::now();
    if let Ok(duration) = now.duration_since(std::time::UNIX_EPOCH) {
        (duration.as_nanos() / 100) as i64 + 116444736000000000
    } else {
        0
    }
}

pub fn spawn_media_monitor() -> Arc<Mutex<MediaInfo>> {
    let shared = Arc::new(Mutex::new(MediaInfo::default()));
    let shared_clone = shared.clone();

    tokio::spawn(async move {
        let mut manager_opt: Option<GlobalSystemMediaTransportControlsSessionManager> = None;
        let mut last_prop_check = tokio::time::Instant::now();
        let mut current_title = String::new();
        let mut current_artist = String::new();
        let mut current_album = String::new();
        let mut current_duration_ms = 0u64;
        let mut manager_retry_at = tokio::time::Instant::now();

        loop {
            if manager_opt.is_none() && tokio::time::Instant::now() >= manager_retry_at {
                if let Ok(async_op) =
                    GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                {
                    if let Ok(m) = async_op.get() {
                        manager_opt = Some(m);
                    }
                }
                manager_retry_at = tokio::time::Instant::now() + Duration::from_secs(1);
            }

            if let Some(ref manager) = manager_opt {
                if let Ok(session) = manager.GetCurrentSession() {
                    if last_prop_check.elapsed().as_millis() > 1000 {
                        last_prop_check = tokio::time::Instant::now();
                        if let Ok(properties_async) = session.TryGetMediaPropertiesAsync() {
                            if let Ok(properties) = properties_async.get() {
                                if let (Ok(title_h), Ok(artist_h)) =
                                    (properties.Title(), properties.Artist())
                                {
                                    current_title = title_h.to_string();
                                    current_artist = artist_h.to_string();
                                    current_album = properties
                                        .AlbumTitle()
                                        .map(|a| a.to_string())
                                        .unwrap_or_default();
                                }
                            }
                        }
                    }

                    let mut position_ms = 0u64;
                    let mut is_playing = false;

                    if let Ok(timeline) = session.GetTimelineProperties() {
                        if let Ok(end) = timeline.EndTime() {
                            current_duration_ms = (end.Duration / 10_000) as u64;
                        }

                        if let Ok(position_ticks) = timeline.Position() {
                            let base_ms = (position_ticks.Duration / 10_000) as u64;

                            is_playing = if let Ok(info) = session.GetPlaybackInfo() {
                                if let Ok(status) = info.PlaybackStatus() {
                                    status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            position_ms = if is_playing {
                                if let Ok(last_updated) = timeline.LastUpdatedTime() {
                                    let now_ticks = get_current_windows_ticks();
                                    let elapsed_ticks = now_ticks - last_updated.UniversalTime;
                                    if elapsed_ticks > 0 && elapsed_ticks < 86_400_000_000_000 {
                                        base_ms + (elapsed_ticks / 10_000) as u64
                                    } else {
                                        base_ms
                                    }
                                } else {
                                    base_ms
                                }
                            } else {
                                base_ms
                            };
                        }
                    }

                    if let Ok(mut info) = shared_clone.lock() {
                        info.title = current_title.clone();
                        info.artist = current_artist.clone();
                        info.album = current_album.clone();
                        info.duration_ms = current_duration_ms;
                        info.position_ms = position_ms;
                        info.is_playing = is_playing;
                    }
                } else {
                    current_title.clear();
                    current_artist.clear();
                    current_album.clear();
                    current_duration_ms = 0;
                    if let Ok(mut info) = shared_clone.lock() {
                        *info = MediaInfo::default();
                    }
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
    });

    shared
}
