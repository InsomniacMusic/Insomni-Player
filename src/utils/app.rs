use std::collections::BTreeMap;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use egui::{Color32, FontId, RichText, Rounding, TextureHandle};
use rodio::{Decoder, DeviceSinkBuilder, Player};

use super::library::LibraryView;
use super::theme::*;
use super::track::{Track, decode_cover, fmt_dur};

pub struct App {
    pub(crate) tracks: Vec<Track>,
    pub(crate) current: Option<usize>,
    pub(crate) is_playing: bool,
    pub(crate) volume: f32,
    pub(crate) seek_frac: f32,
    pub(crate) theme_applied: bool,
    pub(crate) library_view: LibraryView,
    pub(crate) expanded_artist: Option<String>,
    pub(crate) expanded_album: Option<String>,
    pub(crate) hovered_track: Option<usize>,
    pub(crate) album_textures: BTreeMap<String, TextureHandle>,

    pub(crate) cover_texture: Option<TextureHandle>,
    pub(crate) cover_for: Option<usize>,

    pub(crate) stream: rodio::MixerDeviceSink,
    pub(crate) sink: Arc<Mutex<Player>>,
    pub(crate) playback_offset: Duration,

    pub(crate) shuffle: bool,
    pub(crate) shuffle_seed: u64,
    pub(crate) play_queue: Vec<usize>,
    pub(crate) queue_pos: Option<usize>,
    pub(crate) shuffle_history: Vec<usize>,
}

impl App {
    pub fn new() -> Self {
        let stream = DeviceSinkBuilder::open_default_sink().expect("no audio output");
        let sink = Player::connect_new(&stream.mixer());
        sink.set_volume(0.8);

        let mut app = Self {
            tracks: vec![],
            current: None,
            is_playing: false,
            volume: 0.8,
            seek_frac: 0.0,
            theme_applied: false,
            library_view: LibraryView::Artists,
            expanded_artist: None,
            expanded_album: None,
            hovered_track: None,
            album_textures: BTreeMap::new(),
            cover_texture: None,
            cover_for: None,
            stream,
            sink: Arc::new(Mutex::new(sink)),
            playback_offset: Duration::ZERO,

            shuffle: false,
            shuffle_seed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(1),
            play_queue: Vec::new(),
            queue_pos: None,
            shuffle_history: Vec::new(),
        };

        app.scan_music_dir();
        app
    }

    pub(crate) fn open_files(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("audio", &["mp3", "flac", "wav", "ogg"])
            .set_title("add tracks")
            .pick_files()
        {
            for p in paths {
                self.add_track_path(p, false);
            }
        }
    }

    pub(crate) fn play_track(&mut self, idx: usize) {
        let track = match self.tracks.get(idx) {
            Some(t) => t.clone(),
            None => return,
        };

        let Ok(file) = File::open(&track.path) else {
            return;
        };

        let Ok(decoder) = Decoder::try_from(file) else {
            return;
        };

        let new_sink = Player::connect_new(&self.stream.mixer());
        new_sink.set_volume(self.volume);
        new_sink.append(decoder);
        new_sink.play();

        *self.sink.lock().unwrap() = new_sink;

        self.current = Some(idx);
        self.is_playing = true;
        self.seek_frac = 0.0;
        self.playback_offset = Duration::ZERO;
    }

    pub(crate) fn seek_to(&mut self, seconds: f32) {
        let seek_target = Duration::from_secs_f32(seconds);

        let sink = self.sink.lock().unwrap();

        match sink.try_seek(seek_target) {
            Ok(()) => {
                self.playback_offset = Duration::ZERO;
                self.seek_frac = self
                    .current
                    .and_then(|i| self.tracks.get(i))
                    .and_then(|t| t.duration)
                    .map(|total| seek_target.as_secs_f32() / total.as_secs_f32().max(1.0))
                    .unwrap_or(0.0);
            }
            Err(e) => {
                eprintln!("seek failed: {e:?}");
            }
        }
    }

    pub(crate) fn toggle_pause(&mut self) {
        let sink = self.sink.lock().unwrap();

        if sink.is_paused() {
            sink.play();
            self.is_playing = true;
        } else {
            sink.pause();
            self.is_playing = false;
        }
    }

