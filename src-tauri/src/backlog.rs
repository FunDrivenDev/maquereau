//! The Linear issues of the backlog, as a tree of issues and their sub-issues.

use serde::Serialize;

use crate::live::Issue;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    pub issue: Issue,
    /// 0 for an issue shown at the top, 1 for its sub-issues…
    pub depth: u8,
    /// Its sub-issues listed right after it.
    pub children: u32,
}

/// The issues assigned to Raphaël and their sub-issues, but those `linked` by an open
/// topic, in preorder: each issue followed by its sub-issues, in the order given. An
/// assigned issue whose parent is not listed shows at the top; any other sub-issue shows
/// only under its parent.
pub fn tree(assigned: &[Issue], sub_issues: &[Issue], linked: &[String]) -> Vec<Entry> {
    let mut pool: Vec<&Issue> = Vec::new();
    for issue in assigned.iter().chain(sub_issues) {
        if !linked.contains(&issue.key) && !pool.iter().any(|i| i.key == issue.key) {
            pool.push(issue);
        }
    }
    let mut out = Vec::new();
    for root in &pool {
        let assigned = assigned.iter().any(|i| i.key == root.key);
        if assigned && parent(&pool, root).is_none() {
            walk(&pool, root, 0, &mut out);
        }
    }
    out
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

    #[test]
    fn a_linked_issue_leaves_with_the_sub_issues_only_it_listed() {
        let assigned = [issue("A", None), issue("A2", Some("A"))];
        let subs = [issue("A1", Some("A"))];
        assert_eq!(
            shape(&tree(&assigned, &subs, &["A".into()])),
            [("A2", 0, 0)]
        );
    }
}
