use egui::{Color32, FontId, RichText, Rounding, Stroke, TextureHandle};
use std::collections::BTreeMap;

use super::app::App;
use super::library::AlbumGroup;
use super::theme::*;
use super::track::fmt_dur;

impl App {
    pub fn track_row_fill(is_current: bool, is_hovered: bool) -> Color32 {
        if is_current {
            Color32::from_rgb(30, 30, 38)
        } else if is_hovered {
            Color32::from_rgb(26, 26, 31)
        } else {
            Color32::TRANSPARENT
        }
    }

    pub fn show_queue_view(
        &mut self,
        ui: &mut egui::Ui,
        current_idx: Option<usize>,
        hovered_track: Option<usize>,
    ) {
        if self.play_queue.is_empty() {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("queue is empty").size(13.0).color(TEXT_DIM));
                ui.add_space(6.0);
                ui.label(
                    RichText::new("play a track, artist, or album to create a queue")
                        .size(11.0)
                        .color(TEXT_DIM),
                );
            });
            return;
        }

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("queue")
                    .size(18.0)
                    .strong()
                    .color(Color32::WHITE),
            );

            ui.label(
                RichText::new(format!("{} tracks", self.play_queue.len()))
                    .size(11.0)
                    .color(TEXT_DIM),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new("clear queue").size(11.0)).clicked() {
                    self.play_queue.clear();
                    self.queue_pos = None;
                    self.shuffle_history.clear();
                }
            });
        });

        ui.add_space(8.0);

        let queue = self.play_queue.clone();
        let mut to_play_pos: Option<usize> = None;

        for (pos, idx) in queue.iter().copied().enumerate() {
            let Some(track) = self.tracks.get(idx) else {
                continue;
            };

            let is_current = current_idx == Some(idx);
            let is_queue_pos = self.queue_pos == Some(pos);
            let row_bg = Self::track_row_fill(is_current, hovered_track == Some(idx));

            let row = egui::Frame::none()
                .fill(row_bg)
                .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                .rounding(Rounding::same(4.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    ui.horizontal(|ui| {
                        let marker = if is_current {
                            "▶".to_string()
                        } else {
                            format!("{:>2}", pos + 1)
                        };

                        ui.add_sized(
                            [28.0, 16.0],
                            egui::Label::new(
                                RichText::new(marker)
                                    .size(11.0)
                                    .color(if is_current { ACCENT } else { TEXT_DIM })
                                    .font(FontId::monospace(11.0)),
                            ),
                        );

                        ui.vertical(|ui| {
                            let title_color = if is_current {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(215, 215, 220)
                            };

                            ui.label(
                                RichText::new(track.display_title())
                                    .size(13.0)
                                    .color(title_color),
                            );

                            let mut sub = String::new();

                            if let Some(artist) = &track.artist {
                                sub.push_str(artist);
                            }

                            if let Some(album) = &track.album {
                                if !sub.is_empty() {
                                    sub.push_str(" · ");
                                }
                                sub.push_str(album);
                            }

                            if !sub.is_empty() {
                                ui.label(RichText::new(sub).size(11.0).color(TEXT_DIM));
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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

                            if is_queue_pos {
                                ui.label(RichText::new("current").size(10.0).color(TEXT_MID));
                            }
                        });
                    });
                });

            let row_resp = ui.interact(
                row.response.rect,
                ui.make_persistent_id(("queue_track_row", pos, idx)),
                egui::Sense::click(),
            );

            if row_resp.hovered() {
                self.hovered_track = Some(idx);
            }

            if row_resp.double_clicked() {
                to_play_pos = Some(pos);
            }
        }

        if let Some(pos) = to_play_pos {
            self.play_queued_at(pos, true);
        }
    }

    pub fn artist_card(
        ui: &mut egui::Ui,
        artist: &str,
        track_count: usize,
        selected: bool,
    ) -> egui::Response {
        let size = egui::vec2(150.0, 192.0);
        let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());

        let square = egui::Rect::from_min_size(rect.min, egui::vec2(150.0, 150.0));

        let bg = if selected {
            Color32::from_rgb(46, 46, 58)
        } else if resp.hovered() {
            Color32::from_rgb(38, 38, 44)
        } else {
            SURFACE2
        };

        ui.painter().rect_filled(square, Rounding::same(12.0), bg);

        ui.painter().text(
            square.center(),
            egui::Align2::CENTER_CENTER,
            Self::artist_initials(artist),
            FontId::proportional(38.0),
            Color32::WHITE,
        );

        ui.painter().text(
            egui::pos2(rect.center().x, square.max.y + 10.0),
            egui::Align2::CENTER_TOP,
            Self::short_text(artist, 18),
            FontId::proportional(13.0),
            Color32::from_rgb(235, 235, 240),
        );

        ui.painter().text(
            egui::pos2(rect.center().x, square.max.y + 28.0),
            egui::Align2::CENTER_TOP,
            format!("{track_count} tracks"),
            FontId::proportional(10.0),
            TEXT_DIM,
        );

        resp
    }

    pub fn show_artist_grid(
        &mut self,
        ui: &mut egui::Ui,
        current_idx: Option<usize>,
        hovered_track: Option<usize>,
    ) {
        let groups = self.artist_groups();

        if groups.is_empty() {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("no artists with metadata found")
                        .size(13.0)
                        .color(TEXT_DIM),
                );
            });
            return;
        }

        let mut sections: BTreeMap<String, Vec<(String, Vec<usize>)>> = BTreeMap::new();

        for (artist, indices) in groups.clone() {
            let letter = Self::artist_section_letter(&artist);
            sections.entry(letter).or_default().push((artist, indices));
        }

        for (letter, section_groups) in sections {
            ui.add_space(12.0);
            ui.label(
                RichText::new(&letter)
                    .size(18.0)
                    .strong()
                    .color(Color32::from_rgb(255, 140, 40)),
            );
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(18.0, 24.0);

                for (artist, indices) in section_groups.iter() {
                    let selected = self.expanded_artist.as_deref() == Some(artist.as_str());

                    let resp = Self::artist_card(ui, artist, indices.len(), selected);

                    if resp.clicked() {
                        if selected {
                            self.expanded_artist = None;
                        } else {
                            self.expanded_artist = Some(artist.clone());
                        }
                    }
                }
            });
        }

        let Some(expanded) = self.expanded_artist.clone() else {
            return;
        };

        let Some((_, indices)) = groups.iter().find(|(artist, _)| *artist == expanded) else {
            return;
        };

        let artist_queue = indices.clone();

        ui.add_space(18.0);

        ui.label(
            RichText::new(&expanded)
                .size(17.0)
                .strong()
                .color(Color32::WHITE),
        );

        ui.add_space(6.0);

        let mut to_play: Option<(usize, Vec<usize>)> = None;

        for idx in artist_queue.iter().copied() {
            let track = &self.tracks[idx];
            let is_current = current_idx == Some(idx);

            let row_bg = Self::track_row_fill(is_current, hovered_track == Some(idx));

            let row = egui::Frame::none()
                .fill(row_bg)
                .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                .rounding(Rounding::same(4.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    ui.horizontal(|ui| {
                        let marker = if is_current { "▶" } else { "♪" };

                        ui.add_sized(
                            [20.0, 16.0],
                            egui::Label::new(
                                RichText::new(marker).size(11.0).color(if is_current {
                                    ACCENT
                                } else {
                                    TEXT_DIM
                                }),
                            ),
                        );

                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(track.display_title())
                                    .size(13.0)
                                    .color(Color32::from_rgb(215, 215, 220)),
                            );

                            if let Some(album) = &track.album {
                                ui.label(RichText::new(album).size(11.0).color(TEXT_DIM));
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                        });
                    });
                });

            let row_resp = ui.interact(
                row.response.rect,
                ui.make_persistent_id(("artist_track_row", idx)),
                egui::Sense::click(),
            );

            if row_resp.hovered() {
                self.hovered_track = Some(idx);
            }

            if row_resp.double_clicked() {
                to_play = Some((idx, artist_queue.clone()));
            }
        }

        if let Some((i, queue)) = to_play {
            self.set_queue_and_play(i, queue);
        }
    }

    pub fn album_card(
        ui: &mut egui::Ui,
        album: &str,
        artist: Option<&str>,
        track_count: usize,
        selected: bool,
        cover: Option<&TextureHandle>,
    ) -> egui::Response {
        let size = egui::vec2(160.0, 218.0);
        let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());

        let square = egui::Rect::from_min_size(rect.min, egui::vec2(160.0, 160.0));

        let bg = if selected {
            Color32::from_rgb(46, 46, 58)
        } else if resp.hovered() {
            Color32::from_rgb(38, 38, 44)
        } else {
            SURFACE2
        };

        ui.painter().rect_filled(square, Rounding::same(12.0), bg);

        if let Some(tex) = cover {
            let image_rect = square.shrink(1.0);

            ui.put(
                image_rect,
                egui::Image::new(tex)
                    .fit_to_exact_size(image_rect.size())
                    .rounding(Rounding::same(12.0)),
            );

            ui.painter().rect_stroke(
                image_rect,
                Rounding::same(12.0),
                Stroke::new(1.0, Color32::from_rgb(55, 55, 62)),
            );
        } else {
            ui.painter().text(
                square.center(),
                egui::Align2::CENTER_CENTER,
                Self::artist_initials(album),
                FontId::proportional(36.0),
                Color32::WHITE,
            );
        }

        ui.painter().text(
            egui::pos2(rect.left(), square.max.y + 10.0),
            egui::Align2::LEFT_TOP,
            Self::short_text(album, 22),
            FontId::proportional(14.0),
            Color32::from_rgb(235, 235, 240),
        );

        if let Some(artist) = artist {
            ui.painter().text(
                egui::pos2(rect.left(), square.max.y + 29.0),
                egui::Align2::LEFT_TOP,
                Self::short_text(artist, 24),
                FontId::proportional(11.0),
                TEXT_MID,
            );
        }

        ui.painter().text(
            egui::pos2(rect.left(), square.max.y + 45.0),
            egui::Align2::LEFT_TOP,
            format!("{track_count} tracks"),
            FontId::proportional(10.0),
            TEXT_DIM,
        );

        resp
    }

    pub fn show_album_grid(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        current_idx: Option<usize>,
        hovered_track: Option<usize>,
    ) {
        let groups = self.album_groups();

        if groups.is_empty() {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("no albums with metadata found")
                        .size(13.0)
                        .color(TEXT_DIM),
                );
            });
            return;
        }

        let mut sections: BTreeMap<String, Vec<AlbumGroup>> = BTreeMap::new();

        for group in groups.clone() {
            let letter = Self::artist_section_letter(&group.album);
            sections.entry(letter).or_default().push(group);
        }

        for (letter, section_groups) in sections {
            ui.add_space(12.0);
            ui.label(
                RichText::new(&letter)
                    .size(18.0)
                    .strong()
                    .color(Color32::from_rgb(255, 140, 40)),
            );
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(18.0, 24.0);

                for group in section_groups.iter() {
                    let selected = self.expanded_album.as_deref() == Some(group.key.as_str());

                    let cover = self.get_album_cover_texture(ctx, &group.key, group.cover_track);

                    let resp = Self::album_card(
                        ui,
                        &group.album,
                        group.artist.as_deref(),
                        group.indices.len(),
                        selected,
                        cover.as_ref(),
                    );

                    if resp.clicked() {
                        if selected {
                            self.expanded_album = None;
                        } else {
                            self.expanded_album = Some(group.key.clone());
                        }
                    }
                }
            });
        }

        let Some(expanded_key) = self.expanded_album.clone() else {
            return;
        };

        let Some(group) = groups.iter().find(|g| g.key == expanded_key).cloned() else {
            return;
        };

        let album_queue = group.indices.clone();

        ui.add_space(18.0);

        ui.label(
            RichText::new(&group.album)
                .size(17.0)
                .strong()
                .color(Color32::WHITE),
        );

        if let Some(artist) = &group.artist {
            ui.label(RichText::new(artist).size(12.0).color(TEXT_MID));
        }

        ui.add_space(6.0);

        let mut to_play: Option<(usize, Vec<usize>)> = None;

        for idx in album_queue.iter().copied() {
            let track = &self.tracks[idx];
            let is_current = current_idx == Some(idx);

            let row_bg = Self::track_row_fill(is_current, hovered_track == Some(idx));

            let row = egui::Frame::none()
                .fill(row_bg)
                .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                .rounding(Rounding::same(4.0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    ui.horizontal(|ui| {
                        let marker = if is_current { "▶" } else { "♪" };

                        ui.add_sized(
                            [20.0, 16.0],
                            egui::Label::new(
                                RichText::new(marker).size(11.0).color(if is_current {
                                    ACCENT
                                } else {
                                    TEXT_DIM
                                }),
                            ),
                        );

                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(track.display_title())
                                    .size(13.0)
                                    .color(Color32::from_rgb(215, 215, 220)),
                            );

                            if let Some(artist) = &track.artist {
                                ui.label(RichText::new(artist).size(11.0).color(TEXT_DIM));
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                        });
                    });
                });

            let row_resp = ui.interact(
                row.response.rect,
                ui.make_persistent_id(("album_track_row", idx)),
                egui::Sense::click(),
            );

            if row_resp.hovered() {
                self.hovered_track = Some(idx);
            }

            if row_resp.double_clicked() {
                to_play = Some((idx, album_queue.clone()));
            }
        }

        if let Some((i, queue)) = to_play {
            self.set_queue_and_play(i, queue);
        }
    }
}
