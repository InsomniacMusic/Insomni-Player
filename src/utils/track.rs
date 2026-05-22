use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;

use egui::{Color32, ColorImage, TextureHandle, TextureOptions};
use lofty::file::TaggedFileExt;
use lofty::prelude::*;
use lofty::probe::Probe;
use rodio::{Decoder, Source};

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,
    pub name: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<Duration>,
    pub cover_data: Option<Vec<u8>>,
    pub track_number: Option<u32>,
}

fn clean_meta(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

impl Track {
    pub fn from_path(path: PathBuf, require_metadata: bool) -> Option<Self> {
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut title = None;
        let mut artist = None;
        let mut album = None;
        let mut cover_data = None;
        let mut track_number = None;

        if let Ok(tagged) = Probe::open(&path).and_then(|p| p.read()) {
            let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
            if let Some(tag) = tag {
                title = clean_meta(tag.title().map(|s| s.to_string()));
                artist = clean_meta(tag.artist().map(|s| s.to_string()));
                album = clean_meta(tag.album().map(|s| s.to_string()));
                cover_data = tag.pictures().first().map(|p| p.data().to_vec());
                track_number = tag.track();
            }
        }

        if require_metadata && title.is_none() && artist.is_none() && album.is_none() {
            return None;
        }

        let name = title.clone().unwrap_or(file_name);

        let duration = File::open(&path)
            .ok()
            .and_then(|f| Decoder::try_from(f).ok())
            .and_then(|d| d.total_duration());

        Some(Self {
            path,
            name,
            title,
            artist,
            album,
            duration,
            cover_data,
            track_number,
        })
    }

    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.name)
    }

    pub fn display_artist(&self) -> &str {
        self.artist.as_deref().unwrap_or("unknown artist")
    }
}

pub fn fmt_dur(d: Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

pub fn decode_cover(data: &[u8], ctx: &egui::Context) -> Option<TextureHandle> {
    let img = image::load_from_memory(data).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let pixels: Vec<Color32> = img
        .pixels()
        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    let color_img = ColorImage {
        size: [w as usize, h as usize],
        pixels,
    };
    Some(ctx.load_texture("cover", color_img, TextureOptions::LINEAR))
}

pub fn decode_cover_named(data: &[u8], ctx: &egui::Context, name: String) -> Option<TextureHandle> {
    let img = image::load_from_memory(data).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let pixels: Vec<Color32> = img
        .pixels()
        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();

    let color_img = ColorImage {
        size: [w as usize, h as usize],
        pixels,
    };

    Some(ctx.load_texture(name, color_img, TextureOptions::LINEAR))
}
