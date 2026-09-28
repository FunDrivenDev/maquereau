//! maquereau keeps Raphaël on four priority topics, one per slot (Feature, Bug / Run,
//! Exploration, Tooling), with their issues, pull requests, notes, next step, flow times
//! and herdr session.
//!
//! The front end holds no state of its own: each command returns a `Snapshot`, and each
//! one that changes something goes through `History::perform`, so it can be undone. A
//! background thread reads Linear, GitHub and herdr, and sends a new `Snapshot` as a
//! `snapshot` event when it has.

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
use tauri_plugin_notification::NotificationExt;

use history::History;
use live::{Issue, Live};
use model::{Block, Change, Initiative, Link, Model, Note, Rework, Slot, Stage, Step, Time, Topic};
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

fn required(text: &str, what: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err(format!("{what} is needed"));
    }
    Ok(text.to_owned())
}

impl App {
    /// Runs `action` on the model, saves it, and returns the new snapshot.
    fn update(
        &self,
        action: impl FnOnce(&mut Inner) -> Result<(), String>,
    ) -> Result<Snapshot, String> {
        let mut inner = self.inner.lock().unwrap();
        action(&mut inner)?;
        store::save(&self.dir.join(TOPICS), &inner.model.topics)?;
        store::save(&self.dir.join(SETTINGS), &inner.model.settings)?;
        Ok(Snapshot::of(&inner))
    }

    /// Performs the change `action` builds, as the action it names.
    fn change(
        &self,
        action: impl FnOnce(&mut Model) -> Result<(String, Change), String>,
    ) -> Result<Snapshot, String> {
        self.update(|inner| {
            let (label, change) = action(&mut inner.model)?;
            inner.history.perform(&mut inner.model, label, change)
        })
    }

