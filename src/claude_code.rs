//! Adds ZapFast's MCP server (`zapfast mcp`, see [`crate::mcp`]) to Claude
//! Code, so any Claude Code session on this computer reaches the running
//! ZapFast through agent access.
//!
//! Claude Code keeps its configuration to itself and rewrites it while it
//! runs, so ZapFast asks its command line, `claude mcp add --scope user`,
//! rather than edit the file.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The name the server goes by in Claude Code.
pub const SERVER_NAME: &str = "zapfast";

/// How long each `claude` command gets before ZapFast gives up on it.
const STEP_TIME: Duration = Duration::from_secs(60);

/// Where installing stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Idle,
    Installing,
    Installed,
    /// No `claude` that ZapFast can run: not installed, or out of a
    /// Flatpak's reach.
    NotFound,
    /// Claude Code refused, or could not be run, with what it said.
    Failed(String),
}

/// Installs away from the interface and keeps the outcome for it to show.
#[derive(Default)]
pub struct Installer {
    status: Arc<Mutex<Status>>,
}

impl Installer {
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Starts installing unless it is already under way. `wake` runs once
    /// the outcome is in.
    pub fn start(&self, wake: impl FnOnce() + Send + 'static) {
        {
            let mut status = self
                .status
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if *status == Status::Installing {
                return;
            }
            *status = Status::Installing;
        }
        let status = Arc::clone(&self.status);
        let spawned = std::thread::Builder::new()
            .name("claude-code".into())
            .spawn({
                let status = Arc::clone(&status);
                move || {
                    let outcome = install(&server_command());
                    *status
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) = outcome;
                    wake();
                }
            });
        if let Err(error) = spawned {
            *status
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                Status::Failed(error.to_string());
        }
    }
}

/// The command Claude Code runs to start the server: this ZapFast with `mcp`.
pub fn server_command() -> Vec<String> {
    command_for(
        std::env::var("FLATPAK_ID").ok().as_deref(),
        std::env::var_os("APPIMAGE").map(PathBuf::from).as_deref(),
        std::env::current_exe().ok().as_deref(),
    )
}

/// An AppImage runs from a mount that changes each time, so its own file is
/// the one to start; a Flatpak starts through `flatpak run`.
fn command_for(flatpak: Option<&str>, appimage: Option<&Path>, exe: Option<&Path>) -> Vec<String> {
    if let Some(id) = flatpak.filter(|id| !id.is_empty()) {
        return ["flatpak", "run", id, "mcp"].map(String::from).to_vec();
    }
    let program = appimage
        .or(exe)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| "zapfast".to_owned());
    vec![program, "mcp".to_owned()]
}

/// `claude`'s arguments to add `server` for every project.
fn add_arguments(server: &[String]) -> Vec<String> {
    ["mcp", "add", "--scope", "user", SERVER_NAME, "--"]
        .map(String::from)
        .into_iter()
        .chain(server.iter().cloned())
        .collect()
}

