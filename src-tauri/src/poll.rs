//! When and how the outside sources are read: herdr every `session_seconds`, Linear and
//! GitHub every `refresh_minutes` or when asked, and what of it deserves a notification.
//!
//! `Poller::tick` holds every decision of the background loop; `lib.rs` only sleeps,
//! calls it, saves what it read and sends it. What is read lands in `Live`, never in the
//! undo history.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::herdr::Session;
use crate::links::{self, Kind};
use crate::live::{self, Alert, Live, open_topics};
use crate::model::{Time, Topic};
use crate::settings::Settings;
use crate::sources::Sources;

/// What the loop remembers from one tick to the next.
pub struct Poller {
    gh_login: String,
    last_sessions: Option<Instant>,
    last_remote: Option<Instant>,
    /// No read has been made since launch.
    first: bool,
}

/// What a tick read.
#[derive(Debug)]
pub struct Read {
    pub live: Live,
    /// What changed since the last read that Raphaël should hear about.
    pub alerts: Vec<Alert>,
}

impl Default for Poller {
    fn default() -> Self {
        Self {
            gh_login: String::new(),
            last_sessions: None,
            last_remote: None,
            first: true,
        }
    }
}

impl Poller {
    /// Reads what is due, given `before`, what was last read: the herdr sessions every
    /// `session_seconds`, and Linear and GitHub as well every `refresh_minutes` or when
    /// `refresh_now`, both already within `BOUNDS`. `None` when nothing was due. `clock`
    /// gives the time now, for the scheduling and for `refreshed_at`; it is asked after
    /// each read, so a period runs from the end of the read.
    pub fn tick(
        &mut self,
        sources: &impl Sources,
        clock: impl Fn() -> (Instant, Time),
        refresh_now: bool,
        topics: &[Topic],
        settings: &Settings,
        before: &Live,
    ) -> Option<Read> {
        let (now, _) = clock();
        let due = |last: Option<Instant>, every: u64| {
            last.is_none_or(|t| now.duration_since(t) >= Duration::from_secs(every))
        };
        let remote = refresh_now || due(self.last_remote, u64::from(settings.refresh_minutes) * 60);
        let sessions = remote || due(self.last_sessions, u64::from(settings.session_seconds));
        if !sessions {
            return None;
        }
        let mut after = if remote {
            if self.gh_login.is_empty() {
                self.gh_login = sources.gh_login().unwrap_or_default();
            }
            let mut live = read_remote(sources, topics, before, &self.gh_login);
            let (at, time) = clock();
            live.refreshed_at = Some(time);
            self.last_remote = Some(at);
            live
        } else {
            before.clone()
        };
        after.sessions = read_sessions(sources, topics);
        self.last_sessions = Some(clock().0);
        // The first read after launch only sets what later reads compare with: live.json
        // may be days old.
        let alerts = if self.first {
            vec![]
        } else {
            live::alerts(
                topics,
                before,
                &after,
                settings.notify_comments,
                settings.notify_sessions,
            )
        };
        self.first = false;
        Some(Read {
            live: after,
            alerts,
        })
    }
}

/// Reads Linear and GitHub for the links of the open topics, Raphaël's assigned issues and
/// the open initiatives. A source that fails keeps what was read before, with its error.
pub fn read_remote(
    sources: &impl Sources,
    topics: &[Topic],
    before: &Live,
    gh_login: &str,
) -> Live {
    let mut live = before.clone();
    live.errors.clear();
    let key = sources.linear_key();
    let mut links = BTreeMap::new();
    for link in open_topics(topics).flat_map(|t| &t.links) {
        if links.contains_key(&link.url) {
            continue;
        }
        let status = match (link.kind, &key) {
            (Kind::LinearIssue, Some(key)) => links::key(&link.url)
                .ok_or_else(|| format!("{}: not a Linear issue", link.url))
                .and_then(|issue| sources.linear_status(key, &issue)),
            (Kind::PullRequest | Kind::GithubIssue, _) => {
                sources.github_status(&link.url, link.kind, gh_login)
            }
            _ => continue,
        };
        match status {
            Ok(status) => {
                links.insert(link.url.clone(), status);
            }
            Err(e) => {
                live.errors.push(e);
                if let Some(old) = before.links.get(&link.url) {
                    links.insert(link.url.clone(), old.clone());
                }
            }
        }
    }
    live.links = links;
    match &key {
        Some(key) => {
            match sources.assigned(key) {
                Ok(issues) => live.assigned = issues,
                Err(e) => live.errors.push(e),
            }
            match sources.sub_issues(key) {
                Ok(issues) => live.sub_issues = issues,
                Err(e) => live.errors.push(e),
            }
            match sources.initiatives(key) {
                Ok(list) => live.initiatives = list,
                Err(e) => live.errors.push(e),
            }
        }
        None => live
            .errors
            .push("Linear: no API key; set one in Settings (⌘,)".into()),
    }
    live
}

