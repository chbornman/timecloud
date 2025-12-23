use std::collections::{HashMap, VecDeque};

/// Metadata about the current article being processed
#[derive(Debug, Clone, Default)]
pub struct ArticleInfo {
    pub date: String,
    pub title: String,
    pub image_path: Option<String>,
}

/// State of the word cloud at a point in time
#[derive(Debug, Clone)]
pub struct CloudState {
    pub word_frequencies: HashMap<String, u32>,
    pub top_words: Vec<(String, u32)>,
    pub total_words_processed: u64,
    pub current_queue_size: usize,
    pub latest_word: Option<String>,
    pub current_article: Option<ArticleInfo>,
}

/// Sliding window frequency tracker
pub struct TimeCloud {
    queue: VecDeque<String>,
    frequencies: HashMap<String, u32>,
    max_queue_size: usize,
    max_display_words: usize,
    total_words_processed: u64,
    current_article: Option<ArticleInfo>,
    /// Current ramp limit (grows over time if ramp is enabled)
    ramp_current: usize,
    /// Frames between adding each word during ramp (0 = disabled)
    ramp_frames_per_word: u32,
    /// Frame counter for ramp
    ramp_frame_count: u32,
    /// Words currently being displayed (locked in during ramp)
    displayed_words: Vec<String>,
}

impl TimeCloud {
    pub fn new(max_queue_size: usize, max_display_words: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(max_queue_size),
            frequencies: HashMap::new(),
            max_queue_size,
            max_display_words,
            total_words_processed: 0,
            current_article: None,
            ramp_current: 1,
            ramp_frames_per_word: 0,
            ramp_frame_count: 0,
            displayed_words: Vec::new(),
        }
    }

    /// Set the ramp-up rate (frames between adding each word to display limit)
    /// 0 = disabled (use queue fill ratio), >0 = add one word every N frames
    pub fn set_ramp_frames_per_word(&mut self, frames: u32) {
        self.ramp_frames_per_word = frames;
        if frames > 0 {
            self.ramp_current = 1; // Start with 1 word
        }
    }

    /// Add the next highest-frequency word to displayed_words
    fn add_next_display_word(&mut self) {
        let mut all_words: Vec<_> = self
            .frequencies
            .iter()
            .map(|(w, &f)| (w.clone(), f))
            .collect();
        all_words.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        for (word, _) in all_words {
            if !self.displayed_words.contains(&word) {
                self.displayed_words.push(word);
                break;
            }
        }
    }

    /// Call each frame to advance the ramp
    pub fn advance_ramp(&mut self) {
        if self.ramp_frames_per_word > 0 && self.ramp_current < self.max_display_words {
            // Seed the first word if we haven't yet
            if self.displayed_words.is_empty() && !self.frequencies.is_empty() {
                self.add_next_display_word();
            }

            self.ramp_frame_count += 1;
            if self.ramp_frame_count >= self.ramp_frames_per_word {
                self.ramp_frame_count = 0;
                self.ramp_current += 1;
                self.add_next_display_word();
            }
        }
    }

    /// Set the current article being processed
    pub fn set_article(&mut self, article: ArticleInfo) {
        self.current_article = Some(article);
    }

    /// Add a word to the sliding window, returning the new state
    pub fn add_word(&mut self, word: String) -> CloudState {
        // Evict oldest if at capacity
        if self.queue.len() >= self.max_queue_size {
            if let Some(old_word) = self.queue.pop_front() {
                if let Some(count) = self.frequencies.get_mut(&old_word) {
                    *count -= 1;
                    if *count == 0 {
                        self.frequencies.remove(&old_word);
                    }
                }
            }
        }

        // Add new word
        *self.frequencies.entry(word.clone()).or_insert(0) += 1;
        self.queue.push_back(word);
        self.total_words_processed += 1;

        self.get_state()
    }

    /// Get the current state without modifying it
    pub fn get_state(&self) -> CloudState {
        // Determine display count based on ramp mode
        let display_count = if self.ramp_frames_per_word > 0 {
            // Frame-based ramp: use ramp_current directly
            self.ramp_current
        } else {
            // Queue fill ratio ramp (original behavior)
            let fill_ratio = self.queue.len() as f32 / self.max_queue_size as f32;
            let scaled_display = ((self.max_display_words as f32) * fill_ratio).ceil() as usize;
            scaled_display.max(3) // Always show at least 3 words
        };

        // During ramp (not at full capacity), use locked displayed_words list
        // Only swap words once we've reached max_display_words
        let word_vec = if self.ramp_frames_per_word > 0 && self.ramp_current < self.max_display_words {
            // Still ramping: return the locked displayed_words with their current frequencies
            self.displayed_words
                .iter()
                .filter_map(|w| self.frequencies.get(w).map(|&f| (w.clone(), f)))
                .collect()
        } else {
            // At full capacity or no ramp: normal top-N by frequency
            let mut word_vec: Vec<_> = self
                .frequencies
                .iter()
                .map(|(w, &f)| (w.clone(), f))
                .collect();
            word_vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            word_vec.truncate(display_count);
            word_vec
        };

        CloudState {
            word_frequencies: self.frequencies.clone(),
            top_words: word_vec,
            total_words_processed: self.total_words_processed,
            current_queue_size: self.queue.len(),
            latest_word: self.queue.back().cloned(),
            current_article: self.current_article.clone(),
        }
    }

    /// Process words, yielding state every N words
    pub fn process_words_batched<I>(
        &mut self,
        words: I,
        batch_size: usize,
    ) -> impl Iterator<Item = CloudState> + '_
    where
        I: IntoIterator<Item = String>,
        I::IntoIter: 'static,
    {
        let mut count = 0;
        words.into_iter().filter_map(move |word| {
            self.add_word(word);
            count += 1;
            if count >= batch_size {
                count = 0;
                Some(self.get_state())
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_word() {
        let mut tc = TimeCloud::new(100, 10);
        let state = tc.add_word("hello".to_string());
        assert_eq!(state.total_words_processed, 1);
        assert_eq!(state.top_words.len(), 1);
        assert_eq!(state.top_words[0], ("hello".to_string(), 1));
    }

    #[test]
    fn test_frequency_tracking() {
        let mut tc = TimeCloud::new(100, 10);
        tc.add_word("hello".to_string());
        tc.add_word("world".to_string());
        let state = tc.add_word("hello".to_string());

        assert_eq!(state.top_words[0], ("hello".to_string(), 2));
        assert_eq!(state.top_words[1], ("world".to_string(), 1));
    }

    #[test]
    fn test_eviction() {
        let mut tc = TimeCloud::new(3, 10);
        tc.add_word("a".to_string());
        tc.add_word("b".to_string());
        tc.add_word("c".to_string());
        let state = tc.add_word("d".to_string()); // evicts "a"

        assert_eq!(state.current_queue_size, 3);
        assert!(!state.word_frequencies.contains_key("a"));
    }
}