/// What to run in a terminal to add the server by hand.
pub fn manual_command() -> String {
    let words: Vec<String> = std::iter::once("claude".to_owned())
        .chain(add_arguments(&server_command()))
        .collect();
    words
        .iter()
        .map(|word| quoted(word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `word` as a shell reads it back.
fn quoted(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=@+,".contains(c));
    if plain {
        word.to_owned()
    } else if cfg!(windows) {
        format!("\"{}\"", word.replace('"', "\\\""))
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

/// Adds the server, replacing one of the same name, so a ZapFast that has
/// moved (from an AppImage to the package, say) is found again.
fn install(server: &[String]) -> Status {
    match find_claude() {
        Some(claude) => install_with(&claude, server),
        None => Status::NotFound,
    }
}

fn install_with(claude: &Path, server: &[String]) -> Status {
    // The first time there is nothing to remove, which Claude Code reports
    // as a failure.
    let _ = run(claude, &["mcp", "remove", "--scope", "user", SERVER_NAME]);
    match run(claude, &add_arguments(server)) {
        Ok(ran) if ran.success => Status::Installed,
        Ok(ran) => {
            log::warn!("Claude Code did not add the MCP server");
            Status::Failed(ran.said)
        }
        Err(error) => {
            log::warn!("could not run Claude Code: {error}");
            Status::Failed(error.to_string())
        }
    }
}

/// Where `claude` is: on the PATH, or where its installers put it, since a
/// program started from the desktop menu may not have the shell's PATH.
/// Inside a Flatpak the host's programs are out of reach.
fn find_claude() -> Option<PathBuf> {
    if std::env::var_os("FLATPAK_ID").is_some() {
        return None;
    }
    let dirs = directories::BaseDirs::new();
    let mut extra = Vec::new();
    if let Some(dirs) = &dirs {
        // npm's global folder on Windows.
        if cfg!(windows) {
            extra.push(dirs.data_dir().join("npm"));
        }
    }
    candidates(
        std::env::var_os("PATH").as_deref(),
        dirs.as_ref().map(directories::BaseDirs::home_dir),
        extra,
    )
    .into_iter()
    .find(|candidate| candidate.is_file())
}

fn candidates(path: Option<&OsStr>, home: Option<&Path>, extra: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = path
        .map(|path| std::env::split_paths(path).collect())
        .unwrap_or_default();
    if let Some(home) = home {
        // The native installer, the old local install, npm's and bun's
        // per-user folders, and every Node version nvm keeps.
        for folder in [
            ".local/bin",
            ".claude/local",
            ".npm-global/bin",
            ".bun/bin",
            ".volta/bin",
        ] {
            folders.push(home.join(folder));
        }
        if let Ok(versions) = std::fs::read_dir(home.join(".nvm/versions/node")) {
            let mut versions: Vec<PathBuf> = versions
                .flatten()
                .map(|version| version.path().join("bin"))
                .collect();
            versions.sort();
            folders.extend(versions.into_iter().rev());
        }
    }
    folders.extend(extra);
    if cfg!(unix) {
        folders.extend(["/usr/local/bin", "/opt/homebrew/bin", "/usr/bin"].map(PathBuf::from));
    }
    let names: &[&str] = if cfg!(windows) {
        &["claude.exe", "claude.cmd"]
    } else {
        &["claude"]
    };
    folders
        .iter()
        .flat_map(|folder| names.iter().map(move |name| folder.join(name)))
        .collect()
}

struct Ran {
    success: bool,
    /// What the command printed, errors first, on one line.
    said: String,
}

fn run(claude: &Path, arguments: &[impl AsRef<OsStr>]) -> std::io::Result<Ran> {
    let mut command = Command::new(claude);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Started from a Claude Code terminal, ZapFast is no session of it.
        .env_remove("CLAUDECODE");
    // An npm install is a Node script: the node it runs with sits beside it.
    if let Some(folder) = claude.parent() {
        let rest = std::env::var_os("PATH").unwrap_or_default();
        let folders = std::iter::once(folder.to_path_buf()).chain(std::env::split_paths(&rest));
        if let Ok(path) = std::env::join_paths(folders) {
            command.env("PATH", path);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console flashes up.
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn()?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() > STEP_TIME {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Claude Code did not answer",
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut errors = String::new();
    let mut output = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_string(&mut errors);
    }
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut output);
    }
    let said = [errors, output]
        .iter()
        .flat_map(|text| text.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Ran {
        success: status.success(),
        said: said.chars().take(300).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_code_starts_this_zapfast_wherever_it_came_from() {
        let exe = Path::new("/usr/bin/zapfast");
        assert_eq!(
            command_for(None, None, Some(exe)),
            ["/usr/bin/zapfast", "mcp"]
        );
        let image = Path::new("/home/me/Apps/ZapFast.AppImage");
        assert_eq!(
            command_for(None, Some(image), Some(Path::new("/tmp/.mount_x/zapfast"))),
            ["/home/me/Apps/ZapFast.AppImage", "mcp"]
        );
        assert_eq!(
            command_for(Some("rocks.zapfast.ZapFast"), None, Some(exe)),
            ["flatpak", "run", "rocks.zapfast.ZapFast", "mcp"]
        );
        assert_eq!(command_for(Some(""), None, None), ["zapfast", "mcp"]);
    }

    #[test]
    fn the_server_is_added_for_every_project() {
        let server = ["/opt/Zap Fast/zapfast".to_owned(), "mcp".to_owned()];
        assert_eq!(
            add_arguments(&server),
            [
                "mcp",
                "add",
                "--scope",
                "user",
                "zapfast",
                "--",
                "/opt/Zap Fast/zapfast",
                "mcp"
            ]
        );
        assert_eq!(quoted("/usr/bin/zapfast"), "/usr/bin/zapfast");
        if cfg!(unix) {
            assert_eq!(quoted("/opt/Zap Fast/zapfast"), "'/opt/Zap Fast/zapfast'");
            assert_eq!(quoted("it's"), r"'it'\''s'");
        }
        assert!(manual_command().starts_with("claude mcp add --scope user zapfast -- "));
    }

    #[test]
    fn claude_is_looked_for_where_its_installers_put_it() {
        let home = tempfile::tempdir().unwrap();
        for version in ["v20.1.0", "v22.3.0"] {
            std::fs::create_dir_all(
                home.path()
                    .join(".nvm/versions/node")
                    .join(version)
                    .join("bin"),
            )
            .unwrap();
        }
        let path = std::env::join_paths(["/somewhere/bin"]).unwrap();
        let found = candidates(Some(&path), Some(home.path()), Vec::new());
        let name = if cfg!(windows) {
            "claude.exe"
        } else {
            "claude"
        };
        assert_eq!(found[0], Path::new("/somewhere/bin").join(name));
        let position = |folder: PathBuf| {
            found
                .iter()
                .position(|candidate| candidate == &folder.join(name))
                .unwrap()
        };
        let native = position(home.path().join(".local/bin"));
        let newer = position(home.path().join(".nvm/versions/node/v22.3.0/bin"));
        let older = position(home.path().join(".nvm/versions/node/v20.1.0/bin"));
        assert!(native < newer && newer < older);
    }

    /// A stand-in for `claude` that notes its arguments and answers as
    /// `body` says.
    #[cfg(unix)]
    fn fake_claude(folder: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let claude = folder.join("claude");
        let calls = folder.join("calls");
        std::fs::write(
            &claude,
            format!("#!/bin/sh\necho \"$@\" >> '{}'\n{body}\n", calls.display()),
        )
        .unwrap();
        std::fs::set_permissions(&claude, std::fs::Permissions::from_mode(0o755)).unwrap();
        claude
    }

    #[cfg(unix)]
    #[test]
    fn installing_replaces_any_earlier_server_of_the_same_name() {
        let folder = tempfile::tempdir().unwrap();
        let claude = fake_claude(
            folder.path(),
            "if [ \"$2\" = remove ]; then echo 'No MCP server found' >&2; exit 1; fi\n\
             echo 'Added stdio MCP server zapfast to user config'",
        );
        let server = ["/usr/bin/zapfast".to_owned(), "mcp".to_owned()];
        assert_eq!(install_with(&claude, &server), Status::Installed);
        assert_eq!(
            std::fs::read_to_string(folder.path().join("calls")).unwrap(),
            "mcp remove --scope user zapfast\n\
             mcp add --scope user zapfast -- /usr/bin/zapfast mcp\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_refusal_comes_back_with_what_claude_code_said() {
        let folder = tempfile::tempdir().unwrap();
        let claude = fake_claude(
            folder.path(),
            "echo 'Error: Invalid scope' >&2\necho '  try again  '\nexit 1",
        );
        let server = ["/usr/bin/zapfast".to_owned(), "mcp".to_owned()];
        assert_eq!(
            install_with(&claude, &server),
            Status::Failed("Error: Invalid scope try again".to_owned())
        );
        let missing = folder.path().join("nothing-here");
        assert!(matches!(install_with(&missing, &server), Status::Failed(_)));
    }
}
