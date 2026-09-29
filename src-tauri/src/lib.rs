//! maquereau: Keeps focus on four priority topics, one per slot, with their issues, pull requests, notes, flow times and Claude sessions.
//!
//! The front end holds no state of its own: each command returns a `Snapshot`, and each
//! one that changes something goes through `History::perform`, so it can be undone.

mod history;
mod model;
mod search;
mod settings;
mod store;

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, State, Wry};

use history::History;
use model::{Change, Item, Model};
use settings::Settings;

const ITEMS: &str = "items.json";
const SETTINGS: &str = "settings.json";

struct App {
    dir: PathBuf,
    inner: Mutex<Inner>,
}

struct Inner {
    model: Model,
    history: History,
}

impl App {
    /// Runs `action` on the model, saves it, and returns the new snapshot.
    fn update(
        &self,
        action: impl FnOnce(&mut Inner) -> Result<(), String>,
    ) -> Result<Snapshot, String> {
        let mut inner = self.inner.lock().unwrap();
        action(&mut inner)?;
        store::save(&self.dir.join(ITEMS), &inner.model.items)?;
        store::save(&self.dir.join(SETTINGS), &inner.model.settings)?;
        Ok(Snapshot::of(&inner))
    }
}

/// Everything the front end shows.
#[derive(Serialize)]
struct Snapshot {
    items: Vec<Item>,
    settings: Settings,
    /// The label of the action ⌘Z would undo.
    undo: Option<String>,
    /// The label of the action ⌘⇧Z would redo.
    redo: Option<String>,
}

impl Snapshot {
    fn of(inner: &Inner) -> Self {
        Self {
            items: inner.model.items.list.clone(),
            settings: inner.model.settings.clone(),
            undo: inner.history.next_undo().map(str::to_owned),
            redo: inner.history.next_redo().map(str::to_owned),
        }
    }
}

#[tauri::command]
fn snapshot(app: State<App>) -> Snapshot {
    Snapshot::of(&app.inner.lock().unwrap())
}

#[tauri::command]
fn add(app: State<App>, title: &str, index: usize) -> Result<Snapshot, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("a title is needed".into());
    }
    app.update(|Inner { model, history }| {
        let item = model.new_item(title.into());
        history.perform(
            model,
            format!("Add “{title}”"),
            Change::Insert { index, item },
        )
    })
}

#[tauri::command]
fn rename(app: State<App>, id: u64, title: &str) -> Result<Snapshot, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("a title is needed".into());
    }
    app.update(|Inner { model, history }| {
        let before = model.item(id)?.title.clone();
        let change = Change::Rename {
            id,
            title: title.into(),
        };
        history.perform(model, format!("Rename “{before}”"), change)
    })
}

#[tauri::command]
fn remove(app: State<App>, id: u64) -> Result<Snapshot, String> {
    app.update(|Inner { model, history }| {
        let title = model.item(id)?.title.clone();
        history.perform(model, format!("Delete “{title}”"), Change::Remove { id })
    })
}

#[tauri::command]
fn set_settings(app: State<App>, settings: Settings) -> Result<Snapshot, String> {
    app.update(|Inner { model, history }| {
        if model.settings == settings {
            return Ok(());
        }
        history.perform(model, "Change settings", Change::Settings(settings))
    })
}

#[tauri::command]
fn undo(app: State<App>) -> Result<Snapshot, String> {
    app.update(|Inner { model, history }| history.undo(model).map(drop))
}

#[tauri::command]
fn redo(app: State<App>) -> Result<Snapshot, String> {
    app.update(|Inner { model, history }| history.redo(model).map(drop))
}

/// The items whose title matches `query`, best first.
#[tauri::command]
fn search_items(app: State<App>, query: &str) -> Vec<search::Match> {
    let inner = app.inner.lock().unwrap();
    let limit = inner.model.settings.search_limit as usize;
    let titles = inner.model.items.list.iter().map(|i| i.title.as_str());
    search::fuzzy(query, titles, limit)
}

/// Fuzzy search over any list the front end holds, such as the commands or the settings.
#[tauri::command]
fn fuzzy(query: &str, candidates: Vec<String>) -> Vec<search::Match> {
    search::fuzzy(query, candidates.iter().map(String::as_str), usize::MAX)
}

/// The macOS menu: Settings… (⌘,) in the app menu, and the app's own Undo and Redo in
/// the Edit menu. A chosen item reaches the front end as a `menu` event with its id.
fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let settings = MenuItemBuilder::with_id("settings", "Settings…")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
    let undo = MenuItemBuilder::with_id("undo", "Undo")
        .accelerator("CmdOrCtrl+Z")
        .build(app)?;
    let redo = MenuItemBuilder::with_id("redo", "Redo")
        .accelerator("CmdOrCtrl+Shift+Z")
        .build(app)?;
    let app_menu = SubmenuBuilder::new(app, &app.package_info().name)
        .about(None)
        .separator()
        .item(&settings)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .item(&undo)
        .item(&redo)
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .close_window()
        .build()?;
    MenuBuilder::new(app)
        .items(&[&app_menu, &edit, &window])
        .build()
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let model = Model {
                items: store::load(&dir.join(ITEMS)),
                settings: store::load(&dir.join(SETTINGS)),
            };
            app.manage(App {
                dir,
                inner: Mutex::new(Inner {
                    model,
                    history: History::default(),
                }),
            });
            app.set_menu(menu(app.handle())?)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            let _ = app.emit("menu", event.id().as_ref());
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            add,
            rename,
            remove,
            set_settings,
            undo,
            redo,
            search_items,
            fuzzy
        ])
        .run(tauri::generate_context!())
        .expect("error while running maquereau");
}
