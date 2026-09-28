//! herdr (<https://herdr.dev>) holds each topic's terminals and coding agents in a named
//! session. The app opens that session in a terminal, and reads its agents' states to
//! tell when one needs an answer.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::settings::Terminal;
use crate::shell::{Shell, apple_string, expand, quote};

/// What a session's agents are doing. `idle`, `working`, `blocked`, `done` and `unknown`
/// are herdr's states: `blocked` waits for an answer, `done` has finished unseen.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub running: bool,
    pub agents: Vec<Agent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// The agent's name, else its kind (`claude`), else its pane.
    pub name: String,
    pub status: String,
}

pub fn session(shell: &Shell, name: &str) -> Session {
    let args = ["--session", name, "agent", "list"];
    let out = shell
        .run("herdr", &args, None)
        .unwrap_or_else(|error| error);
    // herdr prints its JSON to standard output even when the server is not running.
    let json = out.find('{').map_or("", |i| &out[i..]);
    serde_json::from_str::<Value>(json)
        .map(|v| read(&v))
        .unwrap_or_default()
}

fn read(value: &Value) -> Session {
    if value.get("error").is_some() {
        return Session::default();
    }
    let mut agents = Vec::new();
    collect(&value["result"], &mut agents);
    Session {
        running: true,
        agents,
    }
}

/// Finds the agents anywhere in `value`: the objects carrying an agent status.
fn collect(value: &Value, agents: &mut Vec<Agent>) {
    match value {
        Value::Array(items) => items.iter().for_each(|v| collect(v, agents)),
        Value::Object(map) => {
            let status = map.get("agent_status").or_else(|| map.get("status"));
            if let Some(status) = status.and_then(Value::as_str) {
                let name = ["name", "agent", "pane_id"]
                    .iter()
                    .find_map(|k| map.get(*k).and_then(Value::as_str))
                    .unwrap_or("agent");
                agents.push(Agent {
                    name: name.to_owned(),
                    status: status.to_owned(),
                });
            } else {
                map.values().for_each(|v| collect(v, agents));
            }
        }
        _ => {}
    }
}

/// Opens session `name` in `terminal`, starting it in `folder` when it is not running:
/// `herdr --session <name>` launches a session or attaches to it.
pub fn open(shell: &Shell, terminal: Terminal, name: &str, folder: &str) -> Result<(), String> {
    if !crate::model::is_session_name(name) {
        return Err(format!("“{name}” is not a herdr session name"));
    }
    let folder = expand(folder);
    let script = format!("cd {} && exec herdr --session {name}", quote(&folder));
    match terminal {
        Terminal::Ghostty => {
            let command = format!("--command=/bin/zsh -ilc {}", quote(&script));
            shell.run("open", &["-na", "Ghostty", "--args", &command], None)?;
        }
        Terminal::Terminal => {
            let apple = format!(
                "tell application \"Terminal\"\n  activate\n  do script {}\nend tell",
                apple_string(&script)
            );
            shell.run("osascript", &["-e", &apple], None)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stopped_server_is_not_running() {
        let v =
            serde_json::json!({"id": "cli:agent:list", "error": {"code": "server_not_running"}});
        assert_eq!(read(&v), Session::default());
    }

    #[test]
    fn finds_the_agents_and_their_states() {
        let v = serde_json::json!({"id": "x", "result": {"type": "agent_list", "agents": [
            {"pane_id": "w1:p1", "agent": "claude", "agent_status": "blocked"},
            {"pane_id": "w1:p2", "name": "reviewer", "agent": "codex", "agent_status": "working"}
        ]}});
        let session = read(&v);
        assert!(session.running);
        assert_eq!(session.agents[0].status, "blocked");
        let names: Vec<&str> = session.agents.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["claude", "reviewer"]);
    }
}
