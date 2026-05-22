use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use egui::TextureHandle;

use super::app::App;
use super::track::{Track, decode_cover_named};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LibraryView {
    Tracks,
    Artists,
    Albums,
    Queue,
}

#[derive(Clone)]
pub(crate) struct AlbumGroup {
    pub(crate) key: String,
    pub(crate) album: String,
    pub(crate) artist: Option<String>,
    pub(crate) indices: Vec<usize>,
    pub(crate) cover_track: Option<usize>,
}

impl App {
    fn default_music_dirs() -> Vec<PathBuf> {
        #[cfg(target_os = "android")]
        {
            // Android doesnt work the same as desktop file systems (windows, mac, linux).
            // Later this should be replaced with MediaStore / SAF picker results.
            Vec::new()
        }

        #[cfg(not(target_os = "android"))]
        {
            let mut dirs = Vec::new();

            if let Some(audio_dir) = dirs::audio_dir() {
                dirs.push(audio_dir);
            }

            #[cfg(target_os = "windows")]
            {
                if let Some(profile) = std::env::var_os("USERPROFILE") {
                    let music = PathBuf::from(profile).join("Music");
                    if music.exists() && !dirs.contains(&music) {
                        dirs.push(music);
                    }
                }
            }

            #[cfg(any(target_os = "macos", target_os = "linux"))]
            {
                if let Some(home) = std::env::var_os("HOME") {
                    let music = PathBuf::from(home).join("Music");
                    if music.exists() && !dirs.contains(&music) {
                        dirs.push(music);
                    }
                }
            }

            dirs
        }
    }

