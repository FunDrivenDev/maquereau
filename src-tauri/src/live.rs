//! What the app reads from outside — Linear, GitHub, herdr — and keeps in `live.json` so a
//! new launch shows it at once. None of it is undoable: it is read, never changed here.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::herdr::Session;
use crate::model::{Initiative, Stage, Time, Topic};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Live {
    /// By link URL (or Linear key, as the link holds it).
    pub links: BTreeMap<String, LinkStatus>,
    /// The open Linear issues assigned to Raphaël.
    pub assigned: Vec<Issue>,
    /// Their sub-issues, whoever they are assigned to, done ones included.
    pub sub_issues: Vec<Issue>,
    pub initiatives: Vec<Initiative>,
    /// By herdr session name.
    pub sessions: BTreeMap<String, Session>,
    /// When Linear and GitHub were last read.
    pub refreshed_at: Option<Time>,
    /// What went wrong on the last read, one line per source.
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkStatus {
    pub title: String,
    /// The state as its source names it: `In Review`, `Merged`…
    pub state: String,
    pub tone: Tone,
    /// The state's colour in its source, `#rrggbb`, when it has one (Linear).
    pub color: Option<String>,
    pub url: String,
    /// Comments and reviews by others.
    pub comments: u32,
    pub last_comment_at: Option<String>,
    pub initiatives: Vec<Initiative>,
}

/// How a state reads at a glance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    #[default]
    Open,
    Progress,
    Review,
    Attention,
    Done,
    Closed,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Issue {
    pub key: String,
    pub title: String,
    pub url: String,
    pub state: String,
    pub state_type: StateType,
    pub tone: Tone,
    /// The state's colour in Linear, `#rrggbb`.
    pub state_color: String,
    /// Linear's priority: 0 none, 1 urgent … 4 low.
    pub priority: u8,
    pub initiatives: Vec<Initiative>,
    /// Markdown, as written in Linear.
    pub description: String,
    pub project: Option<String>,
    pub labels: Vec<Label>,
    pub estimate: Option<f64>,
    /// `YYYY-MM-DD`.
    pub due_date: Option<String>,
    /// ISO 8601.
    pub created_at: String,
    /// ISO 8601.
    pub updated_at: String,
    pub parent: Option<IssueRef>,
    pub assignee: Option<String>,
    /// Assigned to Raphaël.
    pub mine: bool,
}

/// The type Linear gives every workflow state, whatever its name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateType {
    Triage,
    #[default]
    Backlog,
    /// Ready: to do, not started.
    Unstarted,
    Started,
    Completed,
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    /// `#rrggbb`.
    pub color: String,
    /// The label group it belongs to (`Type`, `App`…).
    pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueRef {
    pub key: String,
    pub title: String,
}

/// Something worth a notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    pub title: String,
    pub body: String,
}

/// An initiative a topic could be linked to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Suggestion {
    pub initiative: Initiative,
    /// One of the topic's linked issues belongs to it.
    pub from_issues: bool,
}

/// The initiatives `topic` could be linked to, each once: those of its linked issues
/// first, in the order of its links, then the others read from Linear.
pub fn suggestions(topic: &Topic, live: &Live) -> Vec<Suggestion> {
    let from_issues = topic
        .links
        .iter()
        .filter_map(|l| live.links.get(&l.url))
        .flat_map(|s| &s.initiatives)
        .map(|i| (i, true));
    let others = live.initiatives.iter().map(|i| (i, false));
    let mut out: Vec<Suggestion> = Vec::new();
    for (initiative, from_issues) in from_issues.chain(others) {
        if !out.iter().any(|s| s.initiative.id == initiative.id) {
            out.push(Suggestion {
                initiative: initiative.clone(),
                from_issues,
            });
        }
    }
    out
}

/// Topics worth reading about: those not done.
pub fn open_topics(topics: &[Topic]) -> impl Iterator<Item = &Topic> {
    topics.iter().filter(|t| t.stage != Stage::Done)
}