    pub(crate) fn stop(&mut self) {
        self.sink.lock().unwrap().stop();
        self.is_playing = false;
        self.seek_frac = 0.0;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_applied {
            apply_theme(ctx);
            self.theme_applied = true;
        }

        if self.is_playing {
            let sink = self.sink.lock().unwrap();

            if sink.empty() && !sink.is_paused() && sink.get_pos().as_secs_f32() > 0.1 {
                drop(sink);
                self.is_playing = false;
                self.play_next();
            }
        }

        if self.cover_for != self.current {
            self.cover_texture = self
                .current
                .and_then(|i| self.tracks.get(i))
                .and_then(|t| t.cover_data.as_deref())
                .and_then(|data| decode_cover(data, ctx));
            self.cover_for = self.current;
        }

        let pos = self.playback_offset + self.sink.lock().unwrap().get_pos();

        let total = self
            .current
            .and_then(|i| self.tracks.get(i))
            .and_then(|t| t.duration);

        let total_secs = total.map(|d| d.as_secs_f32()).unwrap_or(1.0).max(1.0);

        let track_title = self
            .current
            .and_then(|i| self.tracks.get(i))
            .map(|t| t.display_title().to_string())
            .unwrap_or_else(|| "nothing playing".into());

        let track_artist = self
            .current
            .and_then(|i| self.tracks.get(i))
            .and_then(|t| t.artist.clone());

        let track_album = self
            .current
            .and_then(|i| self.tracks.get(i))
            .and_then(|t| t.album.clone());

        let is_playing = self.is_playing;
        let track_count = self.tracks.len();
        let current_idx = self.current;

        egui::TopBottomPanel::top("player")
            .min_height(130.0)
            .frame(
                egui::Frame::none()
                    .fill(SURFACE)
                    .inner_margin(egui::Margin::symmetric(16.0, 12.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let art_size = egui::vec2(80.0, 80.0);

                    if let Some(tex) = &self.cover_texture {
                        ui.add(
                            egui::Image::new(tex)
                                .fit_to_exact_size(art_size)
                                .rounding(Rounding::same(4.0)),
                        );
                    } else {
                        let (rect, _) = ui.allocate_exact_size(art_size, egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, Rounding::same(4.0), SURFACE2);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "♪",
                            FontId::proportional(28.0),
                            TEXT_DIM,
                        );
                    }

                    ui.add_space(10.0);

                    ui.vertical(|ui| {
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            let indicator = if is_playing { "▶" } else { "■" };
                            let ind_color = if is_playing { Color32::WHITE } else { TEXT_DIM };

                            ui.label(RichText::new(indicator).color(ind_color).size(10.0));
                            ui.label(
                                RichText::new(&track_title)
                                    .size(15.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button(RichText::new("+ add files").size(12.0)).clicked()
                                    {
                                        self.open_files();
                                    }
                                },
                            );
                        });

                        if let Some(artist) = &track_artist {
                            ui.label(RichText::new(artist).size(12.0).color(TEXT_MID));
                        }

                        if let Some(album) = &track_album {
                            ui.label(RichText::new(album).size(11.0).color(TEXT_DIM));
                        }
                    });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(fmt_dur(pos))
                            .size(11.0)
                            .color(TEXT_MID)
                            .font(FontId::monospace(11.0)),
                    );

                    let resp = ui.add(
                        egui::Slider::new(&mut self.seek_frac, 0.0..=1.0)
                            .show_value(false)
                            .clamp_to_range(true),
                    );

                    if resp.drag_stopped() {
                        self.seek_to(self.seek_frac * total_secs);
                    } else if !resp.dragged() {
                        self.seek_frac = pos.as_secs_f32() / total_secs;
                    }

                    let total_str = total.map(fmt_dur).unwrap_or_else(|| "--:--".into());

