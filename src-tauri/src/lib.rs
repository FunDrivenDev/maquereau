//! maquereau keeps Raphaël on four priority topics, one per slot (Feature, Bug / Run,
//! Exploration, Tooling), with their issues, pull requests, notes, next step, flow times
//! and herdr session.
//!
//! The front end holds no state of its own: each command returns a `Snapshot`, and each
//! one that changes something calls its action in `actions`, which keeps the rules, then
//! performs it through `History::perform`, so it can be undone. A
//! background thread reads Linear, GitHub and herdr, and sends a new `Snapshot` as a
//! `snapshot` event when it has.

mod actions;
mod backlog;
mod flow;
mod github;
mod herdr;
mod history;
mod linear;
mod links;
mod live;
mod model;
mod search;
mod settings;
mod shell;
mod store;

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, State, Wry};

use actions::Action;
use history::History;
use live::Live;
use model::{Initiative, Model, Slot, Stage, Time, Topic};
use settings::Settings;
use shell::Shell;

const TOPICS: &str = "topics.json";
const SETTINGS: &str = "settings.json";
const LIVE: &str = "live.json";

struct App {
    dir: PathBuf,
    shell: Shell,
    inner: Mutex<Inner>,
    /// Set to read Linear and GitHub at the next tick rather than at the next period.
    refresh_now: AtomicBool,
}

struct Inner {
    model: Model,
    history: History,
    live: Live,
    has_linear_key: bool,
}

fn now() -> Time {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as Time)
}

impl App {
    /// The one way the model changes: `step` performs, undoes or redoes an action
    /// through the history, and says whether it changed anything. The model is saved
    /// when it did, and the new snapshot returned.
    fn update(
        &self,
        step: impl FnOnce(&mut History, &mut Model, &Live) -> Result<bool, String>,
    ) -> Result<Snapshot, String> {
        let mut inner = self.inner.lock().unwrap();
        let Inner {
            model,
            history,
            live,
            ..
        } = &mut *inner;
        if step(history, model, live)? {
            store::save(&self.dir.join(TOPICS), &model.topics)?;
            store::save(&self.dir.join(SETTINGS), &model.settings)?;
        }
        Ok(Snapshot::of(&inner))
    }

    /// Performs the action `act` builds from the model and what was last read from
    /// outside; an action of `None` changes nothing.
    fn perform<A: Into<Option<Action>>>(
        &self,
        act: impl FnOnce(&mut Model, &Live) -> Result<A, String>,
    ) -> Result<Snapshot, String> {
        self.update(|history, model, live| {
            let Some(Action { label, change }) = act(model, live)?.into() else {
                return Ok(false);
            };
            history.perform(model, label, change)?;
            Ok(true)
        })
    }
}

/// Everything the front end shows.
#[derive(Clone, Serialize)]
struct Snapshot {
    topics: Vec<Topic>,
    /// The flow times of each topic, by id.
    times: Vec<(u64, flow::Times)>,
    stats: Vec<flow::SlotStats>,
    live: Live,
    /// The assigned Linear issues no open topic links to, and their sub-issues, as a tree.
    backlog: Vec<backlog::Entry>,
    /// What the backlog holds to pull: see `backlog::count`.
    backlog_count: usize,
    settings: Settings,
    has_linear_key: bool,
    now: Time,
    /// The label of the action ⌘Z would undo.
    undo: Option<String>,
    /// The label of the action ⌘⇧Z would redo.
    redo: Option<String>,
}

impl Snapshot {
    fn of(inner: &Inner) -> Self {
        let now = now();
        let topics = &inner.model.topics.list;
        let linked: Vec<String> = topics
            .iter()
            .filter(|t| t.stage != Stage::Done)
            .flat_map(|t| &t.links)
            .filter_map(|l| links::key(&l.url))
            .collect();
        Self {
            topics: topics.clone(),
            times: topics.iter().map(|t| (t.id, flow::times(t, now))).collect(),
            stats: flow::stats(topics, now, inner.model.settings.stats_days),
            live: inner.live.clone(),
            backlog: backlog::tree(&inner.live.assigned, &inner.live.sub_issues, &linked),
            backlog_count: backlog::count(
                topics,
                &inner.live.assigned,
                &inner.live.sub_issues,
                &linked,
            ),
            settings: inner.model.settings.clone(),
            has_linear_key: inner.has_linear_key,
            now,
            undo: inner.history.next_undo().map(str::to_owned),
            redo: inner.history.next_redo().map(str::to_owned),
        }
    }
}

