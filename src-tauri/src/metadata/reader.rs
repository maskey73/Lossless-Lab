use base64::Engine;
use lofty::prelude::*;
use lofty::probe::Probe;
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Serialize)]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub duration_secs: f64,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
    pub file_path: String,
    pub file_name: String,
    pub format: String,
    pub has_album_art: bool,
}

pub fn read_metadata(path: &str) -> Result<TrackMetadata, String> {
    let tagged_file = Probe::open(path)
        .map_err(|e| format!("Failed to open file: {}", e))?
        .read()
        .map_err(|e| format!("Failed to read tags: {}", e))?;

    let properties = tagged_file.properties();
    let duration_secs = properties.duration().as_secs_f64();
    let sample_rate = properties.sample_rate();
    let bit_depth = properties.bit_depth();
    let channels = properties.channels();

    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

    let (title, artist, album, album_artist, year, genre, track_number, disc_number, has_art) =
        if let Some(tag) = tag {
            (
                tag.title().map(|s| s.to_string()),
                tag.artist().map(|s| s.to_string()),
                tag.album().map(|s| s.to_string()),
                tag.get_string(&ItemKey::AlbumArtist).map(|s| s.to_string()),
                tag.year(),
                tag.genre().map(|s| s.to_string()),
                tag.track().map(|t| t as u32),
                tag.disk().map(|d| d as u32),
                !tag.pictures().is_empty(),
            )
        } else {
            (None, None, None, None, None, None, None, None, false)
        };

    let file_path_obj = Path::new(path);
    let file_name = file_path_obj
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let format = file_path_obj
        .extension()
        .map(|e| e.to_string_lossy().to_uppercase())
        .unwrap_or_else(|| "UNKNOWN".to_string());

    Ok(TrackMetadata {
        title,
        artist,
        album,
        album_artist,
        year,
        genre,
        track_number,
        disc_number,
        duration_secs,
        sample_rate,
        bit_depth,
        channels,
        file_path: path.to_string(),
        file_name,
        format,
        has_album_art: has_art,
    })
}

pub fn get_album_art_base64(path: &str) -> Result<Option<String>, String> {
    let tagged_file = Probe::open(path)
        .map_err(|e| format!("Failed to open file: {}", e))?
        .read()
        .map_err(|e| format!("Failed to read tags: {}", e))?;

    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

    if let Some(tag) = tag {
        if let Some(picture) = tag.pictures().first() {
            let mime = picture.mime_type().map(|m| m.as_str()).unwrap_or("image/jpeg");
            let b64 = base64::engine::general_purpose::STANDARD.encode(picture.data());
            return Ok(Some(format!("data:{};base64,{}", mime, b64)));
        }
    }

    Ok(None)
}
