use anyhow::Result;
use regex::Regex;
use rust_stemmers::{Algorithm, Stemmer};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::config::Config;

/// A word with its source article info
#[derive(Debug, Clone)]
pub struct TokenizedWord {
    pub word: String,
    pub article_date: String,
    pub article_title: String,
    pub article_image: Option<String>,
}

/// Text tokenizer with stopword filtering and optional stemming
pub struct Tokenizer {
    stopwords: HashSet<String>,
    stemmer: Option<Stemmer>,
    word_regex: Regex,
    contraction_regex: Regex,
    min_word_length: usize,
    lowercase: bool,
}

impl Tokenizer {
    pub fn new(config: &Config) -> Result<Self> {
        let stopwords = if config.filter_stopwords {
            match &config.stopwords_path {
                Some(path) => Self::load_stopwords(path)?,
                None => anyhow::bail!("stopwords_path must be set when filter_stopwords is enabled"),
            }
        } else {
            HashSet::new()
        };

        let stemmer = if config.enable_stemming {
            Some(Stemmer::create(Algorithm::English))
        } else {
            None
        };

        // Regex to strip contraction suffixes (both straight and curly apostrophes)
        // Matches: n't, 't, 're, 's, 'd, 'll, 've, 'm
        // These are all stop words when expanded, so we just remove them
        let contraction_regex = Regex::new(r"(?i)(n['']t|[''][tsdm]|['']re|['']ll|['']ve)\b").unwrap();

        Ok(Self {
            stopwords,
            stemmer,
            word_regex: Regex::new(r"[a-zA-Z]+").unwrap(),
            contraction_regex,
            min_word_length: config.min_word_length,
            lowercase: config.lowercase,
        })
    }

    /// Tokenize a string into words
    pub fn tokenize(&self, text: &str) -> Vec<String> {
        let text = if self.lowercase {
            text.to_lowercase()
        } else {
            text.to_string()
        };

        // Strip contraction suffixes (they expand to stop words anyway)
        let text = self.contraction_regex.replace_all(&text, "");

        self.word_regex
            .find_iter(&text)
            .map(|m| m.as_str().to_string())
            .filter(|w| w.len() >= self.min_word_length)
            .filter(|w| !self.stopwords.contains(&w.to_lowercase()))
            .map(|w| self.stem_if_enabled(&w))
            .collect()
    }

    /// Tokenize multiple files, sorted by filename
    /// Returns words with article metadata (date/title parsed from filename)
    /// Also detects matching image files (same name with image extension)
    pub fn tokenize_files_with_metadata(&self, dir: &Path) -> Result<Vec<TokenizedWord>> {
        let mut entries: Vec<_> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "txt")
                    .unwrap_or(false)
            })
            .collect();

        // Sort by filename for chronological order
        entries.sort_by_key(|e| e.file_name());

        let mut all_words = Vec::new();
        for entry in entries {
            let filename = entry.file_name().to_string_lossy().to_string();
            let (date, title) = Self::parse_filename(&filename);

            // Look for matching image file with same base name
            let base_name = filename.trim_end_matches(".txt");
            let image_path = Self::find_matching_image(dir, base_name);

            let content = fs::read_to_string(entry.path())?;
            for word in self.tokenize(&content) {
                all_words.push(TokenizedWord {
                    word,
                    article_date: date.clone(),
                    article_title: title.clone(),
                    article_image: image_path.clone(),
                });
            }
        }

        Ok(all_words)
    }

    /// Find an image file matching the base name (without extension)
    fn find_matching_image(dir: &Path, base_name: &str) -> Option<String> {
        let image_extensions = ["png", "jpg", "jpeg", "webp", "bmp", "gif"];

        for ext in image_extensions {
            let image_path = dir.join(format!("{}.{}", base_name, ext));
            if image_path.exists() {
                return Some(image_path.to_string_lossy().to_string());
            }
            // Also check uppercase extension
            let image_path = dir.join(format!("{}.{}", base_name, ext.to_uppercase()));
            if image_path.exists() {
                return Some(image_path.to_string_lossy().to_string());
            }
        }
        None
    }

    /// Tokenize multiple files, sorted by filename (simple version without metadata)
    pub fn tokenize_files(&self, dir: &Path) -> Result<Vec<String>> {
        let words = self.tokenize_files_with_metadata(dir)?;
        Ok(words.into_iter().map(|tw| tw.word).collect())
    }

    /// Parse filename like "2024-01-15_article-title.txt" into (date, title)
    fn parse_filename(filename: &str) -> (String, String) {
        let name = filename.trim_end_matches(".txt");

        // Try to split on first underscore: "2024-01-15_article-title"
        if let Some(idx) = name.find('_') {
            let date = &name[..idx];
            let slug = &name[idx + 1..];
            // Convert slug back to readable title (replace dashes with spaces, title case)
            let title = slug
                .replace('-', " ")
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().chain(chars).collect(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            (date.to_string(), title)
        } else {
            // No date prefix, just use filename as title
            (String::new(), name.replace('-', " "))
        }
    }

    fn stem_if_enabled(&self, word: &str) -> String {
        match &self.stemmer {
            Some(s) => s.stem(word).to_string(),
            None => word.to_string(),
        }
    }

    fn load_stopwords(path: &Path) -> Result<HashSet<String>> {
        let content = fs::read_to_string(path)?;
        Ok(content
            .lines()
            .map(|l| l.trim().to_lowercase())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect())
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let config = Config {
            filter_stopwords: false,
            enable_stemming: false,
            ..Default::default()
        };
        let tokenizer = Tokenizer::new(&config).unwrap();
        let words = tokenizer.tokenize("Hello World");
        assert_eq!(words, vec!["hello", "world"]);
    }

    #[test]
    fn test_stopword_filtering() {
        let config = Config::default();
        let tokenizer = Tokenizer::new(&config).unwrap();
        let words = tokenizer.tokenize("the quick brown fox");
        assert!(!words.contains(&"the".to_string()));
        assert!(words.contains(&"quick".to_string()));
    }

    #[test]
    fn test_contraction_stripping() {
        let config = Config {
            filter_stopwords: false,
            ..Default::default()
        };
        let tokenizer = Tokenizer::new(&config).unwrap();

        // Contractions are stripped, leaving just the base word
        let words = tokenizer.tokenize("don't won't isn't");
        assert!(words.contains(&"do".to_string()));
        assert!(words.contains(&"wo".to_string()));
        assert!(words.contains(&"is".to_string()));
        assert!(!words.contains(&"not".to_string()));
        assert!(!words.iter().any(|w| w.contains("n't") || w.contains("t")));

        // Possessives are stripped too
        let words = tokenizer.tokenize("John's car");
        assert!(words.contains(&"john".to_string()));
        assert!(words.contains(&"car".to_string()));
        assert!(!words.iter().any(|w| w.contains("'s")));
    }
}