                    ui.label(
                        RichText::new(total_str)
                            .size(11.0)
                            .color(TEXT_DIM)
                            .font(FontId::monospace(11.0)),
                    );
                });

                ui.horizontal(|ui| {
                    let play_lbl = if is_playing { "⏸" } else { "▶" };

                    if ui.button(RichText::new("⏮").size(14.0)).clicked() {
                        self.play_previous();
                    }

                    if ui.button(RichText::new(play_lbl).size(16.0)).clicked() {
                        if self.current.is_some() {
                            self.toggle_pause();
                        } else if !self.tracks.is_empty() {
                            let queue = self.active_context_queue();
                            let start = queue.first().copied().unwrap_or(0);
                            self.set_queue_and_play(start, queue);
                        }
                    }

                    if ui.button(RichText::new("⏹").size(14.0)).clicked() {
                        self.stop();
                    }

                    if ui.button(RichText::new("⏭").size(14.0)).clicked() {
                        self.play_next();
                    }

                    let shuffle_fill = if self.shuffle {
                        Color32::from_rgb(45, 45, 58)
                    } else {
                        SURFACE2
                    };

                    if ui
                        .add(egui::Button::new(RichText::new("🔀").size(14.0)).fill(shuffle_fill))
                        .clicked()
                    {
                        self.shuffle = !self.shuffle;

                        if self.shuffle {
                            self.sync_queue_with_context_if_current_inside();
                        }
                    }

                    ui.add_space(12.0);

                    ui.label(RichText::new("vol").size(11.0).color(TEXT_DIM));

                    let vol_r = ui.add(
                        egui::Slider::new(&mut self.volume, 0.0..=1.0)
                            .show_value(false)
                            .clamp_to_range(true),
                    );

                    if vol_r.changed() {
                        self.sink.lock().unwrap().set_volume(self.volume);
                    }

                    ui.label(
                        RichText::new(format!("{}%", (self.volume * 100.0).round() as u32))
                            .size(11.0)
                            .color(TEXT_DIM)
                            .font(FontId::monospace(11.0)),
                    );
                });
            });

        egui::TopBottomPanel::bottom("statusbar")
            .frame(
                egui::Frame::none()
                    .fill(SURFACE)
                    .inner_margin(egui::Margin::symmetric(16.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{track_count} tracks"))
                            .size(11.0)
                            .color(TEXT_DIM),
                    );

                    ui.label(RichText::new("·").size(11.0).color(TEXT_DIM));

                    ui.label(
                        RichText::new("double-click row to play")
                            .size(11.0)
                            .color(TEXT_DIM),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .selectable_label(self.library_view == LibraryView::Queue, "queue")
                            .clicked()
                        {
                            self.library_view = LibraryView::Queue;
                            self.expanded_artist = None;
                            self.expanded_album = None;
                        }

                        if ui
                            .selectable_label(self.library_view == LibraryView::Albums, "albums")
                            .clicked()
                        {
                            self.library_view = LibraryView::Albums;
                            self.expanded_artist = None;
                        }

                        if ui
                            .selectable_label(self.library_view == LibraryView::Artists, "artists")
                            .clicked()
                        {
                            self.library_view = LibraryView::Artists;
                            self.expanded_album = None;
                        }

                        if ui
                            .selectable_label(self.library_view == LibraryView::Tracks, "tracks")
                            .clicked()
                        {
                            self.library_view = LibraryView::Tracks;
                            self.expanded_artist = None;
                            self.expanded_album = None;
                        }
                    });
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.tracks.is_empty() {
                ui.add_space(60.0);

                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("no tracks loaded").size(13.0).color(TEXT_DIM));
                    ui.add_space(8.0);

                    if ui.button("+ add files").clicked() {
                        self.open_files();
                    }
                });

                return;
            }

            let hovered_track = self.hovered_track;
            self.hovered_track = None;

            if self.library_view == LibraryView::Queue {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    self.show_queue_view(ui, current_idx, hovered_track);
                });

                return;
            }

            if self.library_view == LibraryView::Artists {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    self.show_artist_grid(ui, current_idx, hovered_track);
                });

                return;
            }

            if self.library_view == LibraryView::Albums {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    self.show_album_grid(ctx, ui, current_idx, hovered_track);
                });

                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                let visible_indices = self.current_view_queue();
                let mut to_play: Option<(usize, Vec<usize>)> = None;

                for (row_n, i) in visible_indices.iter().copied().enumerate() {
                    let track = &self.tracks[i];

                    let is_current = current_idx == Some(i);
                    let row_bg = Self::track_row_fill(is_current, hovered_track == Some(i));

                    let row = egui::Frame::none()
                        .fill(row_bg)
                        .inner_margin(egui::Margin::symmetric(12.0, 5.0))
                        .rounding(Rounding::same(4.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());

                            ui.horizontal(|ui| {
                                let idx_label = if is_current {
                                    RichText::new("▶").color(ACCENT).size(10.0)
                                } else {
                                    RichText::new(format!("{:>2}", row_n + 1))
                                        .color(TEXT_DIM)
                                        .size(11.0)
                                        .font(FontId::monospace(11.0))
                                };

                                ui.add_sized([20.0, 16.0], egui::Label::new(idx_label));

                                ui.vertical(|ui| {
                                    ui.add_space(1.0);

                                    let title_color = if is_current {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(205, 205, 210)
                                    };

                                    ui.label(
                                        RichText::new(track.display_title())
                                            .size(13.0)
                                            .color(title_color),
                                    );

                                    if let Some(artist) = &track.artist {
                                        ui.label(RichText::new(artist).size(11.0).color(TEXT_DIM));
                                    }
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let dur = track
                                            .duration
                                            .map(fmt_dur)
                                            .unwrap_or_else(|| "--:--".into());

                                        ui.label(
                                            RichText::new(dur)
                                                .size(11.0)
                                                .color(TEXT_DIM)
                                                .font(FontId::monospace(11.0)),
                                        );
                                    },
                                );
                            });
                        });

                    let row_resp = ui.interact(
                        row.response.rect,
                        ui.make_persistent_id(("track_row", i)),
                        egui::Sense::click(),
                    );

                    if row_resp.hovered() {
                        self.hovered_track = Some(i);
                    }

                    if row_resp.double_clicked() {
                        to_play = Some((i, visible_indices.clone()));
                    }
                }

                if let Some((i, queue)) = to_play {
                    self.set_queue_and_play(i, queue);
                }
            });
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
