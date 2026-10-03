//! What the user does to the topics and the settings, with the rules each action keeps.
//!
//! An action reads the model, and what was last read from outside where it needs it, and
//! returns the `Change` it makes with its label; it changes nothing but the next id.
//! `History::perform` applies the change, so every action can be undone.

use crate::links;
use crate::live::Live;
use crate::model::{
    Block, Change, Initiative, Link, Model, Note, Rework, Slot, Stage, Step, Time, Topic,
    is_session_name,
};
use crate::settings::Settings;
use crate::shell;

/// A change to perform, and the label the undo and redo show for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub label: String,
    pub change: Change,
}

impl Action {
    fn new(label: impl Into<String>, change: Change) -> Self {
        Self {
            label: label.into(),
            change,
        }
    }
}

/// `text` trimmed, or an error saying `what` is needed when it is blank.
fn required(text: &str, what: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err(format!("{what} is needed"));
    }
    Ok(text.to_owned())
}

/// Changes topic `id` through `edit`, which returns the action's label.
fn edit(
    model: &mut Model,
    id: u64,
    edit: impl FnOnce(&mut Topic, &mut Model) -> Result<String, String>,
) -> Result<Action, String> {
    let mut topic = model.topic(id)?.clone();
    let label = edit(&mut topic, model)?;
    Ok(Action::new(label, Change::put(topic)))
}

/// Adds a topic to `slot`; `activate` makes it the slot's topic, else it waits in the
/// backlog.
pub fn create_topic(
    model: &mut Model,
    now: Time,
    title: &str,
    slot: Slot,
    activate: bool,
) -> Result<Action, String> {
    let title = required(title, "a title")?;
    let topic = model.new_topic(title.clone(), slot, now);
    let label = format!("Add “{title}”");
    if activate {
        return Ok(Action::new(
            label,
            Change::Batch(model.activation(topic, now)),
        ));
    }
    Ok(Action::new(label, Change::put(topic)))
}

/// Makes the active topic of `slot` from the assigned Linear issue `key`, or one of their
/// sub-issues: its title, a link to it, and its first initiative.
pub fn topic_from_issue(
    model: &mut Model,
    now: Time,
    live: &Live,
    key: &str,
    slot: Slot,
) -> Result<Action, String> {
    let issue = live
        .assigned
        .iter()
        .chain(&live.sub_issues)
        .find(|i| i.key == key)
        .ok_or_else(|| format!("no issue {key}"))?;
    let mut topic = model.new_topic(issue.title.clone(), slot, now);
    topic.links.push(Link {
        id: model.take_id(),
        url: issue.url.clone(),
        kind: links::Kind::LinearIssue,
    });
    topic.initiative = issue.initiatives.first().cloned();
    let changes = model.activation(topic, now);
    Ok(Action::new(
        format!("Focus on {key}"),
        Change::Batch(changes),
    ))
}

pub fn rename(model: &mut Model, id: u64, title: &str) -> Result<Action, String> {
    let title = required(title, "a title")?;
    edit(model, id, |topic, _| {
        let label = format!("Rename “{}”", topic.title);
        topic.title = title;
        Ok(label)
    })
}

/// Moves a topic to another slot. An active topic stays active there, parking the one it
/// replaces.
pub fn move_topic(model: &mut Model, now: Time, id: u64, slot: Slot) -> Result<Action, String> {
    let mut topic = model.topic(id)?.clone();
    let label = format!("Move “{}” to {}", topic.title, slot.label());
    let active = topic.stage == Stage::Active;
    topic.slot = slot;
    if active {
        return Ok(Action::new(
            label,
            Change::Batch(model.activation(topic, now)),
        ));
    }
    Ok(Action::new(label, Change::put(topic)))
}

/// Puts a topic in its slot, parking the one there; a done topic comes back as rework.
pub fn activate(model: &mut Model, now: Time, id: u64) -> Result<Action, String> {
    let slot = model.topic(id)?.slot;
    put_in(model, now, id, slot)
}