/// What changed from `before` to `after` that Raphaël should hear about: new comments by
/// others on a topic's links, and agents that now wait for him or have finished.
pub fn alerts(
    topics: &[Topic],
    before: &Live,
    after: &Live,
    comments: bool,
    sessions: bool,
) -> Vec<Alert> {
    let mut alerts = Vec::new();
    for topic in open_topics(topics) {
        if comments {
            for link in &topic.links {
                let (Some(old), Some(new)) =
                    (before.links.get(&link.url), after.links.get(&link.url))
                else {
                    continue;
                };
                if new.comments > old.comments {
                    alerts.push(Alert {
                        title: format!("New comment · {}", topic.title),
                        body: new.title.clone(),
                    });
                }
            }
        }
        if sessions {
            let old = before.sessions.get(&topic.session);
            let new = after.sessions.get(&topic.session);
            for agent in new.into_iter().flat_map(|s| &s.agents) {
                let was = old
                    .into_iter()
                    .flat_map(|s| &s.agents)
                    .find(|a| a.name == agent.name)
                    .map(|a| a.status.as_str());
                if was == Some(agent.status.as_str()) {
                    continue;
                }
                let body = match agent.status.as_str() {
                    "blocked" => format!("{} needs you", agent.name),
                    "done" => format!("{} has finished", agent.name),
                    _ => continue,
                };
                alerts.push(Alert {
                    title: topic.title.clone(),
                    body,
                });
            }
        }
    }
    alerts
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::herdr::Agent;
    use crate::links::Kind;
    use crate::model::tests::topic;
    use crate::model::{Link, Model, Slot};

    pub fn status(comments: u32) -> LinkStatus {
        LinkStatus {
            title: "BIM-1 Export".into(),
            state: "In Progress".into(),
            tone: Tone::Progress,
            color: None,
            url: "u".into(),
            comments,
            last_comment_at: None,
            initiatives: vec![],
        }
    }

    pub fn with_agent(status: &str) -> Live {
        let mut live = Live::default();
        live.sessions.insert(
            "mq-export".into(),
            Session {
                running: true,
                agents: vec![Agent {
                    name: "claude".into(),
                    status: status.into(),
                }],
            },
        );
        live
    }

    #[test]
    fn a_new_comment_by_someone_else_is_an_alert() {
        let mut model = Model::default();
        let mut t = topic(&mut model, "Export", Slot::Feature);
        t.links.push(Link {
            id: 9,
            url: "BIM-1".into(),
            kind: Kind::LinearIssue,
        });
        let mut before = Live::default();
        before.links.insert("BIM-1".into(), status(1));
        let mut after = before.clone();
        assert!(alerts(&[t.clone()], &before, &after, true, true).is_empty());
        after.links.insert("BIM-1".into(), status(2));
        let found = alerts(&[t.clone()], &before, &after, true, true);
        assert_eq!(found[0].title, "New comment · Export");
        assert!(alerts(&[t], &before, &after, false, true).is_empty());
    }

    #[test]
    fn suggests_the_initiatives_of_the_linked_issues_first_each_once() {
        let initiative = |id: &str| Initiative {
            id: id.into(),
            name: id.into(),
            url: String::new(),
        };
        let mut model = Model::default();
        let mut t = topic(&mut model, "Export", Slot::Feature);
        for (id, url) in [(9, "BIM-1"), (10, "BIM-2"), (11, "BIM-3")] {
            t.links.push(Link {
                id,
                url: url.into(),
                kind: Kind::LinearIssue,
            });
        }
        let mut live = Live {
            initiatives: vec![initiative("a"), initiative("b"), initiative("c")],
            ..Live::default()
        };
        assert!(suggestions(&t, &live).iter().all(|s| !s.from_issues));
        let mut one = status(0);
        one.initiatives = vec![initiative("c"), initiative("d")];
        let mut two = status(0);
        two.initiatives = vec![initiative("d"), initiative("a")];
        live.links.insert("BIM-1".into(), one);
        live.links.insert("BIM-2".into(), two);
        let found = suggestions(&t, &live);
        let found: Vec<(&str, bool)> = found
            .iter()
            .map(|s| (s.initiative.id.as_str(), s.from_issues))
            .collect();
        assert_eq!(found, [("c", true), ("d", true), ("a", true), ("b", false)]);
    }

    #[test]
    fn an_agent_turning_blocked_or_done_is_an_alert_once() {
        let mut model = Model::default();
        let t = topic(&mut model, "Export", Slot::Feature);
        let topics = [t];
        let working = with_agent("working");
        let blocked = with_agent("blocked");
        let found = alerts(&topics, &working, &blocked, true, true);
        assert_eq!(found[0].body, "claude needs you");
        assert!(alerts(&topics, &blocked, &blocked, true, true).is_empty());
        let done = alerts(&topics, &working, &with_agent("done"), true, true);
        assert_eq!(done[0].body, "claude has finished");
        assert!(alerts(&topics, &working, &blocked, true, false).is_empty());
    }
}
