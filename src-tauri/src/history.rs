//! The undo and redo stacks. Every action goes through `History::perform`, which keeps
//! the inverse of its change: nothing the user does needs a confirmation, since
//! everything can be taken back.

use crate::model::{Change, Model};

/// How many actions can be undone.
const DEPTH: usize = 200;

#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
}

/// A change that takes the model one step back (or forward), with the label of the
/// action it belongs to, such as `Delete “groceries”`.
#[derive(Debug)]
struct Entry {
    label: String,
    change: Change,
}

impl History {
    /// Applies `change` as the action `label`, and forgets what could be redone.
    pub fn perform(
        &mut self,
        model: &mut Model,
        label: impl Into<String>,
        change: Change,
    ) -> Result<(), String> {
        let inverse = model.apply(change)?;
        self.undo.push(Entry {
            label: label.into(),
            change: inverse,
        });
        if self.undo.len() > DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
        Ok(())
    }

    /// Undoes the last action; returns its label, or `None` when there is none.
    pub fn undo(&mut self, model: &mut Model) -> Result<Option<String>, String> {
        step(&mut self.undo, &mut self.redo, model)
    }

    /// Redoes the last undone action; returns its label, or `None` when there is none.
    pub fn redo(&mut self, model: &mut Model) -> Result<Option<String>, String> {
        step(&mut self.redo, &mut self.undo, model)
    }

    /// The label of the action `undo` would take back.
    pub fn next_undo(&self) -> Option<&str> {
        self.undo.last().map(|e| e.label.as_str())
    }

    /// The label of the action `redo` would do again.
    pub fn next_redo(&self) -> Option<&str> {
        self.redo.last().map(|e| e.label.as_str())
    }
}

fn step(
    from: &mut Vec<Entry>,
    to: &mut Vec<Entry>,
    model: &mut Model,
) -> Result<Option<String>, String> {
    let Some(entry) = from.pop() else {
        return Ok(None);
    };
    let inverse = model.apply(entry.change)?;
    to.push(Entry {
        label: entry.label.clone(),
        change: inverse,
    });
    Ok(Some(entry.label))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::model::Slot;
    use crate::model::tests::topic;

    fn titles(model: &Model) -> Vec<&str> {
        model.topics.list.iter().map(|t| t.title.as_str()).collect()
    }

    fn add(history: &mut History, model: &mut Model, title: &str) {
        let topic = topic(model, title, Slot::Feature);
        history
            .perform(model, format!("Add “{title}”"), Change::put(topic))
            .unwrap();
    }

    #[test]
    fn undoes_and_redoes_in_order() {
        let (mut history, mut model) = (History::default(), Model::default());
        add(&mut history, &mut model, "a");
        add(&mut history, &mut model, "b");
        history
            .perform(
                &mut model,
                "Delete “a”",
                Change::Put {
                    id: 0,
                    index: 0,
                    topic: None,
                },
            )
            .unwrap();
        assert_eq!(titles(&model), ["b"]);

        assert_eq!(
            history.undo(&mut model).unwrap().as_deref(),
            Some("Delete “a”")
        );
        assert_eq!(titles(&model), ["a", "b"]);
        assert_eq!(
            history.undo(&mut model).unwrap().as_deref(),
            Some("Add “b”")
        );
        assert_eq!(titles(&model), ["a"]);
        assert_eq!(history.next_redo(), Some("Add “b”"));

        history.redo(&mut model).unwrap();
        history.redo(&mut model).unwrap();
        assert_eq!(titles(&model), ["b"]);
        assert_eq!(history.redo(&mut model).unwrap(), None);
    }

    #[test]
    fn a_new_action_forgets_the_redo_stack() {
        let (mut history, mut model) = (History::default(), Model::default());
        add(&mut history, &mut model, "a");
        history.undo(&mut model).unwrap();
        add(&mut history, &mut model, "b");
        assert_eq!(history.next_redo(), None);
        assert_eq!(history.next_undo(), Some("Add “b”"));
    }

    #[test]
    fn keeps_a_bounded_depth() {
        let (mut history, mut model) = (History::default(), Model::default());
        for n in 0..DEPTH + 5 {
            add(&mut history, &mut model, &n.to_string());
        }
        let mut undone = 0;
        while history.undo(&mut model).unwrap().is_some() {
            undone += 1;
        }
        assert_eq!(undone, DEPTH);
        assert_eq!(model.topics.list.len(), 5);
    }
}