/// Makes a topic the active one of `slot`, moving it there first when it lives in
/// another: one action, so one undo takes back both the move and the focus. The topic
/// active in `slot` is parked; a done topic comes back as rework.
pub fn put_in(model: &mut Model, now: Time, id: u64, slot: Slot) -> Result<Action, String> {
    let mut topic = model.topic(id)?.clone();
    let label = if topic.slot == slot {
        format!("Focus on “{}”", topic.title)
    } else {
        format!("Focus on “{}” in {}", topic.title, slot.label())
    };
    topic.slot = slot;
    if topic.stage == Stage::Done {
        topic.reworks.push(Rework {
            id: model.take_id(),
            reason: "Reopened".into(),
            at: now,
        });
    }
    Ok(Action::new(
        label,
        Change::Batch(model.activation(topic, now)),
    ))
}

/// Sends the active topic of its slot back to the backlog. A done topic stays done, its
/// cycle time ended: reopening it is `activate` or `rework`.
pub fn park(model: &mut Model, id: u64) -> Result<Action, String> {
    edit(model, id, |topic, _| {
        if topic.stage != Stage::Active {
            return Err("Only a topic in its slot can be parked".into());
        }
        topic.stage = Stage::Queued;
        Ok(format!("Park “{}”", topic.title))
    })
}

/// Marks a topic done, ending the block it is in: its cycle time stops now.
pub fn finish(model: &mut Model, now: Time, id: u64) -> Result<Action, String> {
    edit(model, id, |topic, _| {
        topic.stage = Stage::Done;
        topic.started_at.get_or_insert(now);
        topic.finished_at = Some(now);
        if let Some(block) = topic.open_block() {
            block.until = Some(now);
        }
        Ok(format!("Finish “{}”", topic.title))
    })
}

/// Records work that came back; a done topic is reopened in its slot.
pub fn rework(model: &mut Model, now: Time, id: u64, reason: &str) -> Result<Action, String> {
    let reason = required(reason, "a reason")?;
    let mut topic = model.topic(id)?.clone();
    topic.reworks.push(Rework {
        id: model.take_id(),
        reason,
        at: now,
    });
    let label = format!("Rework on “{}”", topic.title);
    if topic.stage == Stage::Done {
        return Ok(Action::new(
            label,
            Change::Batch(model.activation(topic, now)),
        ));
    }
    Ok(Action::new(label, Change::put(topic)))
}

pub fn block(model: &mut Model, now: Time, id: u64, reason: &str) -> Result<Action, String> {
    let reason = required(reason, "a reason")?;
    edit(model, id, |topic, model| {
        if topic.is_blocked() {
            return Err("already blocked".into());
        }
        topic.blocks.push(Block {
            id: model.take_id(),
            reason,
            since: now,
            until: None,
        });
        Ok(format!("Block “{}”", topic.title))
    })
}

pub fn unblock(model: &mut Model, now: Time, id: u64) -> Result<Action, String> {
    edit(model, id, |topic, _| {
        let block = topic.open_block().ok_or("not blocked")?;
        block.until = Some(now);
        Ok(format!("Unblock “{}”", topic.title))
    })
}

pub fn add_step(model: &mut Model, id: u64, text: &str) -> Result<Action, String> {
    let text = required(text, "a step")?;
    edit(model, id, |topic, model| {
        topic.steps.push(Step {
            id: model.take_id(),
            text: text.clone(),
            done_at: None,
        });
        Ok(format!("Add step “{text}”"))
    })
}

/// Marks step `part` done, or not done any more.
pub fn toggle_step(model: &mut Model, now: Time, id: u64, part: u64) -> Result<Action, String> {
    edit(model, id, |topic, _| {
        let step = topic
            .steps
            .iter_mut()
            .find(|s| s.id == part)
            .ok_or("no such step")?;
        let label = if step.done_at.is_some() {
            format!("Reopen “{}”", step.text)
        } else {
            format!("Done “{}”", step.text)
        };
        step.done_at = match step.done_at {
            Some(_) => None,
            None => Some(now),
        };
        Ok(label)
    })
}

