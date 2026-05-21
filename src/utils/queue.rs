use super::app::App;

impl App {
    pub(crate) fn set_queue_and_play(&mut self, idx: usize, mut queue: Vec<usize>) {
        if queue.is_empty() {
            queue.push(idx);
        }

        let pos = queue.iter().position(|&i| i == idx).unwrap_or(0);

        self.play_queue = queue;
        self.queue_pos = Some(pos);
        self.shuffle_history.clear();
        self.shuffle_history.push(pos);

        self.play_track(idx);
    }

    pub(crate) fn play_queued_at(&mut self, pos: usize, record_history: bool) {
        let Some(idx) = self.play_queue.get(pos).copied() else {
            return;
        };

        self.queue_pos = Some(pos);

        if record_history && self.shuffle_history.last().copied() != Some(pos) {
            self.shuffle_history.push(pos);
        }

        self.play_track(idx);
    }

    pub(crate) fn ensure_queue(&mut self) {
        if !self.play_queue.is_empty() {
            return;
        }

        self.play_queue = self.current_view_queue();

        if self.play_queue.is_empty() {
            self.play_queue = (0..self.tracks.len()).collect();
        }

        self.queue_pos = self
            .current
            .and_then(|idx| self.play_queue.iter().position(|&i| i == idx));
    }

    pub(crate) fn random_queue_pos(&mut self) -> Option<usize> {
        let len = self.play_queue.len();

        if len == 0 {
            return None;
        }

        if len == 1 {
            return Some(0);
        }

        if self.shuffle_seed == 0 {
            self.shuffle_seed = 1;
        }

        if self.shuffle_history.len() >= len {
            self.shuffle_history.clear();

            if let Some(pos) = self.queue_pos {
                self.shuffle_history.push(pos);
            }
        }

        for _ in 0..(len * 3) {
            self.shuffle_seed ^= self.shuffle_seed << 13;
            self.shuffle_seed ^= self.shuffle_seed >> 7;
            self.shuffle_seed ^= self.shuffle_seed << 17;

            let pos = (self.shuffle_seed as usize) % len;

            if Some(pos) != self.queue_pos && !self.shuffle_history.contains(&pos) {
                return Some(pos);
            }
        }

        let current = self.queue_pos.unwrap_or(0);
        Some((current + 1) % len)
    }

    pub(crate) fn play_next(&mut self) {
        if self.tracks.is_empty() {
            return;
        }

        self.sync_queue_with_context_if_current_inside();
        self.ensure_queue();

        if self.play_queue.is_empty() {
            return;
        }

        if self.shuffle {
            if let Some(pos) = self.random_queue_pos() {
                self.play_queued_at(pos, true);
            }
        } else {
            let current_pos = self.queue_pos.unwrap_or(0);
            let next_pos = if current_pos + 1 < self.play_queue.len() {
                current_pos + 1
            } else {
                0
            };

            self.play_queued_at(next_pos, true);
        }
    }

    pub(crate) fn play_previous(&mut self) {
        if self.tracks.is_empty() {
            return;
        }

        self.sync_queue_with_context_if_current_inside();
        self.ensure_queue();

        if self.play_queue.is_empty() {
            return;
        }

        if self.shuffle && self.shuffle_history.len() > 1 {
            self.shuffle_history.pop();

            if let Some(pos) = self.shuffle_history.last().copied() {
                self.play_queued_at(pos, false);
                return;
            }
        }

        let current_pos = self.queue_pos.unwrap_or(0);
        let prev_pos = if current_pos > 0 {
            current_pos - 1
        } else {
            self.play_queue.len() - 1
        };

        self.play_queued_at(prev_pos, true);
    }
}
