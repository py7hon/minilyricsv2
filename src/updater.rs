// src/updater.rs
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::Write;
use std::process::Command;
use windows::core::{w, PCWSTR};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_YESNO, SW_SHOWNORMAL,
};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO_RELEASES_URL: &str = "https://api.github.com/repos/py7hon/minilyricsv2/releases/latest";

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<ReleaseAsset>,
}

pub fn is_newer_version(latest_tag: &str, current_ver: &str) -> bool {
    let clean_latest = latest_tag.trim().trim_start_matches('v');
    let clean_current = current_ver.trim().trim_start_matches('v');

    let parse_ver = |s: &str| -> Vec<u32> {
        s.split('.')
            .filter_map(|part| part.parse::<u32>().ok())
            .collect()
    };

    let l = parse_ver(clean_latest);
    let c = parse_ver(clean_current);

    if l.len() == 3 && c.len() == 3 {
        (l[0], l[1], l[2]) > (c[0], c[1], c[2])
    } else {
        clean_latest != clean_current
    }
}

pub fn open_url(url: &str) {
    let url_w: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(url_w.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
}

pub fn show_msg_box(title: &str, text: &str, is_question: bool) -> bool {
    let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let text_w: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();

    let flags = if is_question {
        MB_YESNO | MB_ICONQUESTION
    } else {
        MB_OK | MB_ICONINFORMATION
    };

    unsafe {
        let res = MessageBoxW(
            None,
            PCWSTR(text_w.as_ptr()),
            PCWSTR(title_w.as_ptr()),
            flags,
        );
        res == IDYES
    }
}

pub fn check_for_updates_async(manual_trigger: bool) {
    tokio::spawn(async move {
        let client = match Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };

        let req = client
            .get(REPO_RELEASES_URL)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) MiniLyricsV2AutoUpdater",
            )
            .send()
            .await;

        let resp = match req {
            Ok(r) => r,
            Err(e) => {
                if manual_trigger {
                    show_msg_box(
                        "Update Check Failed",
                        &format!("Unable to check for updates.\nError: {}", e),
                        false,
                    );
                }
                return;
            }
        };

        let release: GitHubRelease = match resp.json().await {
            Ok(rel) => rel,
            Err(_) => {
                if manual_trigger {
                    show_msg_box(
                        "Update Check Failed",
                        "Could not parse release information from GitHub.",
                        false,
                    );
                }
                return;
            }
        };

        if is_newer_version(&release.tag_name, CURRENT_VERSION) {
            let msg = format!(
                "A new version of MiniLyrics V2 is available!\n\nCurrent Version: v{}\nLatest Version: {}\n\nWould you like to download and install the update now?",
                CURRENT_VERSION, release.tag_name
            );

            let user_agreed = show_msg_box("New Update Available", &msg, true);
            if user_agreed {
                // Find installer asset (.exe) or first asset
                let download_url = release
                    .assets
                    .iter()
                    .find(|a| a.name.ends_with(".exe"))
                    .map(|a| a.browser_download_url.clone())
                    .unwrap_or_else(|| release.html_url.clone());

                if download_url.ends_with(".exe") {
                    if download_and_install_update(&client, &download_url).await {
                        std::process::exit(0);
                    }
                } else {
                    open_url(&release.html_url);
                }
            }
        } else if manual_trigger {
            show_msg_box(
                "Up to Date",
                &format!(
                    "You are using the latest version of MiniLyrics V2 (v{}).",
                    CURRENT_VERSION
                ),
                false,
            );
        }
    });
}

async fn download_and_install_update(client: &Client, url: &str) -> bool {
    let temp_dir = env::temp_dir();
    let temp_exe = temp_dir.join("MiniLyricsV2_Setup_Update.exe");

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(_) => {
            show_msg_box(
                "Update Failed",
                "Failed to download update setup file.",
                false,
            );
            return false;
        }
    };

    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(_) => return false,
    };

    let mut file = match File::create(&temp_exe) {
        Ok(f) => f,
        Err(_) => return false,
    };

    if file.write_all(&bytes).is_err() {
        return false;
    }

    drop(file);

    // Launch setup executable in silent mode
    let spawn_res = Command::new(&temp_exe).arg("/SILENT").spawn();

    spawn_res.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("v0.1.11", "0.1.10"));
        assert!(is_newer_version("0.2.0", "0.1.10"));
        assert!(is_newer_version("1.0.0", "0.1.10"));
        assert!(!is_newer_version("v0.1.10", "0.1.10"));
        assert!(!is_newer_version("0.1.9", "0.1.10"));
    }
}