/// Moves step `part` up (`-1`) or down (`1`) among the steps.
pub fn move_step(model: &mut Model, id: u64, part: u64, delta: i32) -> Result<Action, String> {
    edit(model, id, |topic, _| {
        let from = topic
            .steps
            .iter()
            .position(|s| s.id == part)
            .ok_or("no such step")?;
        let to = from
            .checked_add_signed(delta as isize)
            .filter(|&i| i < topic.steps.len())
            .ok_or("no step there")?;
        topic.steps.swap(from, to);
        Ok(format!("Move “{}”", topic.steps[to].text))
    })
}

pub fn add_note(model: &mut Model, now: Time, id: u64, text: &str) -> Result<Action, String> {
    let text = required(text, "a note")?;
    edit(model, id, |topic, model| {
        topic.notes.push(Note {
            id: model.take_id(),
            text,
            at: now,
        });
        Ok(format!("Note on “{}”", topic.title))
    })
}

/// Links an issue, pull request, Slack thread or any page, from its URL or Linear key.
pub fn add_link(model: &mut Model, id: u64, text: &str) -> Result<Action, String> {
    let (url, kind) = links::parse(text).ok_or("not a URL nor a Linear key")?;
    edit(model, id, |topic, model| {
        if topic.links.iter().any(|l| l.url == url) {
            return Err("already linked".into());
        }
        topic.links.push(Link {
            id: model.take_id(),
            url: url.clone(),
            kind,
        });
        Ok(format!("Link {url}"))
    })
}

/// Removes a step, note, link, block or rework from topic `id`.
pub fn remove_part(model: &mut Model, id: u64, part: u64) -> Result<Action, String> {
    edit(model, id, |topic, _| {
        if !topic.remove_part(part) {
            return Err("nothing to remove".into());
        }
        Ok(format!("Remove from “{}”", topic.title))
    })
}

pub fn set_initiative(
    model: &mut Model,
    id: u64,
    initiative: Option<Initiative>,
) -> Result<Action, String> {
    edit(model, id, |topic, _| {
        let label = match &initiative {
            Some(i) => format!("Link “{}” to {}", topic.title, i.name),
            None => format!("Unlink “{}” from its initiative", topic.title),
        };
        topic.initiative = initiative;
        Ok(label)
    })
}

pub fn set_session(model: &mut Model, id: u64, session: &str) -> Result<Action, String> {
    let session = session.trim().to_owned();
    if !is_session_name(&session) {
        return Err("a session name is lowercase letters, digits, - and _, 32 at most".into());
    }
    edit(model, id, |topic, _| {
        topic.session = session;
        Ok(format!("Rename the session of “{}”", topic.title))
    })
}

/// Sets the folder topic `id`'s session starts in, which must exist.
pub fn set_folder(model: &mut Model, id: u64, folder: &str) -> Result<Action, String> {
    let folder = required(folder, "a folder")?;
    if !std::path::Path::new(&shell::expand(&folder)).is_dir() {
        return Err(format!("{folder} is not a folder"));
    }
    edit(model, id, |topic, _| {
        topic.folder = folder;
        Ok(format!("Change the folder of “{}”", topic.title))
    })
}

pub fn remove(model: &Model, id: u64) -> Result<Action, String> {
    let title = &model.topic(id)?.title;
    let change = Change::Put {
        id,
        index: 0,
        topic: None,
    };
    Ok(Action::new(format!("Delete “{title}”"), change))
}

