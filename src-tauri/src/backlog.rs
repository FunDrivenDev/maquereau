//! The Linear issues of the backlog, as a tree of issues and their sub-issues, and the
//! size of the backlog.

use serde::Serialize;

use crate::live::{Issue, StateType};
use crate::model::{Stage, Topic};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    pub issue: Issue,
    /// 0 for an issue shown at the top, 1 for its sub-issues…
    pub depth: u8,
    /// Its sub-issues listed right after it.
    pub children: u32,
}

/// The issues assigned to Raphaël and their sub-issues, but those `linked` by an open
/// topic, in preorder: each issue followed by its sub-issues, and issues side by side in
/// the order of `rank`, then in the order given. An assigned issue shows under the
/// nearest of its listed ancestors that is assigned too, and at the top when none is: so
/// every assigned issue shows exactly once, even when the issue between it and an
/// assigned ancestor is someone else's and that ancestor is linked. Any other sub-issue
/// shows only under its parent.
pub fn tree(assigned: &[Issue], sub_issues: &[Issue], linked: &[String]) -> Vec<Entry> {
    let mut pool: Vec<&Issue> = Vec::new();
    for issue in assigned.iter().chain(sub_issues) {
        if !linked.contains(&issue.key) && !pool.iter().any(|i| i.key == issue.key) {
            pool.push(issue);
        }
    }
    pool.sort_by_key(|i| rank(i));
    let mut out = Vec::new();
    let mine = |issue: &Issue| issue.mine || assigned.iter().any(|i| i.key == issue.key);
    for root in &pool {
        if mine(root) && !ancestors(&pool, root).any(&mine) {
            walk(&pool, root, 0, &mut out);
        }
    }
    out
}

/// `issue`'s parent, its parent's parent…, as long as each is in `pool`.
fn ancestors<'a>(pool: &'a [&'a Issue], issue: &'a Issue) -> impl Iterator<Item = &'a Issue> {
    let mut next = Some(issue);
    // A parent cycle cannot come from Linear; the bound keeps one from looping anyway.
    std::iter::from_fn(move || {
        let key = parent(pool, next?)?;
        next = pool.iter().copied().find(|i| i.key == key);
        next
    })
    .take(pool.len())
}

/// How much the backlog holds to pull: the queued topics, plus the issues assigned to
/// Raphaël, neither done nor canceled, that no open topic `linked`. The sub-issues listed
/// in the tree only as someone else's, or done, do not count.
pub fn count(
    topics: &[Topic],
    assigned: &[Issue],
    sub_issues: &[Issue],
    linked: &[String],
) -> usize {
    let queued = topics.iter().filter(|t| t.stage == Stage::Queued).count();
    let mut keys: Vec<&str> = Vec::new();
    for issue in assigned.iter().chain(sub_issues.iter().filter(|i| i.mine)) {
        let open = !matches!(issue.state_type, StateType::Completed | StateType::Canceled);
        if open && !linked.contains(&issue.key) && !keys.contains(&issue.key.as_str()) {
            keys.push(&issue.key);
        }
    }
    queued + keys.len()
}

/// Where an issue goes among its siblings: the most urgent first, then in progress, ready,
/// in the backlog; an issue without priority after the low ones. Done issues go last
/// whatever their priority, as nothing is left to pull in them.
fn rank(issue: &Issue) -> (bool, u8, u8) {
    let priority = match issue.priority {
        0 => 5,
        p => p,
    };
    let state = match issue.state_type {
        StateType::Started => 0,
        StateType::Unstarted => 1,
        StateType::Triage | StateType::Backlog => 2,
        StateType::Completed | StateType::Canceled => 3,
    };
    (state == 3, priority, state)
}

/// The key of `issue`'s parent, when it is in `pool`.
fn parent<'a>(pool: &[&Issue], issue: &'a Issue) -> Option<&'a str> {
    let key = issue.parent.as_ref()?.key.as_str();
    pool.iter().any(|i| i.key == key).then_some(key)
}

