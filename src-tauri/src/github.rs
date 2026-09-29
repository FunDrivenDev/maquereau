//! Pull requests and GitHub issues, read through `gh`, which holds Raphaël's login.

use serde_json::Value;

use crate::links::Kind;
use crate::live::{LinkStatus, Tone};
use crate::shell::Shell;

/// The login `gh` is signed in as.
pub fn me(shell: &Shell) -> Result<String, String> {
    let login = shell.run("gh", &["api", "user", "--jq", ".login"], None)?;
    Ok(login.trim().to_owned())
}

pub fn status(shell: &Shell, url: &str, kind: Kind, me: &str) -> Result<LinkStatus, String> {
    let json = match kind {
        Kind::PullRequest => shell.run(
            "gh",
            &[
                "pr",
                "view",
                url,
                "--json",
                "title,state,isDraft,reviewDecision,comments,reviews,url",
            ],
            None,
        )?,
        _ => shell.run(
            "gh",
            &["issue", "view", url, "--json", "title,state,comments,url"],
            None,
        )?,
    };
    let value: Value = serde_json::from_str(&json).map_err(|e| format!("gh: {e}"))?;
    Ok(read(&value, me))
}

fn read(value: &Value, me: &str) -> LinkStatus {
    let text = |key: &str| value[key].as_str().unwrap_or_default().to_owned();
    let state = text("state");
    let decision = text("reviewDecision");
    let (label, tone) = match (
        state.as_str(),
        value["isDraft"].as_bool(),
        decision.as_str(),
    ) {
        ("MERGED", ..) => ("Merged", Tone::Done),
        ("CLOSED", ..) => ("Closed", Tone::Closed),
        (_, Some(true), _) => ("Draft", Tone::Progress),
        (_, _, "CHANGES_REQUESTED") => ("Changes requested", Tone::Attention),
        (_, _, "APPROVED") => ("Approved", Tone::Review),
        (_, Some(false), _) => ("In review", Tone::Review),
        _ => ("Open", Tone::Open),
    };
    let comments = value["comments"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["viewerDidAuthor"].as_bool() != Some(true))
        .map(|c| (&c["author"]["login"], &c["createdAt"]));
    let reviews = value["reviews"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| (&r["author"]["login"], &r["submittedAt"]));
    let others: Vec<&str> = comments
        .chain(reviews)
        .filter(|(login, _)| {
            let login = login.as_str().unwrap_or_default();
            login != me && !is_bot(login)
        })
        .filter_map(|(_, at)| at.as_str())
        .collect();
    LinkStatus {
        title: text("title"),
        state: label.into(),
        tone,
        color: None,
        url: text("url"),
        comments: others.len() as u32,
        last_comment_at: others.iter().max().map(|at| (*at).to_owned()),
        initiatives: vec![],
    }
}

fn is_bot(login: &str) -> bool {
    login.ends_with("[bot]") || login == "github-actions" || login.ends_with("-bot")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_the_comments_and_reviews_of_others() {
        let value = serde_json::json!({
            "title": "Export PDF",
            "state": "OPEN",
            "isDraft": false,
            "reviewDecision": "CHANGES_REQUESTED",
            "url": "https://github.com/o/r/pull/1",
            "comments": [
                {"author": {"login": "me"}, "createdAt": "2026-09-01T00:00:00Z", "viewerDidAuthor": true},
                {"author": {"login": "github-actions"}, "createdAt": "2026-09-02T00:00:00Z", "viewerDidAuthor": false},
                {"author": {"login": "ana"}, "createdAt": "2026-09-03T00:00:00Z", "viewerDidAuthor": false}
            ],
            "reviews": [
                {"author": {"login": "bob"}, "submittedAt": "2026-09-04T00:00:00Z", "state": "CHANGES_REQUESTED"},
                {"author": {"login": "me"}, "submittedAt": "2026-09-05T00:00:00Z", "state": "COMMENTED"}
            ]
        });
        let status = read(&value, "me");
        assert_eq!(status.state, "Changes requested");
        assert_eq!(status.tone, Tone::Attention);
        assert_eq!(status.comments, 2);
        assert_eq!(
            status.last_comment_at.as_deref(),
            Some("2026-09-04T00:00:00Z")
        );
    }

    #[test]
    fn reads_the_state_of_an_issue() {
        let value =
            serde_json::json!({"title": "t", "state": "CLOSED", "comments": [], "url": "u"});
        let status = read(&value, "me");
        assert_eq!(
            (status.state.as_str(), status.tone),
            ("Closed", Tone::Closed)
        );
        assert_eq!(status.comments, 0);
    }
}