#[tauri::command]
fn snapshot(app: State<App>) -> Snapshot {
    Snapshot::of(&app.inner.lock().unwrap())
}

/// Adds a topic to `slot`; `activate` makes it the slot's topic, else it waits in the
/// backlog.
#[tauri::command]
fn create_topic(
    app: State<App>,
    title: &str,
    slot: Slot,
    activate: bool,
) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::create_topic(model, now(), title, slot, activate))
}

/// Makes a topic of the backlog's Linear issue `key` and puts it in `slot`.
#[tauri::command]
fn topic_from_issue(app: State<App>, key: &str, slot: Slot) -> Result<Snapshot, String> {
    app.perform(|model, live| actions::topic_from_issue(model, now(), live, key, slot))
}

#[tauri::command]
fn rename(app: State<App>, id: u64, title: &str) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::rename(model, id, title))
}

#[tauri::command]
fn move_topic(app: State<App>, id: u64, slot: Slot) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::move_topic(model, now(), id, slot))
}

#[tauri::command]
fn activate(app: State<App>, id: u64) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::activate(model, now(), id))
}

#[tauri::command]
fn park(app: State<App>, id: u64) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::park(model, id))
}

#[tauri::command]
fn finish(app: State<App>, id: u64) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::finish(model, now(), id))
}

#[tauri::command]
fn rework(app: State<App>, id: u64, reason: &str) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::rework(model, now(), id, reason))
}

#[tauri::command]
fn block(app: State<App>, id: u64, reason: &str) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::block(model, now(), id, reason))
}

#[tauri::command]
fn unblock(app: State<App>, id: u64) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::unblock(model, now(), id))
}

#[tauri::command]
fn add_step(app: State<App>, id: u64, text: &str) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::add_step(model, id, text))
}

#[tauri::command]
fn toggle_step(app: State<App>, id: u64, part: u64) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::toggle_step(model, now(), id, part))
}

#[tauri::command]
fn move_step(app: State<App>, id: u64, part: u64, delta: i32) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::move_step(model, id, part, delta))
}

#[tauri::command]
fn add_note(app: State<App>, id: u64, text: &str) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::add_note(model, now(), id, text))
}

#[tauri::command]
fn add_link(app: State<App>, id: u64, text: &str) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::add_link(model, id, text))
}

#[tauri::command]
fn remove_part(app: State<App>, id: u64, part: u64) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::remove_part(model, id, part))
}

#[tauri::command]
fn set_initiative(
    app: State<App>,
    id: u64,
    initiative: Option<Initiative>,
) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::set_initiative(model, id, initiative))
}

#[tauri::command]
fn set_session(app: State<App>, id: u64, session: &str) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::set_session(model, id, session))
}

#[tauri::command]
fn set_folder(app: State<App>, id: u64, folder: &str) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::set_folder(model, id, folder))
}

#[tauri::command]
fn remove(app: State<App>, id: u64) -> Result<Snapshot, String> {
    app.perform(|model, _| actions::remove(model, id))
}

#[tauri::command]
fn set_settings(app: State<App>, settings: Settings) -> Result<Snapshot, String> {
    app.perform(|model, _| Ok(actions::set_settings(model, settings)))
}

#[tauri::command]
fn undo(app: State<App>) -> Result<Snapshot, String> {
    app.update(|history, model, _| Ok(history.undo(model)?.is_some()))
}

#[tauri::command]
fn redo(app: State<App>) -> Result<Snapshot, String> {
    app.update(|history, model, _| Ok(history.redo(model)?.is_some()))
}

/// The topics whose title matches `query`, best first.
#[tauri::command]
fn search_topics(app: State<App>, query: &str) -> Vec<search::Match> {
    let inner = app.inner.lock().unwrap();
    let limit = inner.model.settings.search_limit as usize;
    let titles = inner.model.topics.list.iter().map(|t| t.title.as_str());
    search::fuzzy(query, titles, limit)
}

/// Fuzzy search over any list the front end holds, such as the commands or the settings.
#[tauri::command]
fn fuzzy(query: &str, candidates: Vec<String>) -> Vec<search::Match> {
    search::fuzzy(query, candidates.iter().map(String::as_str), usize::MAX)
}

/// Reads Linear and GitHub now rather than at the next period.
#[tauri::command]
fn refresh(app: State<App>) {
    app.refresh_now.store(true, Ordering::Relaxed);
}

