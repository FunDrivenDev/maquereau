//! The topics, and the changes made to them. Applying a change gives back the change
//! that undoes it, which is what makes every action undoable (see `history`).
//!
//! A topic is what Raphaël works on in one of the four slots: it gathers the Linear
//! issues, pull requests and Slack threads it touches, the steps still to take, short
//! notes, and the flow events (start, blocks, rework, finish) its times are read from.

use serde::{Deserialize, Serialize};

use crate::settings::Settings;

/// Unix time, in seconds.
pub type Time = i64;

/// The four kinds of work, each with one active topic at most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    Feature,
    BugRun,
    Exploration,
    Tooling,
}

impl Slot {
    pub const ALL: [Slot; 4] = [
        Slot::Feature,
        Slot::BugRun,
        Slot::Exploration,
        Slot::Tooling,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Slot::Feature => "Feature",
            Slot::BugRun => "Bug / Run",
            Slot::Exploration => "Exploration",
            Slot::Tooling => "Tooling",
        }
    }
}

/// Where a topic is in its life. Only an `Active` topic occupies its slot; `Queued` ones
/// wait in the backlog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Queued,
    Active,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Topic {
    pub id: u64,
    pub title: String,
    pub slot: Slot,
    pub stage: Stage,
    /// When the topic was written down: the start of its lead time.
    pub created_at: Time,
    /// When it first became active: the start of its cycle time.
    pub started_at: Option<Time>,
    pub finished_at: Option<Time>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub notes: Vec<Note>,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default)]
    pub blocks: Vec<Block>,
    #[serde(default)]
    pub reworks: Vec<Rework>,
    #[serde(default)]
    pub initiative: Option<Initiative>,
    /// The name of the herdr session that holds the topic's terminals and agents.
    pub session: String,
    /// The folder the session starts in; `~` is the home folder.
    pub folder: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub id: u64,
    pub text: String,
    pub done_at: Option<Time>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub text: String,
    pub at: Time,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub id: u64,
    /// The URL, or a bare Linear key such as `BIM-123`.
    pub url: String,
    pub kind: crate::links::Kind,
}

/// A time the topic could not move, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub id: u64,
    pub reason: String,
    pub since: Time,
    pub until: Option<Time>,
}

/// Work that came back: a review asking for changes, a bug in what was shipped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rework {
    pub id: u64,
    pub reason: String,
    pub at: Time,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Initiative {
    pub id: String,
    pub name: String,
    pub url: String,
}

/// The topics, as saved in `topics.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Topics {
    pub list: Vec<Topic>,
    /// The next id, shared by the topics and every part of them.
    pub next_id: u64,
}

#[derive(Debug, Default)]
pub struct Model {
    pub topics: Topics,
    pub settings: Settings,
}

/// One change to the model. Every mutation of the app is one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Sets topic `id` to `topic`: replaces it in place, inserts it at `index` when it is
    /// new, or removes it when `topic` is `None`.
    Put {
        id: u64,
        index: usize,
        topic: Option<Box<Topic>>,
    },
    /// Several changes as one action.
    Batch(Vec<Change>),
    Settings(Settings),
}

impl Change {
    pub fn put(topic: Topic) -> Self {
        Change::Put {
            id: topic.id,
            index: usize::MAX,
            topic: Some(Box::new(topic)),
        }
    }
}

impl Model {
    /// A fresh id, for a topic or a part of one. The id is taken even when the change
    /// that uses it is undone, so an id is never reused.
    pub fn take_id(&mut self) -> u64 {
        let id = self.topics.next_id;
        self.topics.next_id += 1;
        id
    }

    /// A new topic of `slot`, waiting in the backlog, with a session named after its title
    /// in the folder of the settings. It takes a fresh id but is not added yet.
    pub fn new_topic(&mut self, title: String, slot: Slot, now: Time) -> Topic {
        Topic {
            id: self.take_id(),
            session: session_name(&title),
            folder: self.settings.folder.clone(),
            title,
            slot,
            stage: Stage::Queued,
            created_at: now,
            started_at: None,
            finished_at: None,
            steps: vec![],
            notes: vec![],
            links: vec![],
            blocks: vec![],
            reworks: vec![],
            initiative: None,
        }
    }