fn walk(pool: &[&Issue], issue: &Issue, depth: u8, out: &mut Vec<Entry>) {
    if out.iter().any(|e| e.issue.key == issue.key) {
        return;
    }
    let at = out.len();
    out.push(Entry {
        issue: issue.clone(),
        depth,
        children: 0,
    });
    let mut children = 0;
    for child in pool
        .iter()
        .filter(|i| parent(pool, i) == Some(issue.key.as_str()))
    {
        let before = out.len();
        walk(pool, child, depth + 1, out);
        children += u32::from(out.len() > before);
    }
    out[at].children = children;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::IssueRef;
    use crate::model::tests::topic;
    use crate::model::{Model, Slot};

    fn issue(key: &str, parent: Option<&str>) -> Issue {
        Issue {
            key: key.into(),
            title: key.into(),
            parent: parent.map(|p| IssueRef {
                key: p.into(),
                title: p.into(),
            }),
            ..Issue::default()
        }
    }

    fn shape(entries: &[Entry]) -> Vec<(&str, u8, u32)> {
        entries
            .iter()
            .map(|e| (e.issue.key.as_str(), e.depth, e.children))
            .collect()
    }

    #[test]
    fn sub_issues_follow_their_parent() {
        let assigned = [
            issue("A", None),
            issue("B", None),
            issue("A2", Some("A")),
            issue("C", Some("X")),
        ];
        let subs = [
            issue("A1", Some("A")),
            issue("A1a", Some("A1")),
            issue("A2", Some("A")),
            issue("Y1", Some("Y")),
        ];
        assert_eq!(
            shape(&tree(&assigned, &subs, &[])),
            [
                ("A", 0, 2),
                ("A2", 1, 0),
                ("A1", 1, 1),
                ("A1a", 2, 0),
                ("B", 0, 0),
                ("C", 0, 0),
            ]
        );
    }

    fn ranked(key: &str, priority: u8, state_type: StateType) -> Issue {
        Issue {
            priority,
            state_type,
            ..issue(key, None)
        }
    }

    #[test]
    fn the_most_urgent_come_first_then_in_progress_ready_backlog_and_done_last() {
        let assigned = [
            ranked("none", 0, StateType::Started),
            ranked("low", 4, StateType::Started),
            ranked("high-backlog", 2, StateType::Backlog),
            ranked("high-ready", 2, StateType::Unstarted),
            ranked("urgent", 1, StateType::Triage),
            ranked("high-started", 2, StateType::Started),
        ];
        let subs = [
            Issue {
                parent: issue("x", Some("urgent")).parent,
                ..ranked("urgent-done", 1, StateType::Completed)
            },
            Issue {
                parent: issue("x", Some("urgent")).parent,
                ..ranked("urgent-ready", 3, StateType::Unstarted)
            },
        ];
        assert_eq!(
            shape(&tree(&assigned, &subs, &[])),
            [
                ("urgent", 0, 2),
                ("urgent-ready", 1, 0),
                ("urgent-done", 1, 0),
                ("high-started", 0, 0),
                ("high-ready", 0, 0),
                ("high-backlog", 0, 0),
                ("low", 0, 0),
                ("none", 0, 0),
            ]
        );
    }

    #[test]
    fn a_linked_issue_leaves_with_the_sub_issues_only_it_listed() {
        let assigned = [issue("A", None), issue("A2", Some("A"))];
        let subs = [issue("A1", Some("A"))];
        assert_eq!(
            shape(&tree(&assigned, &subs, &["A".into()])),
            [("A2", 0, 0)]
        );
    }

    #[test]
    fn an_assigned_issue_under_someone_elses_shows_when_the_assigned_ancestor_is_linked() {
        // A and A1a are Raphaël's, A1 someone else's, and an open topic links A.
        let assigned = [issue("A", None), issue("A1a", Some("A1"))];
        let subs = [issue("A1", Some("A")), issue("A1a", Some("A1"))];
        assert_eq!(
            shape(&tree(&assigned, &subs, &["A".into()])),
            [("A1a", 0, 0)]
        );
        // Unlinked, A1a shows once, under A1 under A.
        assert_eq!(
            shape(&tree(&assigned, &subs, &[])),
            [("A", 0, 1), ("A1", 1, 1), ("A1a", 2, 0)]
        );
    }

    #[test]
    fn the_count_is_the_queued_topics_and_the_open_unlinked_issues_assigned_to_raphael() {
        let mut model = Model::default();
        let queued = topic(&mut model, "queued", Slot::Feature);
        let active = Topic {
            stage: Stage::Active,
            ..topic(&mut model, "active", Slot::BugRun)
        };
        let done = Topic {
            stage: Stage::Done,
            ..topic(&mut model, "done", Slot::Tooling)
        };
        let assigned = [
            issue("A", None),
            issue("B", None),
            Issue {
                state_type: StateType::Canceled,
                ..issue("C", None)
            },
        ];
        let subs = [
            // Someone else's, in the tree under A.
            issue("A1", Some("A")),
            // Raphaël's, also in `assigned`: counted once.
            Issue {
                mine: true,
                ..issue("B", None)
            },
            Issue {
                mine: true,
                ..issue("A2", Some("A"))
            },
            Issue {
                mine: true,
                state_type: StateType::Completed,
                ..issue("A3", Some("A"))
            },
        ];
        assert_eq!(
            count(&[queued, active, done], &assigned, &subs, &["B".into()]),
            1 + 2 // queued; A and A2
        );
    }
}
