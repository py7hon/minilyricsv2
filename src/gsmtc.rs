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

// Menghitung tick internal Windows untuk interpolasi waktu
fn get_current_windows_ticks() -> i64 {
    if let Ok(duration) =
        std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH)
    {
        let secs_ticks = duration.as_secs() as i64 * 10_000_000;
        let subsec_ticks = (duration.subsec_nanos() / 100) as i64;
        secs_ticks + subsec_ticks + 116_444_736_000_000_000
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
                    // Cek metadata judul & artis maksimal 1 kali per detik (1000ms) untuk menghemat CPU
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

                    // Cek Timeline setiap 40ms untuk presisi sinkronisasi yang tinggi
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
                                    // Interpolasi tick manual
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
                    // PENTING: GetCurrentSession() gagal cuma berarti "gak ada media
                    // aktif sekarang" - BUKAN berarti manager-nya invalid. Manager
                    // di-reuse selamanya sekali berhasil didapat; kalau di-set None
                    // di sini, tiap 40ms bakal minta ulang RequestAsync() (blocking
                    // .get() di runtime current-thread) selama gak ada media sama
                    // sekali, bikin CPU nyala terus-terusan tanpa alasan.
                    current_title.clear();
                    current_artist.clear();
                    current_album.clear();
                    current_duration_ms = 0;
                    if let Ok(mut info) = shared_clone.lock() {
                        *info = MediaInfo::default();
                    }
                }
            }
            // Loop persis di angka 40ms seperti fungsi native-mu
            sleep(Duration::from_millis(40)).await;
        }
    });

    shared
}
