//! Linear, read through its GraphQL API with Raphaël's personal API key.
//!
//! The key lives in the macOS keychain. Requests go through `curl`, their headers and
//! body written to its standard input as a config file, so the key never shows in the
//! process list.

use serde_json::{Value, json};

use crate::live::{Issue, IssueRef, Label, LinkStatus, Tone};
use crate::model::Initiative;
use crate::shell::Shell;

const SERVICE: &str = "dev.fundrivendev.maquereau";
const ACCOUNT: &str = "linear";

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())
}

/// The API key, when one is set.
pub fn key() -> Option<String> {
    entry().ok()?.get_password().ok().filter(|k| !k.is_empty())
}

/// Saves `key` in the keychain, or forgets it when blank.
pub fn set_key(key: &str) -> Result<(), String> {
    let entry = entry()?;
    let key = key.trim();
    if key.is_empty() {
        return match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    entry.set_password(key).map_err(|e| e.to_string())
}

const INITIATIVES: &str = "project { initiatives(first: 3) { nodes { id name url } } }";

/// The open issues assigned to Raphaël, most recently updated first.
pub fn assigned(shell: &Shell, key: &str) -> Result<Vec<Issue>, String> {
    let query = format!(
        "query {{ viewer {{ assignedIssues(first: 100, orderBy: updatedAt, filter: \
         {{ state: {{ type: {{ nin: [\"completed\", \"canceled\"] }} }} }}) \
         {{ nodes {{ {ISSUE} }} }} }} }}"
    );
    let data = request(shell, key, &query, json!({}))?;
    Ok(data["viewer"]["assignedIssues"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(read_listed)
        .collect())
}

/// The sub-issues, two levels down, of the open issues assigned to Raphaël, done ones
/// included; those assigned to him come back too.
pub fn sub_issues(shell: &Shell, key: &str) -> Result<Vec<Issue>, String> {
    let mine = "{ assignee: { isMe: { eq: true } }, \
         state: { type: { nin: [\"completed\", \"canceled\"] } } }";
    let query = format!(
        "query {{ issues(first: 100, orderBy: updatedAt, filter: {{ \
         state: {{ type: {{ neq: \"canceled\" }} }}, \
         or: [{{ parent: {mine} }}, {{ parent: {{ parent: {mine} }} }}] }}) \
         {{ nodes {{ {ISSUE} }} }} }}"
    );
    let data = request(shell, key, &query, json!({}))?;
    Ok(data["issues"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(read_listed)
        .collect())
}

/// What the backlog shows of an issue.
const ISSUE: &str = "identifier title url priority description estimate dueDate createdAt \
     updatedAt state { name type } labels(first: 10) { nodes { name color } } \
     parent { identifier title } assignee { name isMe } project { name initiatives(first: 3) { nodes { id name url } } }";

fn read_listed(node: &Value) -> Issue {
    Issue {
        key: text(node, "identifier"),
        title: text(node, "title"),
        url: text(node, "url"),
        state: text(&node["state"], "name"),
        tone: tone(&node["state"]),
        priority: node["priority"].as_f64().unwrap_or_default() as u8,
        initiatives: initiatives(node),
        description: text(node, "description"),
        project: node["project"]["name"].as_str().map(str::to_owned),
        labels: node["labels"]["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|l| Label {
                name: text(l, "name"),
                color: text(l, "color"),
            })
            .collect(),
        estimate: node["estimate"].as_f64(),
        due_date: node["dueDate"].as_str().map(str::to_owned),
        created_at: text(node, "createdAt"),
        updated_at: text(node, "updatedAt"),
        assignee: node["assignee"]["name"].as_str().map(str::to_owned),
        mine: node["assignee"]["isMe"].as_bool().unwrap_or_default(),
        parent: node["parent"].is_object().then(|| IssueRef {
            key: text(&node["parent"], "identifier"),
            title: text(&node["parent"], "title"),
        }),
    }
}

/// The state of issue `key` (`BIM-123`).
pub fn status(shell: &Shell, api_key: &str, key: &str) -> Result<LinkStatus, String> {
    let query = format!(
        "query($id: String!) {{ issue(id: $id) {{ identifier title url state {{ name type }} \
         comments(first: 100) {{ nodes {{ createdAt user {{ isMe }} }} }} {INITIATIVES} }} }}"
    );
    let data = request(shell, api_key, &query, json!({ "id": key }))?;
    Ok(read_issue(&data["issue"]))
}

/// The initiatives still open, by name.
pub fn open_initiatives(shell: &Shell, key: &str) -> Result<Vec<Initiative>, String> {
    let query = "query { initiatives(first: 100) { nodes { id name url status } } }";
    let data = request(shell, key, query, json!({}))?;
    let mut list: Vec<Initiative> = data["initiatives"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| !matches!(n["status"].as_str(), Some("Completed" | "Canceled")))
        .map(initiative)
        .collect();
    list.sort_by_key(|i| i.name.to_lowercase());
    Ok(list)
}

/// How a Linear `state { name type }` reads.
fn tone(state: &Value) -> Tone {
    match state["type"].as_str() {
        Some("completed") => Tone::Done,
        Some("canceled") => Tone::Closed,
        Some("started") if text(state, "name").to_lowercase().contains("review") => Tone::Review,
        Some("started") => Tone::Progress,
        _ => Tone::Open,
    }
}

fn read_issue(issue: &Value) -> LinkStatus {
    let state = text(&issue["state"], "name");
    let tone = tone(&issue["state"]);
    let others: Vec<&str> = issue["comments"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["user"]["isMe"].as_bool() == Some(false))
        .filter_map(|c| c["createdAt"].as_str())
        .collect();
    LinkStatus {
        title: format!("{} {}", text(issue, "identifier"), text(issue, "title")),
        state,
        tone,
        url: text(issue, "url"),
        comments: others.len() as u32,
        last_comment_at: others.iter().max().map(|at| (*at).to_owned()),
        initiatives: initiatives(issue),
    }
}

fn initiatives(node: &Value) -> Vec<Initiative> {
    node["project"]["initiatives"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(initiative)
        .collect()
}

fn initiative(node: &Value) -> Initiative {
    Initiative {
        id: text(node, "id"),
        name: text(node, "name"),
        url: text(node, "url"),
    }
}

fn text(node: &Value, key: &str) -> String {
    node[key].as_str().unwrap_or_default().to_owned()
}

/// Posts a GraphQL query and returns its `data`, or the first error.
fn request(shell: &Shell, key: &str, query: &str, variables: Value) -> Result<Value, String> {
    let body = json!({ "query": query, "variables": variables }).to_string();
    let config = format!(
        "url = \"https://api.linear.app/graphql\"\n\
         silent\nshow-error\nmax-time = 30\n\
         header = \"Content-Type: application/json\"\n\
         header = {}\n\
         data-binary = {}\n",
        curl_string(&format!("Authorization: {key}")),
        curl_string(&body),
    );
    let out = shell.run("curl", &["--config", "-"], Some(&config))?;
    let value: Value = serde_json::from_str(&out).map_err(|e| format!("Linear: {e}"))?;
    if let Some(message) = value["errors"][0]["message"].as_str() {
        return Err(format!("Linear: {message}"));
    }
    Ok(value["data"].clone())
}

/// `text` as a double-quoted string of a curl config file.
fn curl_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '\t' => out.push_str(r"\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_for_a_curl_config() {
        assert_eq!(curl_string(r#"{"a":"b\n"}"#), r#""{\"a\":\"b\\n\"}""#);
        assert_eq!(curl_string("x\ny"), r#""x\ny""#);
    }

    #[test]
    fn reads_an_issue() {
        let issue = json!({
            "identifier": "BIM-7",
            "title": "Export PDF",
            "url": "https://linear.app/ws/issue/BIM-7",
            "state": {"name": "In Review", "type": "started"},
            "comments": {"nodes": [
                {"createdAt": "2026-09-01T00:00:00Z", "user": {"isMe": true}},
                {"createdAt": "2026-09-02T00:00:00Z", "user": null},
                {"createdAt": "2026-09-03T00:00:00Z", "user": {"isMe": false}}
            ]},
            "project": {"initiatives": {"nodes": [{"id": "i1", "name": "Reports", "url": "u"}]}}
        });
        let status = read_issue(&issue);
        assert_eq!(status.title, "BIM-7 Export PDF");
        assert_eq!(status.tone, Tone::Review);
        assert_eq!(status.comments, 1);
        assert_eq!(status.initiatives[0].name, "Reports");
    }

    #[test]
    fn reads_a_listed_issue() {
        let node = json!({
            "identifier": "BIM-8",
            "title": "Export CSV",
            "url": "https://linear.app/ws/issue/BIM-8",
            "priority": 2,
            "description": "## Why\n- faster",
            "estimate": 3,
            "dueDate": null,
            "createdAt": "2026-09-01T00:00:00.000Z",
            "updatedAt": "2026-09-02T00:00:00.000Z",
            "state": {"name": "In Progress", "type": "started"},
            "labels": {"nodes": [{"name": "Bug", "color": "#eb5757"}]},
            "parent": {"identifier": "BIM-7", "title": "Export PDF"},
            "project": {"name": "Reports", "initiatives": {"nodes": []}}
        });
        let issue = read_listed(&node);
        assert_eq!(issue.tone, Tone::Progress);
        assert_eq!(issue.priority, 2);
        assert_eq!(issue.estimate, Some(3.0));
        assert_eq!(issue.due_date, None);
        assert_eq!(issue.project.as_deref(), Some("Reports"));
        assert_eq!(issue.labels[0].name, "Bug");
        assert_eq!(issue.parent.unwrap().key, "BIM-7");
        let bare = read_listed(&json!({"identifier": "BIM-9", "parent": null, "project": null}));
        assert_eq!(bare.parent, None);
        assert_eq!(bare.project, None);
    }

    #[test]
    fn an_issue_saved_before_its_details_still_loads() {
        let issue: Issue = serde_json::from_value(json!({
            "key": "BIM-1", "title": "t", "url": "u", "state": "Todo", "priority": 3, "initiatives": []
        }))
        .unwrap();
        assert_eq!(issue.tone, Tone::Open);
        assert!(issue.description.is_empty());
    }
}