    /// Changes topic `id` through `action`, which returns the action's label.
    fn edit(
        &self,
        id: u64,
        action: impl FnOnce(&mut Topic, &mut Model) -> Result<String, String>,
    ) -> Result<Snapshot, String> {
        self.change(|model| {
            let mut topic = model.topic(id)?.clone();
            let label = action(&mut topic, model)?;
            Ok((label, Change::put(topic)))
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
    /// The assigned Linear issues no open topic links to: the hidden backlog.
    backlog: Vec<Issue>,
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
            backlog: inner
                .live
                .assigned
                .iter()
                .filter(|i| !linked.contains(&i.key))
                .cloned()
                .collect(),
            settings: inner.model.settings.clone(),
            has_linear_key: inner.has_linear_key,
            now,
            undo: inner.history.next_undo().map(str::to_owned),
            redo: inner.history.next_redo().map(str::to_owned),
        }
    }
}

fn new_topic(model: &mut Model, title: String, slot: Slot) -> Topic {
    Topic {
        id: model.take_id(),
        session: model::session_name(&title),
        folder: model.settings.folder.clone(),
        title,
        slot,
        stage: Stage::Queued,
        created_at: now(),
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
    let title = required(title, "a title")?;
    app.change(|model| {
        let topic = new_topic(model, title.clone(), slot);
        let label = format!("Add “{title}”");
        if activate {
            let changes = model.activation(topic, now());
            return Ok((label, Change::Batch(changes)));
        }
        Ok((label, Change::put(topic)))
    })
}

/// Makes a topic of the assigned Linear issue `key` and puts it in `slot`.
#[tauri::command]
fn topic_from_issue(app: State<App>, key: &str, slot: Slot) -> Result<Snapshot, String> {
    app.update(
        |Inner {
             model,
             history,
             live,
             ..
         }| {
            let issue = live
                .assigned
                .iter()
                .find(|i| i.key == key)
                .ok_or_else(|| format!("no issue {key}"))?;
            let mut topic = new_topic(model, issue.title.clone(), slot);
            topic.links.push(Link {
                id: model.take_id(),
                url: issue.url.clone(),
                kind: links::Kind::LinearIssue,
            });
            topic.initiative = issue.initiatives.first().cloned();
            let changes = model.activation(topic, now());
            history.perform(model, format!("Focus on {key}"), Change::Batch(changes))
        },
    )
}

#[tauri::command]
fn rename(app: State<App>, id: u64, title: &str) -> Result<Snapshot, String> {
    let title = required(title, "a title")?;
    app.edit(id, |topic, _| {
        let label = format!("Rename “{}”", topic.title);
        topic.title = title;
        Ok(label)
    })
}

/// Moves a topic to another slot. An active topic stays active there, parking the one it
/// replaces.
#[tauri::command]
fn move_topic(app: State<App>, id: u64, slot: Slot) -> Result<Snapshot, String> {
    app.change(|model| {
        let mut topic = model.topic(id)?.clone();
        let label = format!("Move “{}” to {}", topic.title, slot.label());
        let active = topic.stage == Stage::Active;
        topic.slot = slot;
        if active {
            return Ok((label, Change::Batch(model.activation(topic, now()))));
        }
        Ok((label, Change::put(topic)))
    })
}

/// Puts a topic in its slot, parking the one there; a done topic comes back as rework.
#[tauri::command]
fn activate(app: State<App>, id: u64) -> Result<Snapshot, String> {
    app.change(|model| {
        let mut topic = model.topic(id)?.clone();
        let label = format!("Focus on “{}”", topic.title);
        if topic.stage == Stage::Done {
            topic.reworks.push(Rework {
                id: model.take_id(),
                reason: "Reopened".into(),
                at: now(),
            });
        }
        Ok((label, Change::Batch(model.activation(topic, now()))))
    })
}

#[tauri::command]
fn park(app: State<App>, id: u64) -> Result<Snapshot, String> {
    app.edit(id, |topic, _| {
        topic.stage = Stage::Queued;
        Ok(format!("Park “{}”", topic.title))
    })
}

#[tauri::command]
fn finish(app: State<App>, id: u64) -> Result<Snapshot, String> {
    app.edit(id, |topic, _| {
        let now = now();
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
#[tauri::command]
fn rework(app: State<App>, id: u64, reason: &str) -> Result<Snapshot, String> {
    let reason = required(reason, "a reason")?;
    app.change(|model| {
        let mut topic = model.topic(id)?.clone();
        topic.reworks.push(Rework {
            id: model.take_id(),
            reason,
            at: now(),
        });
        let label = format!("Rework on “{}”", topic.title);
        if topic.stage == Stage::Done {
            return Ok((label, Change::Batch(model.activation(topic, now()))));
        }
        Ok((label, Change::put(topic)))
    })
}

#[tauri::command]
fn block(app: State<App>, id: u64, reason: &str) -> Result<Snapshot, String> {
    let reason = required(reason, "a reason")?;
    app.edit(id, |topic, model| {
        if topic.is_blocked() {
            return Err("already blocked".into());
        }
        topic.blocks.push(Block {
            id: model.take_id(),
            reason,
            since: now(),
            until: None,
        });
        Ok(format!("Block “{}”", topic.title))
    })
}

#[tauri::command]
fn unblock(app: State<App>, id: u64) -> Result<Snapshot, String> {
    app.edit(id, |topic, _| {
        let block = topic.open_block().ok_or("not blocked")?;
        block.until = Some(now());
        Ok(format!("Unblock “{}”", topic.title))
    })
}

#[tauri::command]
fn add_step(app: State<App>, id: u64, text: &str) -> Result<Snapshot, String> {
    let text = required(text, "a step")?;
    app.edit(id, |topic, model| {
        topic.steps.push(Step {
            id: model.take_id(),
            text: text.clone(),
            done_at: None,
        });
        Ok(format!("Add step “{text}”"))
    })
}

/// Marks step `part` done, or not done any more.
#[tauri::command]
fn toggle_step(app: State<App>, id: u64, part: u64) -> Result<Snapshot, String> {
    app.edit(id, |topic, _| {
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
            None => Some(now()),
        };
        Ok(label)
    })
}

/// Moves step `part` up (`-1`) or down (`1`) among the steps.
#[tauri::command]
fn move_step(app: State<App>, id: u64, part: u64, delta: i32) -> Result<Snapshot, String> {
    app.edit(id, |topic, _| {
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

#[tauri::command]
fn add_note(app: State<App>, id: u64, text: &str) -> Result<Snapshot, String> {
    let text = required(text, "a note")?;
    app.edit(id, |topic, model| {
        topic.notes.push(Note {
            id: model.take_id(),
            text,
            at: now(),
        });
        Ok(format!("Note on “{}”", topic.title))
    })
}

/// Links an issue, pull request, Slack thread or any page, from its URL or Linear key.
#[tauri::command]
fn add_link(app: State<App>, id: u64, text: &str) -> Result<Snapshot, String> {
    let (url, kind) = links::parse(text).ok_or("not a URL nor a Linear key")?;
    app.edit(id, |topic, model| {
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
#[tauri::command]
fn remove_part(app: State<App>, id: u64, part: u64) -> Result<Snapshot, String> {
    app.edit(id, |topic, _| {
        if !topic.remove_part(part) {
            return Err("nothing to remove".into());
        }
        Ok(format!("Remove from “{}”", topic.title))
    })
}

#[tauri::command]
fn set_initiative(
    app: State<App>,
    id: u64,
    initiative: Option<Initiative>,
) -> Result<Snapshot, String> {
    app.edit(id, |topic, _| {
        let label = match &initiative {
            Some(i) => format!("Link “{}” to {}", topic.title, i.name),
            None => format!("Unlink “{}” from its initiative", topic.title),
        };
        topic.initiative = initiative;
        Ok(label)
    })
}

#[tauri::command]
fn set_session(app: State<App>, id: u64, session: &str) -> Result<Snapshot, String> {
    let session = session.trim().to_owned();
    if !model::is_session_name(&session) {
        return Err("a session name is lowercase letters, digits, - and _, 32 at most".into());
    }
    app.edit(id, |topic, _| {
        topic.session = session;
        Ok(format!("Rename the session of “{}”", topic.title))
    })
}

#[tauri::command]
fn set_folder(app: State<App>, id: u64, folder: &str) -> Result<Snapshot, String> {
    let folder = required(folder, "a folder")?;
    if !std::path::Path::new(&shell::expand(&folder)).is_dir() {
        return Err(format!("{folder} is not a folder"));
    }
    app.edit(id, |topic, _| {
        topic.folder = folder;
        Ok(format!("Change the folder of “{}”", topic.title))
    })
}

#[tauri::command]
fn remove(app: State<App>, id: u64) -> Result<Snapshot, String> {
    app.change(|model| {
        let title = model.topic(id)?.title.clone();
        let change = Change::Put {
            id,
            index: 0,
            topic: None,
        };
        Ok((format!("Delete “{title}”"), change))
    })
}

#[tauri::command]
fn set_settings(app: State<App>, settings: Settings) -> Result<Snapshot, String> {
    app.update(|Inner { model, history, .. }| {
        if model.settings == settings {
            return Ok(());
        }
        history.perform(model, "Change settings", Change::Settings(settings))
    })
}

#[tauri::command]
fn undo(app: State<App>) -> Result<Snapshot, String> {
    app.update(|Inner { model, history, .. }| history.undo(model).map(drop))
}

#[tauri::command]
fn redo(app: State<App>) -> Result<Snapshot, String> {
    app.update(|Inner { model, history, .. }| history.redo(model).map(drop))
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
    app.update(|inner| {
        inner.has_linear_key = !key.trim().is_empty();
        Ok(())
    })
}

/// Reads herdr every `session_seconds`, and Linear and GitHub every `refresh_minutes` or
/// when asked; sends the new snapshot and notifies what changed.
fn watch(handle: AppHandle) {
    let app = handle.state::<App>();
    let mut gh_login = String::new();
    let mut last_sessions: Option<Instant> = None;
    let mut last_remote: Option<Instant> = None;
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
            // The first read after launch only sets what later reads compare with.
            let alerts = if before.refreshed_at.is_some() || !before.sessions.is_empty() {
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
            for alert in alerts {
                let _ = handle
                    .notification()
                    .builder()
                    .title(alert.title)
                    .body(alert.body)
                    .show();
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
        .plugin(tauri_plugin_notification::init())
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
