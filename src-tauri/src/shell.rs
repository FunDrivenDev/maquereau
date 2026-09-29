//! Runs the command-line tools the app relies on (`gh`, `curl`, `herdr`, `open`).
//!
//! An app started from the Finder gets a bare `PATH`, without Homebrew or mise, so the
//! `PATH` of Raphaël's interactive login shell is read once at launch and given to each
//! command.

use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct Shell {
    path: String,
}

const MARK: &str = "__maquereau_path__";

impl Shell {
    pub fn login() -> Self {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        let fallback = std::env::var("PATH").unwrap_or_default();
        let printed = Command::new(shell)
            .args(["-ilc", &format!("printf '{MARK}%s{MARK}' \"$PATH\"")])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
        let path = printed
            .as_deref()
            .and_then(|out| out.split(MARK).nth(1))
            .filter(|p| !p.is_empty())
            .map(|p| format!("{p}:{fallback}"))
            .unwrap_or(fallback);
        Self { path }
    }

    pub fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command.env("PATH", &self.path);
        command
    }

    /// Runs `program` with `args`, writing `input` to its standard input, and returns its
    /// standard output; its standard error becomes the error when it fails.
    pub fn run(&self, program: &str, args: &[&str], input: Option<&str>) -> Result<String, String> {
        let mut child = self
            .command(program)
            .args(args)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("{program}: {e}"))?;
        if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
            stdin
                .write_all(input.as_bytes())
                .map_err(|e| format!("{program}: {e}"))?;
        }
        let output = child
            .wait_with_output()
            .map_err(|e| format!("{program}: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let message = if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            };
            Err(format!("{program}: {message}"))
        }
    }

    /// Shows a macOS notification. Through `osascript`, it needs neither a signed app nor
    /// the macOS SDK at build time, which the Linux CI lint of the macOS target lacks.
    pub fn notify(&self, title: &str, body: &str) -> Result<(), String> {
        let script = format!(
            "display notification {} with title {}",
            apple_string(body),
            apple_string(title)
        );
        self.run("osascript", &["-e", &script], None).map(drop)
    }
}

/// `text` quoted for a POSIX shell.
pub fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// `text` as an AppleScript string literal.
pub fn apple_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', r"\\").replace('"', "\\\""))
}

/// `path` with a leading `~` replaced by the home folder.
pub fn expand(path: &str) -> String {
    match (path.strip_prefix('~'), std::env::var("HOME")) {
        (Some(rest), Ok(home)) if rest.is_empty() || rest.starts_with('/') => {
            format!("{home}{rest}")
        }
        _ => path.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_for_applescript() {
        assert_eq!(apple_string(r#"say "hi" \ bye"#), r#""say \"hi\" \\ bye""#);
    }

    #[test]
    fn quotes_for_the_shell() {
        assert_eq!(quote("a b"), "'a b'");
        assert_eq!(quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn expands_the_home_folder_only_at_the_start() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand("~/x"), format!("{home}/x"));
        assert_eq!(expand("~"), home);
        assert_eq!(expand("~other/x"), "~other/x");
        assert_eq!(expand("/a/~"), "/a/~");
    }
}