/// Opens topic `id`'s herdr session in the terminal of the settings.
#[tauri::command]
async fn open_session(app: State<'_, App>, id: u64) -> Result<(), String> {
    let (session, folder, terminal) = {
        let inner = app.inner.lock().unwrap();
        let topic = inner.model.topic(id)?;
        (
            topic.session.clone(),
            topic.folder.clone(),
            inner.model.settings.terminal,
        )
    };
    herdr::open(&app.shell, terminal, &session, &folder)
}

#[tauri::command]
fn open_url(app: State<App>, url: &str) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(format!("{url} is not a web address"));
    }
    app.shell.run("open", &[url], None).map(drop)
}

/// Saves the Linear API key in the keychain; a blank key forgets it.
#[tauri::command]
fn set_linear_key(app: State<App>, key: &str) -> Result<Snapshot, String> {
    linear::set_key(key)?;
    app.refresh_now.store(true, Ordering::Relaxed);
    let mut inner = app.inner.lock().unwrap();
    inner.has_linear_key = !key.trim().is_empty();
    Ok(Snapshot::of(&inner))
}

/// Reads herdr every `session_seconds`, and Linear and GitHub every `refresh_minutes` or
/// when asked; sends the new snapshot and notifies what changed.
fn watch(handle: AppHandle) {
    let app = handle.state::<App>();
    let mut gh_login = String::new();
    let mut last_sessions: Option<Instant> = None;
    let mut last_remote: Option<Instant> = None;
    let mut first = true;
    loop {
        let (topics, settings, before) = {
            let inner = app.inner.lock().unwrap();
            (
                inner.model.topics.list.clone(),
                inner.model.settings.clone(),
                inner.live.clone(),
            )
        };
        let due = |last: Option<Instant>, every: u64| {
            last.is_none_or(|t| t.elapsed() >= Duration::from_secs(every))
        };
        let remote = app.refresh_now.swap(false, Ordering::Relaxed)
            || due(last_remote, u64::from(settings.refresh_minutes.max(1)) * 60);
        let sessions = remote || due(last_sessions, u64::from(settings.session_seconds.max(5)));
        if sessions {
            let mut after = if remote {
                if gh_login.is_empty() {
                    gh_login = github::me(&app.shell).unwrap_or_default();
                }
                let mut live = live::read_remote(&app.shell, &topics, &before, &gh_login);
                live.refreshed_at = Some(now());
                last_remote = Some(Instant::now());
                live
            } else {
                before.clone()
            };
            after.sessions = live::read_sessions(&app.shell, &topics);
            last_sessions = Some(Instant::now());
            // The first read after launch only sets what later reads compare with: live.json
            // may be days old.
            let alerts = if !first {
                live::alerts(
                    &topics,
                    &before,
                    &after,
                    settings.notify_comments,
                    settings.notify_sessions,
                )
            } else {
                vec![]
            };
            first = false;
            for alert in alerts {
                let _ = app.shell.notify(&alert.title, &alert.body);
            }
            let snapshot = {
                let mut inner = app.inner.lock().unwrap();
                inner.live = after;
                let _ = store::save(&app.dir.join(LIVE), &inner.live);
                Snapshot::of(&inner)
            };
            let _ = handle.emit("snapshot", snapshot);
        }
        std::thread::sleep(Duration::from_secs(1));
    }
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
                topics: store::load(&dir.join(TOPICS)),
                settings: store::load(&dir.join(SETTINGS)),
            };
            app.manage(App {
                shell: Shell::login(),
                inner: Mutex::new(Inner {
                    model,
                    history: History::default(),
                    live: store::load(&dir.join(LIVE)),
                    has_linear_key: linear::key().is_some(),
                }),
                dir,
                refresh_now: AtomicBool::new(false),
            });
            app.set_menu(menu(app.handle())?)?;
            let handle = app.handle().clone();
            std::thread::spawn(move || watch(handle));
            Ok(())
        })
        .on_menu_event(|app, event| {
            let _ = app.emit("menu", event.id().as_ref());
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            create_topic,
            topic_from_issue,
            rename,
            move_topic,
            activate,
            park,
            finish,
            rework,
            block,
            unblock,
            add_step,
            toggle_step,
            move_step,
            add_note,
            add_link,
            remove_part,
            set_initiative,
            set_session,
            set_folder,
            remove,
            set_settings,
            undo,
            redo,
            search_topics,
            fuzzy,
            refresh,
            open_session,
            open_url,
            set_linear_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running maquereau");
}