/// Replaces the settings; `None` when they are the same, so nothing is there to undo.
pub fn set_settings(model: &Model, settings: Settings) -> Option<Action> {
    (model.settings != settings).then(|| Action::new("Change settings", Change::Settings(settings)))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::history::History;
    use crate::live::Issue;
    use crate::model::tests::topic;

    /// A model and its history, with topics `a` (Feature) and `b` (Tooling) in the
    /// backlog.
    fn setup() -> (Model, History) {
        let (mut model, history) = (Model::default(), History::default());
        for (title, slot) in [("a", Slot::Feature), ("b", Slot::Tooling)] {
            let t = topic(&mut model, title, slot);
            model.apply(Change::put(t)).unwrap();
        }
        (model, history)
    }

    /// Performs `action`, checks undoing it gives back the model as it was, then redoes
    /// it.
    fn perform(
        model: &mut Model,
        history: &mut History,
        action: impl FnOnce(&mut Model) -> Result<Action, String>,
    ) {
        let Action { label, change } = action(model).unwrap();
        let (topics, settings) = (model.topics.list.clone(), model.settings.clone());
        history.perform(model, label.clone(), change).unwrap();
        let after = model.topics.list.clone();
        assert_eq!(history.undo(model).unwrap(), Some(label));
        assert_eq!(model.topics.list, topics);
        assert_eq!(model.settings, settings);
        history.redo(model).unwrap();
        assert_eq!(model.topics.list, after);
    }

    fn get(model: &Model, id: u64) -> &Topic {
        model.topic(id).unwrap()
    }

    #[test]
    fn creating_a_topic_needs_a_title_and_may_activate_it() {
        let (mut model, mut history) = setup();
        assert_eq!(
            create_topic(&mut model, 5, "  ", Slot::Feature, false),
            Err("a title is needed".into())
        );
        let action = create_topic(&mut model, 5, " c ", Slot::Feature, true);
        perform(&mut model, &mut history, |_| action);
        let c = model.active(Slot::Feature).unwrap();
        assert_eq!((c.title.as_str(), c.created_at), ("c", 5));
        assert_eq!(c.started_at, Some(5));
    }

    #[test]
    fn a_topic_from_an_issue_links_it_and_copies_its_first_initiative() {
        let (mut model, mut history) = setup();
        let initiative = |name: &str| Initiative {
            id: name.into(),
            name: name.into(),
            url: format!("https://linear.app/{name}"),
        };
        let live = Live {
            sub_issues: vec![Issue {
                key: "BIM-7".into(),
                title: "Export".into(),
                url: "https://linear.app/i/BIM-7".into(),
                initiatives: vec![initiative("why"), initiative("other")],
                ..Issue::default()
            }],
            ..Live::default()
        };
        assert_eq!(
            topic_from_issue(&mut model, 9, &live, "BIM-8", Slot::Feature),
            Err("no issue BIM-8".into())
        );
        let action = topic_from_issue(&mut model, 9, &live, "BIM-7", Slot::Feature);
        perform(&mut model, &mut history, |_| action);
        let t = model.active(Slot::Feature).unwrap();
        assert_eq!(t.title, "Export");
        assert_eq!(t.links[0].url, "https://linear.app/i/BIM-7");
        assert_eq!(t.links[0].kind, links::Kind::LinearIssue);
        assert_eq!(t.initiative, Some(initiative("why")));
    }

    #[test]
    fn moving_an_active_topic_keeps_it_active_and_parks_the_one_there() {
        let (mut model, mut history) = setup();
        perform(&mut model, &mut history, |m| activate(m, 1, 0));
        perform(&mut model, &mut history, |m| activate(m, 1, 1));
        perform(&mut model, &mut history, |m| {
            move_topic(m, 2, 0, Slot::Tooling)
        });
        assert_eq!(get(&model, 0).stage, Stage::Active);
        assert_eq!(get(&model, 0).slot, Slot::Tooling);
        assert_eq!(get(&model, 1).stage, Stage::Queued);
    }

    #[test]
    fn moving_a_queued_topic_leaves_it_queued() {
        let (mut model, mut history) = setup();
        perform(&mut model, &mut history, |m| activate(m, 1, 1));
        perform(&mut model, &mut history, |m| {
            move_topic(m, 2, 0, Slot::Tooling)
        });
        assert_eq!(get(&model, 0).stage, Stage::Queued);
        assert_eq!(get(&model, 1).stage, Stage::Active);
    }

    #[test]
    fn putting_a_queued_topic_in_another_slot_is_one_undoable_action() {
        let (mut model, mut history) = setup();
        perform(&mut model, &mut history, |m| activate(m, 1, 1));
        perform(&mut model, &mut history, |m| put_in(m, 2, 0, Slot::Tooling));
        let a = get(&model, 0);
        assert_eq!(
            (a.slot, a.stage, a.started_at),
            (Slot::Tooling, Stage::Active, Some(2))
        );
        assert_eq!(get(&model, 1).stage, Stage::Queued);
        assert_eq!(history.next_undo(), Some("Focus on “a” in Tooling"));

        history.undo(&mut model).unwrap();
        let a = get(&model, 0);
        assert_eq!(
            (a.slot, a.stage, a.started_at),
            (Slot::Feature, Stage::Queued, None)
        );
        assert_eq!(get(&model, 1).stage, Stage::Active);
        assert_eq!(history.next_undo(), Some("Focus on “b”"));
    }

    #[test]
    fn putting_a_topic_in_its_own_slot_activates_it() {
        let (mut model, mut history) = setup();
        perform(&mut model, &mut history, |m| put_in(m, 1, 0, Slot::Feature));
        assert_eq!(history.next_undo(), Some("Focus on “a”"));
        assert_eq!(model.active(Slot::Feature).map(|t| t.id), Some(0));
    }

    #[test]
    fn finishing_ends_the_open_block() {
        let (mut model, mut history) = setup();
        perform(&mut model, &mut history, |m| activate(m, 1, 0));
        perform(&mut model, &mut history, |m| block(m, 2, 0, "review"));
        perform(&mut model, &mut history, |m| finish(m, 3, 0));
        let a = get(&model, 0);
        assert_eq!(a.stage, Stage::Done);
        assert_eq!((a.started_at, a.finished_at), (Some(1), Some(3)));
        assert_eq!(a.blocks[0].until, Some(3));
        assert!(!a.is_blocked());
    }

    #[test]
    fn finishing_a_topic_never_started_starts_it_then() {
        let (mut model, mut history) = setup();
        perform(&mut model, &mut history, |m| finish(m, 4, 0));
        assert_eq!(get(&model, 0).started_at, Some(4));
    }

    #[test]
    fn activating_a_done_topic_reopens_it_as_rework() {
        let (mut model, mut history) = setup();
        perform(&mut model, &mut history, |m| activate(m, 1, 0));
        perform(&mut model, &mut history, |m| finish(m, 2, 0));
        perform(&mut model, &mut history, |m| activate(m, 3, 0));
        let a = get(&model, 0);
        assert_eq!(a.stage, Stage::Active);
        assert_eq!((a.started_at, a.finished_at), (Some(1), None));
        assert_eq!(a.reworks.len(), 1);
        assert_eq!(
            (a.reworks[0].reason.as_str(), a.reworks[0].at),
            ("Reopened", 3)
        );
    }

    #[test]
    fn only_an_active_topic_parks() {
        let (mut model, mut history) = setup();
        let refused = Err("Only a topic in its slot can be parked".into());
        assert_eq!(park(&mut model, 0), refused);
        perform(&mut model, &mut history, |m| activate(m, 1, 0));
        perform(&mut model, &mut history, |m| park(m, 0));
        let a = get(&model, 0);
        assert_eq!((a.stage, a.started_at), (Stage::Queued, Some(1)));
        perform(&mut model, &mut history, |m| finish(m, 2, 0));
        assert_eq!(park(&mut model, 0), refused);
        assert_eq!(get(&model, 0).finished_at, Some(2));
    }

    #[test]
    fn rework_reopens_a_done_topic_and_only_records_on_an_open_one() {
        let (mut model, mut history) = setup();
        assert_eq!(
            rework(&mut model, 1, 0, " "),
            Err("a reason is needed".into())
        );
        perform(&mut model, &mut history, |m| rework(m, 1, 0, "bug"));
        assert_eq!(get(&model, 0).stage, Stage::Queued);
        perform(&mut model, &mut history, |m| finish(m, 2, 0));
        perform(&mut model, &mut history, |m| rework(m, 3, 0, "again"));
        let a = get(&model, 0);
        assert_eq!(a.stage, Stage::Active);
        assert_eq!(a.finished_at, None);
        assert_eq!(a.reworks.len(), 2);
    }

    #[test]
    fn a_topic_is_blocked_once_and_unblocked_once() {
        let (mut model, mut history) = setup();
        assert_eq!(unblock(&mut model, 1, 0), Err("not blocked".into()));
        perform(&mut model, &mut history, |m| block(m, 1, 0, "waiting"));
        assert_eq!(
            block(&mut model, 2, 0, "again"),
            Err("already blocked".into())
        );
        perform(&mut model, &mut history, |m| unblock(m, 5, 0));
        assert_eq!(get(&model, 0).blocks[0].until, Some(5));
    }

    #[test]
    fn steps_are_added_toggled_moved_and_removed() {
        let (mut model, mut history) = setup();
        assert_eq!(add_step(&mut model, 0, ""), Err("a step is needed".into()));
        perform(&mut model, &mut history, |m| add_step(m, 0, "one"));
        perform(&mut model, &mut history, |m| add_step(m, 0, "two"));
        let (one, two) = (get(&model, 0).steps[0].id, get(&model, 0).steps[1].id);
        perform(&mut model, &mut history, |m| toggle_step(m, 7, 0, one));
        assert_eq!(get(&model, 0).steps[0].done_at, Some(7));
        assert_eq!(
            move_step(&mut model, 0, one, -1),
            Err("no step there".into())
        );
        perform(&mut model, &mut history, |m| move_step(m, 0, one, 1));
        let ids: Vec<u64> = get(&model, 0).steps.iter().map(|s| s.id).collect();
        assert_eq!(ids, [two, one]);
        perform(&mut model, &mut history, |m| remove_part(m, 0, one));
        assert_eq!(get(&model, 0).steps.len(), 1);
        assert_eq!(
            remove_part(&mut model, 0, one),
            Err("nothing to remove".into())
        );
    }

    #[test]
    fn notes_and_links_are_checked() {
        let (mut model, mut history) = setup();
        assert_eq!(
            add_note(&mut model, 1, 0, " "),
            Err("a note is needed".into())
        );
        perform(&mut model, &mut history, |m| add_note(m, 1, 0, "hi"));
        assert_eq!(
            add_link(&mut model, 0, "not a link"),
            Err("not a URL nor a Linear key".into())
        );
        perform(&mut model, &mut history, |m| add_link(m, 0, "BIM-1"));
        assert_eq!(
            add_link(&mut model, 0, "BIM-1"),
            Err("already linked".into())
        );
    }

    #[test]
    fn renaming_and_sessions_and_folders_are_validated() {
        let (mut model, mut history) = setup();
        assert_eq!(rename(&mut model, 0, ""), Err("a title is needed".into()));
        perform(&mut model, &mut history, |m| rename(m, 0, "z"));
        assert_eq!(get(&model, 0).title, "z");
        assert!(set_session(&mut model, 0, "Not Valid").is_err());
        perform(&mut model, &mut history, |m| set_session(m, 0, " mq-z "));
        assert_eq!(get(&model, 0).session, "mq-z");
        assert_eq!(
            set_folder(&mut model, 0, ""),
            Err("a folder is needed".into())
        );
        assert_eq!(
            set_folder(&mut model, 0, "/no/such/folder"),
            Err("/no/such/folder is not a folder".into())
        );
        let here = env!("CARGO_MANIFEST_DIR");
        perform(&mut model, &mut history, |m| set_folder(m, 0, here));
        assert_eq!(get(&model, 0).folder, here);
    }

    #[test]
    fn a_topic_is_linked_to_an_initiative_then_deleted() {
        let (mut model, mut history) = setup();
        let why = Initiative {
            id: "1".into(),
            name: "why".into(),
            url: String::new(),
        };
        let action = set_initiative(&mut model, 0, Some(why.clone()));
        perform(&mut model, &mut history, |_| action);
        assert_eq!(get(&model, 0).initiative, Some(why));
        perform(&mut model, &mut history, |m| remove(m, 0));
        assert_eq!(model.topic(0), Err("no topic 0".into()));
        assert_eq!(remove(&model, 0), Err("no topic 0".into()));
    }

    #[test]
    fn the_same_settings_are_no_action() {
        let (mut model, mut history) = setup();
        assert_eq!(set_settings(&model, Settings::default()), None);
        let settings = Settings {
            search_limit: 7,
            ..Settings::default()
        };
        let action = set_settings(&model, settings.clone()).ok_or(String::new());
        perform(&mut model, &mut history, |_| action);
        assert_eq!(model.settings, settings);
    }
}
