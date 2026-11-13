//! Rank files and projects for incremental search queries.

use std::path::PathBuf;

/// Maximum number of ranked file results for one query.
pub(crate) const MAX_RESULTS: usize = 50;

const MATCH_START_BONUS: i32 = 12;
const MATCH_BOUNDARY_BONUS: i32 = 10;
const MATCH_CONSECUTIVE_BONUS: i32 = 8;
const MATCH_STREAK_STEP: i32 = 2;
const MATCH_GAP_PENALTY: i32 = 1;
const MATCH_GAP_MAX: i32 = 8;
const PATH_MATCH_PENALTY: i32 = 10;

/// A searchable file of the selected project.
pub(crate) struct FileCandidate {
    pub name: String,
    /// Directory path relative to the project root, using forward slashes.
    pub directory: String,
    pub path: PathBuf,
}

/// Score `query` as a subsequence of `text`.
///
/// Matching ignores ASCII case while the query holds no uppercase character.
/// Word starts and consecutive matches score higher. Characters skipped
/// between matches lower the score. Returns `None` for an empty query or when
/// `text` does not contain the query subsequence.
pub(crate) fn score(query: &str, text: &str) -> Option<i32> {
    score_chars(query, text.chars())
}

/// Rank files whose name or project-relative path matches `query`.
///
/// Name matches outrank path matches. Equal scores sort by name, then path,
/// so the order stays stable between keystrokes. Returns at most
/// [`MAX_RESULTS`] indices in ranked order.
pub(crate) fn rank(query: &str, files: &[FileCandidate]) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut ranked: Vec<(i32, usize)> = files
        .iter()
        .enumerate()
        .filter_map(|(index, file)| {
            let score = score_chars(query, file.name.chars()).or_else(|| {
                score_chars(query, path_chars(file)).map(|score| score - PATH_MATCH_PENALTY)
            })?;
            Some((score, index))
        })
        .collect();
    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| files[left.1].name.cmp(&files[right.1].name))
            .then_with(|| files[left.1].path.cmp(&files[right.1].path))
    });
    ranked.truncate(MAX_RESULTS);
    ranked.into_iter().map(|(_, index)| index).collect()
}

fn path_chars(file: &FileCandidate) -> impl Iterator<Item = char> + '_ {
    file.directory
        .chars()
        .chain(std::iter::once('/'))
        .chain(file.name.chars())
}

fn score_chars(query: &str, mut text: impl Iterator<Item = char>) -> Option<i32> {
    if query.is_empty() {
        return None;
    }
    let sensitive = query.chars().any(char::is_uppercase);
    let mut score = 0;
    let mut previous_match: Option<i32> = None;
    let mut previous_char: Option<char> = None;
    let mut streak = 0;
    let mut cursor: i32 = 0;
    for query_char in query.chars() {
        let mut matched: Option<i32> = None;
        let mut matched_char = '\0';
        for text_char in text.by_ref() {
            let equal = if sensitive {
                text_char == query_char
            } else {
                text_char.eq_ignore_ascii_case(&query_char)
            };
            if equal {
                matched = Some(cursor);
                matched_char = text_char;
                break;
            }
            previous_char = Some(text_char);
            cursor += 1;
        }
        let index = matched?;
        if previous_match == Some(index - 1) {
            streak += 1;
            score += MATCH_CONSECUTIVE_BONUS + MATCH_STREAK_STEP * streak;
        } else {
            streak = 0;
            if index == 0 {
                score += MATCH_START_BONUS;
            } else if previous_char.is_some_and(boundary) {
                score += MATCH_BOUNDARY_BONUS;
            } else if let Some(previous) = previous_match {
                score -= (index - previous - 1).min(MATCH_GAP_MAX) * MATCH_GAP_PENALTY;
            }
        }
        previous_match = Some(index);
        previous_char = Some(matched_char);
        cursor = index + 1;
    }
    Some(score)
}

fn boundary(character: char) -> bool {
    !character.is_alphanumeric()
}

#[cfg(test)]
mod tests;
