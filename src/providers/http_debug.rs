use reqwest::Client;
use std::time::Instant;

#[macro_export]
macro_rules! dprintln {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    };
}

pub async fn http_get_with_debug(
    client: &Client,
    url: &str,
    provider_name: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    dprintln!("    🌐 [HTTP GET] [{}] {}", provider_name, url);
    let start = Instant::now();

    let resp = client
        .get(url)
        .header("User-Agent", "MiniLyric/2.0")
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await;

    let dt = start.elapsed().as_millis();

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            dprintln!(
                "    ❌ [HTTP FAIL] [{}] Connection Error ({}ms): {}",
                provider_name,
                dt,
                e
            );
            return Err(Box::new(e));
        }
    };

    let status = resp.status();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let body = resp.text().await.unwrap_or_default();
    let body_snippet: String = body.chars().take(150).collect();
    let body_clean = body_snippet.replace('\n', " ").replace('\r', "");

    if status.is_success() {
        dprintln!(
            "    📩 [HTTP RESP] [{}] Status: {} ({}) | CT: {} | {}ms | Body: \"{}\"",
            provider_name,
            status.as_u16(),
            status.canonical_reason().unwrap_or("OK"),
            content_type,
            dt,
            body_clean
        );
        Ok(body)
    } else {
        dprintln!(
            "    ⚠️ [HTTP ERR] [{}] Status: {} ({}) | CT: {} | {}ms | Body: \"{}\"",
            provider_name,
            status.as_u16(),
            status.canonical_reason().unwrap_or("Error"),
            content_type,
            dt,
            body_clean
        );
        Err(format!(
            "HTTP {} ({})",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Error")
        )
        .into())
    }
}

pub async fn http_post_form_with_debug(
    client: &Client,
    url: &str,
    params: &[(&str, &str)],
    provider_name: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    dprintln!("    🌐 [HTTP POST] [{}] {}", provider_name, url);
    let start = Instant::now();

    let resp = client
        .post(url)
        .form(params)
        .header("User-Agent", "MiniLyric/2.0")
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await;

    let dt = start.elapsed().as_millis();

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            dprintln!(
                "    ❌ [HTTP FAIL] [{}] Connection Error ({}ms): {}",
                provider_name,
                dt,
                e
            );
            return Err(Box::new(e));
        }
    };

    let status = resp.status();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let body = resp.text().await.unwrap_or_default();
    let body_snippet: String = body.chars().take(150).collect();
    let body_clean = body_snippet.replace('\n', " ").replace('\r', "");

    if status.is_success() {
        dprintln!(
            "    📩 [HTTP RESP] [{}] Status: {} ({}) | CT: {} | {}ms | Body: \"{}\"",
            provider_name,
            status.as_u16(),
            status.canonical_reason().unwrap_or("OK"),
            content_type,
            dt,
            body_clean
        );
        Ok(body)
    } else {
        dprintln!(
            "    ⚠️ [HTTP ERR] [{}] Status: {} ({}) | CT: {} | {}ms | Body: \"{}\"",
            provider_name,
            status.as_u16(),
            status.canonical_reason().unwrap_or("Error"),
            content_type,
            dt,
            body_clean
        );
        Err(format!(
            "HTTP {} ({})",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Error")
        )
        .into())
    }
}
