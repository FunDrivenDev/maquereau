//! What a link attached to a topic points at, read from its URL alone.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    LinearIssue,
    PullRequest,
    GithubIssue,
    Slack,
    Web,
}

/// A link as typed or pasted, cleaned up: its URL (or the Linear key, uppercased) and
/// its kind. `None` when it is neither a URL nor a Linear key.
pub fn parse(text: &str) -> Option<(String, Kind)> {
    let text = text.trim();
    if let Some(key) = linear_key(text) {
        return Some((key, Kind::LinearIssue));
    }
    let rest = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    if host.is_empty() {
        return None;
    }
    let segments: Vec<&str> = path.split(['/', '?', '#']).collect();
    let kind = match (host, segments.as_slice()) {
        ("linear.app", [_, "issue", key, ..]) if linear_key(key).is_some() => Kind::LinearIssue,
        ("github.com", [_, _, "pull", n, ..]) if is_number(n) => Kind::PullRequest,
        ("github.com", [_, _, "issues", n, ..]) if is_number(n) => Kind::GithubIssue,
        (host, _) if host == "slack.com" || host.ends_with(".slack.com") => Kind::Slack,
        _ => Kind::Web,
    };
    Some((text.to_owned(), kind))
}

/// The Linear key a link names (`BIM-123`), from a bare key or an issue URL.
pub fn key(url: &str) -> Option<String> {
    if let Some(key) = linear_key(url) {
        return Some(key);
    }
    let path = url.split_once("linear.app/")?.1;
    let mut segments = path.split('/');
    segments.next()?;
    (segments.next()? == "issue")
        .then(|| linear_key(segments.next()?))
        .flatten()
}

/// `text` as a Linear key, uppercased, when it is one: letters, a dash, digits.
fn linear_key(text: &str) -> Option<String> {
    let (team, number) = text.split_once('-')?;
    let team_ok = !team.is_empty()
        && team.len() <= 7
        && team.chars().next()?.is_ascii_alphabetic()
        && team.chars().all(|c| c.is_ascii_alphanumeric());
    (team_ok && is_number(number)).then(|| format!("{}-{number}", team.to_ascii_uppercase()))
}

fn is_number(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_kind_from_the_url() {
        let kind = |text| parse(text).map(|(_, kind)| kind);
        assert_eq!(kind("bim-12"), Some(Kind::LinearIssue));
        assert_eq!(
            kind("https://linear.app/guestsuite/issue/BIM-12/some-title"),
            Some(Kind::LinearIssue)
        );
        assert_eq!(
            kind("https://github.com/o/r/pull/7/files"),
            Some(Kind::PullRequest)
        );
        assert_eq!(
            kind("https://github.com/o/r/issues/7#issuecomment-1"),
            Some(Kind::GithubIssue)
        );
        assert_eq!(
            kind("https://guestsuite.slack.com/archives/C1/p2"),
            Some(Kind::Slack)
        );
        assert_eq!(kind("https://github.com/o/r"), Some(Kind::Web));
        assert_eq!(kind("hello world"), None);
        assert_eq!(kind("https://"), None);
    }

    #[test]
    fn finds_the_linear_key() {
        assert_eq!(parse("bim-12").unwrap().0, "BIM-12");
        assert_eq!(
            key("https://linear.app/ws/issue/BIM-12/title").as_deref(),
            Some("BIM-12")
        );
        assert_eq!(key("BIM-3").as_deref(), Some("BIM-3"));
        assert_eq!(key("https://github.com/o/r/pull/7"), None);
        assert_eq!(key("x-y"), None);
    }
}