    /// Applies `change` and returns its inverse.
    pub fn apply(&mut self, change: Change) -> Result<Change, String> {
        match change {
            Change::Put { id, index, topic } => {
                let list = &mut self.topics.list;
                let at = list.iter().position(|t| t.id == id);
                let before = match (at, topic) {
                    (Some(i), Some(topic)) => {
                        let old = std::mem::replace(&mut list[i], *topic);
                        (i, Some(Box::new(old)))
                    }
                    (Some(i), None) => (i, Some(Box::new(list.remove(i)))),
                    (None, Some(topic)) => {
                        let i = index.min(list.len());
                        self.topics.next_id = self.topics.next_id.max(id + 1);
                        list.insert(i, *topic);
                        (i, None)
                    }
                    (None, None) => return Err(format!("no topic {id}")),
                };
                Ok(Change::Put {
                    id,
                    index: before.0,
                    topic: before.1,
                })
            }
            Change::Batch(changes) => {
                let mut inverses = Vec::with_capacity(changes.len());
                for change in changes {
                    match self.apply(change) {
                        Ok(inverse) => inverses.push(inverse),
                        Err(e) => {
                            for inverse in inverses.into_iter().rev() {
                                self.apply(inverse)?;
                            }
                            return Err(e);
                        }
                    }
                }
                inverses.reverse();
                Ok(Change::Batch(inverses))
            }
            Change::Settings(settings) => Ok(Change::Settings(std::mem::replace(
                &mut self.settings,
                settings,
            ))),
        }
    }

    pub fn topic(&self, id: u64) -> Result<&Topic, String> {
        self.topics
            .list
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("no topic {id}"))
    }

    /// The active topic of `slot`, if any.
    pub fn active(&self, slot: Slot) -> Option<&Topic> {
        self.topics
            .list
            .iter()
            .find(|t| t.slot == slot && t.stage == Stage::Active)
    }

    /// The changes that make topic `topic` the active one of its slot, sending the topic
    /// there before back to the backlog.
    pub fn activation(&self, mut topic: Topic, now: Time) -> Vec<Change> {
        let mut changes = Vec::new();
        if let Some(current) = self.active(topic.slot)
            && current.id != topic.id
        {
            let mut parked = current.clone();
            parked.stage = Stage::Queued;
            changes.push(Change::put(parked));
        }
        topic.stage = Stage::Active;
        topic.started_at.get_or_insert(now);
        topic.finished_at = None;
        changes.push(Change::put(topic));
        changes
    }
}

impl Topic {
    /// The block still going on, if any.
    pub fn open_block(&mut self) -> Option<&mut Block> {
        self.blocks.iter_mut().find(|b| b.until.is_none())
    }

    pub fn is_blocked(&self) -> bool {
        self.blocks.iter().any(|b| b.until.is_none())
    }

    /// Removes the step, note, link, block or rework `part`; `false` when there is none.
    pub fn remove_part(&mut self, part: u64) -> bool {
        let before = self.steps.len()
            + self.notes.len()
            + self.links.len()
            + self.blocks.len()
            + self.reworks.len();
        self.steps.retain(|p| p.id != part);
        self.notes.retain(|p| p.id != part);
        self.links.retain(|p| p.id != part);
        self.blocks.retain(|p| p.id != part);
        self.reworks.retain(|p| p.id != part);
        let after = self.steps.len()
            + self.notes.len()
            + self.links.len()
            + self.blocks.len()
            + self.reworks.len();
        after < before
    }
}

/// A herdr session name for `title`: `mq-` then its words, lowercased and dashed, within
/// herdr's 32 characters.
pub fn session_name(title: &str) -> String {
    let mut name = String::from("mq-");
    let mut dash = false;
    for c in title.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            if dash && !name.ends_with('-') {
                name.push('-');
            }
            name.push(c);
            dash = false;
        } else {
            dash = true;
        }
        if name.len() >= 32 {
            break;
        }
    }
    name.truncate(32);
    name.trim_end_matches('-').to_owned()
}

