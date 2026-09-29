//! The app's data, and the changes made to it. Applying a change gives back the
//! change that undoes it, which is what makes every action undoable (see `history`).

use serde::{Deserialize, Serialize};

use crate::settings::Settings;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: u64,
    pub title: String,
}

/// The items, as saved in `items.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Items {
    pub list: Vec<Item>,
    pub next_id: u64,
}

#[derive(Debug, Default)]
pub struct Model {
    pub items: Items,
    pub settings: Settings,
}

/// One change to the model. Every mutation of the app is one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Insert { index: usize, item: Item },
    Remove { id: u64 },
    Rename { id: u64, title: String },
    Settings(Settings),
}

impl Model {
    /// A new item, not yet inserted.
    pub fn new_item(&self, title: String) -> Item {
        Item {
            id: self.items.next_id,
            title,
        }
    }

    /// Applies `change` and returns its inverse.
    pub fn apply(&mut self, change: Change) -> Result<Change, String> {
        match change {
            Change::Insert { index, item } => {
                let id = item.id;
                let index = index.min(self.items.list.len());
                self.items.next_id = self.items.next_id.max(id + 1);
                self.items.list.insert(index, item);
                Ok(Change::Remove { id })
            }
            Change::Remove { id } => {
                let index = self.position(id)?;
                let item = self.items.list.remove(index);
                Ok(Change::Insert { index, item })
            }
            Change::Rename { id, title } => {
                let index = self.position(id)?;
                let before = std::mem::replace(&mut self.items.list[index].title, title);
                Ok(Change::Rename { id, title: before })
            }
            Change::Settings(settings) => Ok(Change::Settings(std::mem::replace(
                &mut self.settings,
                settings,
            ))),
        }
    }

    pub fn item(&self, id: u64) -> Result<&Item, String> {
        self.position(id).map(|i| &self.items.list[i])
    }

    fn position(&self, id: u64) -> Result<usize, String> {
        self.items
            .list
            .iter()
            .position(|item| item.id == id)
            .ok_or_else(|| format!("no item {id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(model: &Model) -> Vec<&str> {
        model.items.list.iter().map(|i| i.title.as_str()).collect()
    }

    #[test]
    fn every_change_is_undone_by_its_inverse() {
        let mut model = Model::default();
        for title in ["a", "b", "c"] {
            let item = model.new_item(title.into());
            let index = model.items.list.len();
            model.apply(Change::Insert { index, item }).unwrap();
        }
        let changes = [
            Change::Remove { id: 1 },
            Change::Rename {
                id: 2,
                title: "z".into(),
            },
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
    fn ids_are_never_reused() {
        let mut model = Model::default();
        let item = model.new_item("a".into());
        model.apply(Change::Insert { index: 0, item }).unwrap();
        model.apply(Change::Remove { id: 0 }).unwrap();
        assert_eq!(model.new_item("b".into()).id, 1);
    }

    #[test]
    fn a_missing_item_is_an_error() {
        let mut model = Model::default();
        assert!(model.apply(Change::Remove { id: 9 }).is_err());
    }
}