/// Reads the herdr session of each open topic.
pub fn read_sessions(sources: &impl Sources, topics: &[Topic]) -> BTreeMap<String, Session> {
    let mut sessions = BTreeMap::new();
    for topic in open_topics(topics) {
        if !sessions.contains_key(&topic.session) {
            sessions.insert(topic.session.clone(), sources.session(&topic.session));
        }
    }
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::Issue;
    use crate::live::tests::{status, with_agent};
    use crate::model::tests::topic;
    use crate::model::{Link, Model, Slot};
    use crate::sources::fake::Fake;
    use std::cell::Cell;

    /// A topic linking each of `urls`, with its kind read from the URL.
    fn linking(model: &mut Model, title: &str, urls: &[&str]) -> Topic {
        let mut t = topic(model, title, Slot::Feature);
        for (id, url) in urls.iter().enumerate() {
            let (url, kind) = links::parse(url).unwrap();
            t.links.push(Link {
                id: id as u64 + 10,
                url,
                kind,
            });
        }
        t
    }

    const PR: &str = "https://github.com/o/r/pull/1";

    #[test]
    fn a_link_shared_by_two_topics_is_read_once() {
        let mut model = Model::default();
        let topics = [
            linking(&mut model, "Export", &["BIM-1", PR, "https://example.com"]),
            linking(&mut model, "Import", &["BIM-1", PR]),
        ];
        let sources = Fake::with_key();
        sources.set_status("BIM-1", Ok(status(0)));
        sources.set_status(PR, Ok(status(3)));
        let live = read_remote(&sources, &topics, &Live::default(), "me");
        assert_eq!(sources.status_reads.get(), 2);
        assert_eq!(live.links.keys().collect::<Vec<_>>(), ["BIM-1", PR]);
        assert!(live.errors.is_empty());
    }

    #[test]
    fn a_failed_read_keeps_the_last_good_status_and_says_why() {
        let mut model = Model::default();
        let topics = [linking(&mut model, "Export", &["BIM-1", PR])];
        let sources = Fake::with_key();
        sources.set_status("BIM-1", Ok(status(1)));
        sources.set_status(PR, Ok(status(2)));
        let before = read_remote(&sources, &topics, &Live::default(), "me");
        sources.set_status(PR, Err("gh: offline".into()));
        sources.set_status("BIM-1", Ok(status(4)));
        let after = read_remote(&sources, &topics, &before, "me");
        assert_eq!(after.links[PR], status(2));
        assert_eq!(after.links["BIM-1"], status(4));
        assert_eq!(after.errors, ["gh: offline"]);
        sources.set_status(PR, Ok(status(5)));
        let again = read_remote(&sources, &topics, &after, "me");
        assert!(again.errors.is_empty(), "the errors of a read are cleared");
        assert_eq!(again.links[PR], status(5));
    }

    #[test]
    fn without_a_linear_key_linear_is_skipped_with_an_error_and_github_still_read() {
        let mut model = Model::default();
        let topics = [linking(&mut model, "Export", &["BIM-1", PR])];
        let sources = Fake {
            key: None,
            ..Fake::with_key()
        };
        sources.set_status(PR, Ok(status(0)));
        let before = Live {
            assigned: vec![Issue::default()],
            links: [("BIM-1".into(), status(1))].into(),
            ..Live::default()
        };
        let live = read_remote(&sources, &topics, &before, "me");
        assert_eq!(live.links.keys().collect::<Vec<_>>(), [PR]);
        assert_eq!(
            live.errors,
            ["Linear: no API key; set one in Settings (⌘,)"]
        );
        assert_eq!(live.assigned, before.assigned, "what was read before stays");
        assert_eq!(sources.key_reads.get(), 1);
    }

    #[test]
    fn a_failing_linear_list_keeps_the_last_one() {
        let mut model = Model::default();
        let topics = [topic(&mut model, "Export", Slot::Feature)];
        let before = Live {
            assigned: vec![Issue::default()],
            ..Live::default()
        };
        let sources = Fake {
            assigned: Err("Linear: down".into()),
            ..Fake::with_key()
        };
        let live = read_remote(&sources, &topics, &before, "me");
        assert_eq!(live.assigned, before.assigned);
        assert_eq!(live.errors, ["Linear: down"]);
    }

    #[test]
    fn each_open_session_is_read_once() {
        let mut model = Model::default();
        let topics = [
            topic(&mut model, "Export", Slot::Feature),
            topic(&mut model, "Export", Slot::BugRun),
        ];
        let sources = Fake::with_key();
        sources.set_session(
            "mq-export",
            with_agent("blocked").sessions["mq-export"].clone(),
        );
        let sessions = read_sessions(&sources, &topics);
        assert_eq!(sources.session_reads.get(), 1);
        assert_eq!(sessions["mq-export"].agents[0].status, "blocked");
    }

    /// A clock that stands still until moved.
    struct Clock {
        start: Instant,
        secs: Cell<u64>,
    }

    impl Clock {
        fn new() -> Self {
            Self {
                start: Instant::now(),
                secs: Cell::new(0),
            }
        }

        fn at(&self, secs: u64) -> impl Fn() -> (Instant, Time) + '_ {
            self.secs.set(secs);
            move || {
                let secs = self.secs.get();
                (self.start + Duration::from_secs(secs), 1_000 + secs as Time)
            }
        }
    }

    /// A poller over a topic's sources, ticking on a clock it moves.
    struct Bench {
        poller: Poller,
        sources: Fake,
        clock: Clock,
        topics: Vec<Topic>,
        settings: Settings,
        live: Live,
    }

    impl Bench {
        fn new(settings: Settings) -> Self {
            let mut model = Model::default();
            Self {
                poller: Poller::default(),
                sources: Fake::with_key(),
                clock: Clock::new(),
                topics: vec![topic(&mut model, "Export", Slot::Feature)],
                settings,
                live: Live::default(),
            }
        }

        /// Ticks at `secs`, keeping what was read; whether it read the sessions, and
        /// whether it read Linear and GitHub too.
        fn at(&mut self, secs: u64, refresh_now: bool) -> (bool, bool) {
            let reads = self.sources.session_reads.get();
            let refreshed = self.live.refreshed_at;
            let read = self.poller.tick(
                &self.sources,
                self.clock.at(secs),
                refresh_now,
                &self.topics,
                &self.settings,
                &self.live,
            );
            if let Some(read) = read {
                self.live = read.live;
            }
            (
                self.sources.session_reads.get() > reads,
                self.live.refreshed_at != refreshed,
            )
        }
    }

    #[test]
    fn sessions_are_read_every_session_seconds_and_remote_every_refresh_minutes() {
        let mut bench = Bench::new(Settings {
            session_seconds: 15,
            refresh_minutes: 2,
            ..Settings::default()
        });
        let mut at = |secs| bench.at(secs, false);
        assert_eq!(at(0), (true, true), "the first tick reads everything");
        assert_eq!(at(14), (false, false));
        assert_eq!(at(15), (true, false));
        assert_eq!(at(29), (false, false));
        assert_eq!(at(119), (true, false));
        assert_eq!(at(120), (true, true));
        assert_eq!(
            bench.sources.login_reads.get(),
            1,
            "the gh login is read once"
        );
    }

    #[test]
    fn refresh_now_reads_remote_before_it_is_due() {
        let mut bench = Bench::new(Settings::default());
        assert_eq!(bench.at(0, false), (true, true));
        assert_eq!(bench.at(1, true), (true, true));
        assert_eq!(bench.at(2, false), (false, false));
        assert_eq!(bench.live.refreshed_at, Some(1_001));
    }

    #[test]
    fn the_first_read_after_launch_alerts_nothing_and_a_later_change_does() {
        let mut model = Model::default();
        let topics = [topic(&mut model, "Export", Slot::Feature)];
        let sources = Fake::with_key();
        let clock = Clock::new();
        let settings = Settings::default();
        let mut poller = Poller::default();
        let session = |status: &str| with_agent(status).sessions["mq-export"].clone();
        // live.json, from the last launch.
        let saved = with_agent("working");
        sources.set_session("mq-export", session("blocked"));
        let first = poller
            .tick(&sources, clock.at(0), false, &topics, &settings, &saved)
            .unwrap();
        assert!(first.alerts.is_empty());
        sources.set_session("mq-export", session("done"));
        let second = poller
            .tick(
                &sources,
                clock.at(15),
                false,
                &topics,
                &settings,
                &first.live,
            )
            .unwrap();
        assert_eq!(second.alerts[0].body, "claude has finished");
        let third = poller
            .tick(
                &sources,
                clock.at(30),
                false,
                &topics,
                &settings,
                &second.live,
            )
            .unwrap();
        assert!(third.alerts.is_empty(), "an unchanged state alerts once");
    }
}