/// Whether `name` is a session name herdr accepts and a shell needs no quoting for.
pub fn is_session_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && name.len() <= 32
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn topic(model: &mut Model, title: &str, slot: Slot) -> Topic {
        model.new_topic(title.into(), slot, 0)
    }

    fn titles(model: &Model) -> Vec<&str> {
        model.topics.list.iter().map(|t| t.title.as_str()).collect()
    }

    #[test]
    fn every_change_is_undone_by_its_inverse() {
        let mut model = Model::default();
        for title in ["a", "b", "c"] {
            let t = topic(&mut model, title, Slot::Feature);
            model.apply(Change::put(t)).unwrap();
        }
        let mut renamed = model.topic(1).unwrap().clone();
        renamed.title = "z".into();
        let changes = [
            Change::Put {
                id: 1,
                index: 0,
                topic: None,
            },
            Change::put(renamed.clone()),
            Change::Batch(vec![
                Change::put(renamed),
                Change::Put {
                    id: 0,
                    index: 0,
                    topic: None,
                },
            ]),
            Change::Settings(Settings {
                search_limit: 7,
                ..Settings::default()
            }),
        ];
        for change in changes {
            let inverse = model.apply(change.clone()).unwrap();
            assert_ne!(inverse, change);
            model.apply(inverse).unwrap();
            assert_eq!(titles(&model), ["a", "b", "c"]);
            assert_eq!(model.settings, Settings::default());
        }
    }

    #[test]
    fn a_failed_batch_leaves_the_model_as_it_was() {
        let mut model = Model::default();
        let t = topic(&mut model, "a", Slot::Feature);
        model.apply(Change::put(t.clone())).unwrap();
        let mut renamed = t;
        renamed.title = "z".into();
        let missing = Change::Put {
            id: 9,
            index: 0,
            topic: None,
        };
        assert!(
            model
                .apply(Change::Batch(vec![Change::put(renamed), missing]))
                .is_err()
        );
        assert_eq!(titles(&model), ["a"]);
    }

    #[test]
    fn activating_a_topic_parks_the_one_in_its_slot() {
        let mut model = Model::default();
        let a = topic(&mut model, "a", Slot::Tooling);
        let b = topic(&mut model, "b", Slot::Tooling);
        let other = topic(&mut model, "c", Slot::Feature);
        for t in [a.clone(), b.clone(), other.clone()] {
            model.apply(Change::put(t)).unwrap();
        }
        for t in [a, other] {
            let changes = model.activation(t, 10);
            model.apply(Change::Batch(changes)).unwrap();
        }
        let changes = model.activation(b, 20);
        model.apply(Change::Batch(changes)).unwrap();
        let stages: Vec<Stage> = model.topics.list.iter().map(|t| t.stage).collect();
        assert_eq!(stages, [Stage::Queued, Stage::Active, Stage::Active]);
        assert_eq!(model.topic(0).unwrap().started_at, Some(10));
        assert_eq!(model.topic(1).unwrap().started_at, Some(20));
    }

    #[test]
    fn ids_are_never_reused() {
        let mut model = Model::default();
        let t = topic(&mut model, "a", Slot::Feature);
        let inverse = model.apply(Change::put(t)).unwrap();
        model.apply(inverse).unwrap();
        assert_eq!(model.take_id(), 1);
    }

    #[test]
    fn session_names_are_short_dashed_and_valid() {
        assert_eq!(session_name("Export PDF — v2!"), "mq-export-pdf-v2");
        assert_eq!(session_name("   "), "mq");
        let long = session_name("a very long title that never seems to end at all");
        assert!(long.len() <= 32 && is_session_name(&long), "{long}");
        assert!(!is_session_name("Mq"));
        assert!(!is_session_name("mq x"));
        assert!(!is_session_name(""));
    }
}
