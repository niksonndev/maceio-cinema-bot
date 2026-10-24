use crate::types::{Error, Result};
use dashmap::DashMap;
use image::imageops::FilterType;
use std::sync::OnceLock;
use std::time::Duration;

/// Telegram rejects WebP in `sendPhoto`, while Ingresso.com only serves WebP
/// posters. Convert them to JPEG (once per container) before uploading.
const MAX_WIDTH: u32 = 640;
const JPEG_QUALITY: u8 = 82;
const FETCH_TIMEOUT: Duration = Duration::from_secs(6);

fn cache() -> &'static DashMap<String, Vec<u8>> {
    static CACHE: OnceLock<DashMap<String, Vec<u8>>> = OnceLock::new();
    CACHE.get_or_init(DashMap::new)
}

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .timeout(FETCH_TIMEOUT)
            .build()
            .expect("poster client")
    })
}

/// Returns JPEG bytes ready for Telegram, or `None` when the poster is absent
/// or cannot be fetched/decoded. Callers fall back to a text-only message.
pub async fn jpeg_for(poster: Option<&str>) -> Option<Vec<u8>> {
    let url = poster?;
    if let Some(bytes) = cache().get(url) {
        return Some(bytes.clone());
    }

    match download_and_convert(url).await {
        Ok(bytes) => {
            tracing::debug!(url, bytes = bytes.len(), "Converted poster to JPEG");
            cache().insert(url.to_owned(), bytes.clone());
            Some(bytes)
        }
        Err(error) => {
            tracing::warn!(url, %error, "Could not prepare poster; sending text only");
            None
        }
    }
}

async fn download_and_convert(url: &str) -> Result<Vec<u8>> {
    let bytes = client()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    to_jpeg(&bytes)
}

fn to_jpeg(bytes: &[u8]) -> Result<Vec<u8>> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| Error::Msg(format!("poster decode: {error}")))?;
    let image = limit_width(image, MAX_WIDTH);
    let rgb = image.to_rgb8();
    let mut jpeg = Vec::with_capacity(64 * 1024);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, JPEG_QUALITY);
    encoder
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|error| Error::Msg(format!("poster encode: {error}")))?;
    Ok(jpeg)
}

fn limit_width(image: image::DynamicImage, max_width: u32) -> image::DynamicImage {
    let width = image.width();
    if width <= max_width {
        return image;
    }
    let height = image.height();
    let target_height =
        ((u64::from(height) * u64::from(max_width)) / u64::from(width)).max(1) as u32;
    image.resize_exact(max_width, target_height, FilterType::Lanczos3)
}

#[cfg(test)]
mod tests {
    use super::{limit_width, to_jpeg, MAX_WIDTH};

    const WEBP_4X6: &[u8] = &[
        82, 73, 70, 70, 34, 0, 0, 0, 87, 69, 66, 80, 86, 80, 56, 76, 22, 0, 0, 0, 47, 3, 64, 1, 0,
        7, 80, 143, 34, 87, 171, 255, 1, 128, 32, 252, 111, 155, 136, 232, 127, 8,
    ];

    #[test]
    fn converts_webp_to_jpeg() {
        let jpeg = to_jpeg(WEBP_4X6).unwrap();
        assert_eq!(&jpeg[..3], [0xFF, 0xD8, 0xFF]);
    }

    #[test]
    fn keeps_small_images_untouched_and_downscales_large_ones() {
        let small = image::DynamicImage::new_rgb8(100, 150);
        assert_eq!(limit_width(small, 640).width(), 100);

        let large = image::DynamicImage::new_rgb8(1280, 1920);
        let resized = limit_width(large, MAX_WIDTH);
        assert_eq!(resized.width(), MAX_WIDTH);
        assert_eq!(resized.height(), 960);
    }
}
