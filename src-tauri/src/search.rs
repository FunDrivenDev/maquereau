//! Fuzzy search, with nucleo and fzf's syntax: every word must match; `'word` matches
//! exactly, `^word` at the start, `word$` at the end, `!word` excludes. Every search of
//! the app (items, commands, settings) goes through `fuzzy`.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use serde::Serialize;

/// A matching candidate, with the character positions to highlight.
#[derive(Debug, Serialize)]
pub struct Match {
    /// The candidate's position in the list searched.
    pub index: usize,
    pub score: u32,
    pub indices: Vec<u32>,
}

/// The candidates matching `query`, best first, at most `limit`. A blank query matches
/// every candidate, in order.
pub fn fuzzy<'a>(
    query: &str,
    candidates: impl IntoIterator<Item = &'a str>,
    limit: usize,
) -> Vec<Match> {
    if query.trim().is_empty() {
        return candidates
            .into_iter()
            .take(limit)
            .enumerate()
            .map(|(index, _)| Match {
                index,
                score: 0,
                indices: Vec::new(),
            })
            .collect();
    }
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf = Vec::new();
    let mut matches: Vec<Match> = candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, text)| {
            let mut indices = Vec::new();
            let score =
                pattern.indices(Utf32Str::new(text, &mut buf), &mut matcher, &mut indices)?;
            indices.sort_unstable();
            indices.dedup();
            Some(Match {
                index,
                score,
                indices,
            })
        })
        .collect();
    // Stable: equal scores keep the list's order.
    matches.sort_by_key(|m| std::cmp::Reverse(m.score));
    matches.truncate(limit);
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indexes(query: &str, candidates: &[&str]) -> Vec<usize> {
        fuzzy(query, candidates.iter().copied(), 10)
            .iter()
            .map(|m| m.index)
            .collect()
    }

    #[test]
    fn is_fuzzy_and_ranks_the_closest_first() {
        let candidates = ["Open settings", "Delete item", "Settle the bill"];
        assert_eq!(indexes("sett", &candidates), [0, 2]);
        assert_eq!(indexes("dlt", &candidates), [1]);
    }

    #[test]
    fn needs_every_word_and_honours_fzf_syntax() {
        let candidates = ["new item", "rename item", "new window"];
        assert_eq!(indexes("new itm", &candidates), [0]);
        assert_eq!(indexes("new !window", &candidates), [0]);
        assert_eq!(indexes("^re", &candidates), [1]);
    }

    #[test]
    fn a_blank_query_keeps_everything_in_order() {
        assert_eq!(indexes("  ", &["b", "a"]), [0, 1]);
    }

    #[test]
    fn highlights_the_matched_characters() {
        let m = fuzzy("nw", ["new"], 10);
        assert_eq!(m[0].indices, [0, 2]);
    }
}
