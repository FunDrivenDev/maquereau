//! The one seam between the app and what it reads from outside: Linear, GitHub and herdr.
//!
//! `Sources` is what the readers need, and nothing more; `Real` reads it through the
//! login shell's tools and the keychain, and the tests read it from `fake::Fake`. Every
//! method only reads: an outside source is never written from here.

use crate::herdr::{self, Session};
use crate::links::Kind;
use crate::live::{Issue, LinkStatus};
use crate::model::Initiative;
use crate::shell::Shell;
use crate::{github, linear};

pub trait Sources {
    /// The Linear API key, when one is set. It is read once per remote read, and handed
    /// back to each Linear method.
    fn linear_key(&self) -> Option<String>;
    /// The open Linear issues assigned to Raphaël.
    fn assigned(&self, key: &str) -> Result<Vec<Issue>, String>;
    /// The sub-issues of those, two levels down.
    fn sub_issues(&self, key: &str) -> Result<Vec<Issue>, String>;
    /// The state of Linear issue `issue` (`BIM-123`).
    fn linear_status(&self, key: &str, issue: &str) -> Result<LinkStatus, String>;
    /// The open Linear initiatives.
    fn initiatives(&self, key: &str) -> Result<Vec<Initiative>, String>;
    /// The state of a pull request or GitHub issue, counting the comments of others than
    /// `me`.
    fn github_status(&self, url: &str, kind: Kind, me: &str) -> Result<LinkStatus, String>;
    /// The login `gh` is signed in as.
    fn gh_login(&self) -> Result<String, String>;
    /// The herdr session `name`.
    fn session(&self, name: &str) -> Session;
}

/// The sources as they are: `gh`, `curl` and `herdr` run with the login shell's `PATH`,
/// and the Linear key from the macOS keychain.
pub struct Real {
    shell: Shell,
}

impl Real {
    pub fn new(shell: Shell) -> Self {
        Self { shell }
    }
}

impl Sources for Real {
    fn linear_key(&self) -> Option<String> {
        linear::key()
    }

    fn assigned(&self, key: &str) -> Result<Vec<Issue>, String> {
        linear::assigned(&self.shell, key)
    }

    fn sub_issues(&self, key: &str) -> Result<Vec<Issue>, String> {
        linear::sub_issues(&self.shell, key)
    }

    fn linear_status(&self, key: &str, issue: &str) -> Result<LinkStatus, String> {
        linear::status(&self.shell, key, issue)
    }

    fn initiatives(&self, key: &str) -> Result<Vec<Initiative>, String> {
        linear::open_initiatives(&self.shell, key)
    }

    fn github_status(&self, url: &str, kind: Kind, me: &str) -> Result<LinkStatus, String> {
        github::status(&self.shell, url, kind, me)
    }

    fn gh_login(&self) -> Result<String, String> {
        github::me(&self.shell)
    }

    fn session(&self, name: &str) -> Session {
        herdr::session(&self.shell, name)
    }
}

/// Sources held in memory, for the tests: each answer is set beforehand, and each read is
/// counted.
#[cfg(test)]
pub mod fake {
    use std::cell::{Cell, RefCell};
    use std::collections::BTreeMap;

    use super::*;

    pub struct Fake {
        pub key: Option<String>,
        pub assigned: Result<Vec<Issue>, String>,
        pub sub_issues: Result<Vec<Issue>, String>,
        pub initiatives: Result<Vec<Initiative>, String>,
        /// By issue key or URL; a link missing here fails to read.
        pub statuses: RefCell<BTreeMap<String, Result<LinkStatus, String>>>,
        pub login: String,
        pub sessions: RefCell<BTreeMap<String, Session>>,
        /// How many times each was read.
        pub status_reads: Cell<u32>,
        pub login_reads: Cell<u32>,
        pub session_reads: Cell<u32>,
        pub key_reads: Cell<u32>,
    }

    impl Fake {
        /// Sources with a Linear key and nothing in them.
        pub fn with_key() -> Self {
            Self {
                key: Some("lin_api_test".into()),
                assigned: Ok(vec![]),
                sub_issues: Ok(vec![]),
                initiatives: Ok(vec![]),
                statuses: RefCell::default(),
                login: "me".into(),
                sessions: RefCell::default(),
                status_reads: Cell::default(),
                login_reads: Cell::default(),
                session_reads: Cell::default(),
                key_reads: Cell::default(),
            }
        }

        pub fn set_status(&self, link: &str, status: Result<LinkStatus, String>) {
            self.statuses.borrow_mut().insert(link.into(), status);
        }

        pub fn set_session(&self, name: &str, session: Session) {
            self.sessions.borrow_mut().insert(name.into(), session);
        }

        fn status(&self, link: &str) -> Result<LinkStatus, String> {
            self.status_reads.set(self.status_reads.get() + 1);
            self.statuses
                .borrow()
                .get(link)
                .cloned()
                .unwrap_or_else(|| Err(format!("{link}: not found")))
        }
    }

    fn bump(count: &Cell<u32>) {
        count.set(count.get() + 1);
    }

    impl Sources for Fake {
        fn linear_key(&self) -> Option<String> {
            bump(&self.key_reads);
            self.key.clone()
        }

        fn assigned(&self, _: &str) -> Result<Vec<Issue>, String> {
            self.assigned.clone()
        }

        fn sub_issues(&self, _: &str) -> Result<Vec<Issue>, String> {
            self.sub_issues.clone()
        }

        fn linear_status(&self, _: &str, issue: &str) -> Result<LinkStatus, String> {
            self.status(issue)
        }

        fn initiatives(&self, _: &str) -> Result<Vec<Initiative>, String> {
            self.initiatives.clone()
        }

        fn github_status(&self, url: &str, _: Kind, _: &str) -> Result<LinkStatus, String> {
            self.status(url)
        }

        fn gh_login(&self) -> Result<String, String> {
            bump(&self.login_reads);
            Ok(self.login.clone())
        }

        fn session(&self, name: &str) -> Session {
            bump(&self.session_reads);
            self.sessions
                .borrow()
                .get(name)
                .cloned()
                .unwrap_or_default()
        }
    }
}