    pub(crate) fn is_audio_file(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| {
                matches!(
                    e.to_ascii_lowercase().as_str(),
                    "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" | "opus"
                )
            })
            .unwrap_or(false)
    }

    pub(crate) fn collect_audio_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                Self::collect_audio_files(&path, out);
            } else if Self::is_audio_file(&path) {
                out.push(path);
            }
        }
    }

    pub(crate) fn add_track_path(&mut self, path: PathBuf, require_metadata: bool) {
        if self.tracks.iter().any(|t| t.path == path) {
            return;
        }

        if let Some(track) = Track::from_path(path, require_metadata) {
            self.tracks.push(track);
        }
    }

    pub fn scan_music_dir(&mut self) {
        let music_dirs = Self::default_music_dirs();

        for music_dir in music_dirs {
            let mut paths = Vec::new();
            Self::collect_audio_files(&music_dir, &mut paths);
            paths.sort();

            for path in paths {
                self.add_track_path(path, true);
            }
        }
    }

    pub(crate) fn active_context_queue(&self) -> Vec<usize> {
        match self.library_view {
            LibraryView::Albums => {
                if let Some(expanded_key) = &self.expanded_album {
                    if let Some(group) = self
                        .album_groups()
                        .into_iter()
                        .find(|g| &g.key == expanded_key)
                    {
                        return group.indices;
                    }
                }

                self.current_view_queue()
            }

            LibraryView::Artists => {
                if let Some(expanded_artist) = &self.expanded_artist {
                    if let Some((_, indices)) = self
                        .artist_groups()
                        .into_iter()
                        .find(|(artist, _)| artist == expanded_artist)
                    {
                        return indices;
                    }
                }

                self.current_view_queue()
            }

            LibraryView::Queue => self.play_queue.clone(),

            LibraryView::Tracks => self.current_view_queue(),
        }
    }

    pub(crate) fn sync_queue_with_context_if_current_inside(&mut self) {
        let Some(current) = self.current else {
            return;
        };

        if !matches!(
            self.library_view,
            LibraryView::Artists | LibraryView::Albums
        ) {
            return;
        }

        let queue = self.active_context_queue();

        if queue.is_empty() || self.play_queue == queue {
            return;
        }

        let Some(pos) = queue.iter().position(|&idx| idx == current) else {
            return;
        };

        self.play_queue = queue;
        self.queue_pos = Some(pos);
        self.shuffle_history.clear();
        self.shuffle_history.push(pos);
    }

    pub(crate) fn track_visible_in_view(track: &Track, view: LibraryView) -> bool {
        match view {
            LibraryView::Tracks => true,
            LibraryView::Artists => track.artist.is_some(),
            LibraryView::Albums => track.album.is_some(),
            LibraryView::Queue => true,
        }
    }

    pub(crate) fn current_view_queue(&self) -> Vec<usize> {
        if self.library_view == LibraryView::Queue {
            return self.play_queue.clone();
        }

        let mut queue: Vec<usize> = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| Self::track_visible_in_view(track, self.library_view))
            .map(|(i, _)| i)
            .collect();

        queue.sort_by(|a, b| {
            let ta = &self.tracks[*a];
            let tb = &self.tracks[*b];

            match self.library_view {
                LibraryView::Tracks => ta
                    .display_title()
                    .to_ascii_lowercase()
                    .cmp(&tb.display_title().to_ascii_lowercase()),

                LibraryView::Artists => (
                    ta.display_artist().to_ascii_lowercase(),
                    ta.album.clone().unwrap_or_default().to_ascii_lowercase(),
                    ta.track_number.unwrap_or(u32::MAX),
                    ta.display_title().to_ascii_lowercase(),
                )
                    .cmp(&(
                        tb.display_artist().to_ascii_lowercase(),
                        tb.album.clone().unwrap_or_default().to_ascii_lowercase(),
                        tb.track_number.unwrap_or(u32::MAX),
                        tb.display_title().to_ascii_lowercase(),
                    )),

                LibraryView::Albums => (
                    ta.album.clone().unwrap_or_default().to_ascii_lowercase(),
                    ta.display_artist().to_ascii_lowercase(),
                    ta.track_number.unwrap_or(u32::MAX),
                    ta.display_title().to_ascii_lowercase(),
                )
                    .cmp(&(
                        tb.album.clone().unwrap_or_default().to_ascii_lowercase(),
                        tb.display_artist().to_ascii_lowercase(),
                        tb.track_number.unwrap_or(u32::MAX),
                        tb.display_title().to_ascii_lowercase(),
                    )),

                LibraryView::Queue => std::cmp::Ordering::Equal,
            }
        });

        queue
    }

    pub(crate) fn artist_initials(name: &str) -> String {
        let words: Vec<&str> = name.split_whitespace().collect();

        if words.len() == 1 {
            return words[0].chars().take(3).collect::<String>().to_uppercase();
        }

        words
            .iter()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase()
    }

    pub(crate) fn artist_section_letter(name: &str) -> String {
        name.chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "#".to_string())
    }

    pub(crate) fn short_text(text: &str, max_chars: usize) -> String {
        let count = text.chars().count();

        if count <= max_chars {
            text.to_string()
        } else {
            let mut s = text
                .chars()
                .take(max_chars.saturating_sub(1))
                .collect::<String>();
            s.push('…');
            s
        }
    }

    pub(crate) fn artist_groups(&self) -> Vec<(String, Vec<usize>)> {
        let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();

        for (i, track) in self.tracks.iter().enumerate() {
            let Some(artist) = track.artist.as_ref() else {
                continue;
            };

            groups.entry(artist.clone()).or_default().push(i);
        }

        let mut groups: Vec<(String, Vec<usize>)> = groups.into_iter().collect();

        for (_, indices) in &mut groups {
            indices.sort_by_key(|i| {
                let t = &self.tracks[*i];

                (
                    t.album.clone().unwrap_or_default().to_ascii_lowercase(),
                    t.track_number.unwrap_or(u32::MAX),
                    t.display_title().to_ascii_lowercase(),
                )
            });
        }

        groups
    }

    pub(crate) fn album_groups(&self) -> Vec<AlbumGroup> {
        let mut groups: BTreeMap<String, AlbumGroup> = BTreeMap::new();

        for (i, track) in self.tracks.iter().enumerate() {
            let Some(album) = track.album.as_ref() else {
                continue;
            };

            let artist = track.artist.clone();

            let key = format!(
                "{}\u{1f}{}",
                album.to_ascii_lowercase(),
                artist.clone().unwrap_or_default().to_ascii_lowercase()
            );

            let entry = groups.entry(key.clone()).or_insert_with(|| AlbumGroup {
                key,
                album: album.clone(),
                artist,
                indices: Vec::new(),
                cover_track: None,
            });

            entry.indices.push(i);

            if entry.cover_track.is_none() && track.cover_data.is_some() {
                entry.cover_track = Some(i);
            }
        }

        let mut groups: Vec<AlbumGroup> = groups.into_values().collect();

        for group in &mut groups {
            group.indices.sort_by_key(|i| {
                let t = &self.tracks[*i];

                (
                    t.track_number.unwrap_or(u32::MAX),
                    t.display_title().to_ascii_lowercase(),
                )
            });
        }

        groups.sort_by_key(|g| {
            (
                g.album.to_ascii_lowercase(),
                g.artist.clone().unwrap_or_default().to_ascii_lowercase(),
            )
        });

        groups
    }

    pub(crate) fn get_album_cover_texture(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        cover_track: Option<usize>,
    ) -> Option<TextureHandle> {
        if let Some(tex) = self.album_textures.get(key) {
            return Some(tex.clone());
        }

        let idx = cover_track?;
        let data = self.tracks.get(idx)?.cover_data.clone()?;

        let tex = decode_cover_named(&data, ctx, format!("album_cover_{key}"))?;
        self.album_textures.insert(key.to_string(), tex.clone());

        Some(tex)
    }
}
