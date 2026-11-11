//! Connecting to an instance.
//!
//! A [`ConnectSpec`] describes one connection: SSH with a key, SSH with an
//! Active Directory password, or an SSM session. It is used in two ways:
//!
//! * same terminal - the TUI is suspended and [`run`] executes the command;
//! * new window - the spec is turned into command line tokens
//!   ([`ConnectSpec::to_tokens`]) and the terminal emulator is asked to run
//!   `e2s __connect <tokens>`. [`trampoline`] rebuilds the spec inside the new
//!   window and calls the very same [`run`].
//!
//! Terminal emulators disagree on how they pass a command along: some keep the
//! argument list intact, some join it with spaces, some hand the joined text
//! to a shell. A command such as `ssh -i key user@host; exec zsh` only
//! survives the first kind - the others end up running a bare `ssh`, which
//! prints its usage text. The tokens produced here contain nothing but
//! `[A-Za-z0-9._/:=%-]`, so they arrive unchanged whichever way the emulator
//! (or `wsl.exe`, or AppleScript) treats them.

use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

pub mod askpass;
pub mod launcher;

/// First command line argument of the hidden "run this connection" mode.
pub const SUBCOMMAND: &str = "__connect";

/// Environment variables copied into a new terminal window, which does not
/// always inherit the environment of the TUI (WSL, terminal servers, ...).
/// Never secrets: the tokens end up on a command line.
const FORWARDED_ENV: [&str; 6] = [
    "AWS_PROFILE",
    "AWS_REGION",
    "AWS_DEFAULT_REGION",
    "AWS_CONFIG_FILE",
    "AWS_SHARED_CREDENTIALS_FILE",
    "SSH_AUTH_SOCK",
];

/// How `ssh` authenticates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshAuth {
    /// `ssh -i <key> user@host`
    Key(String),
    /// Password login (Active Directory account). `autofill` is set when a
    /// password is stored in the config file and should be typed for the user.
    Password { autofill: bool },
}

/// What to connect to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Ssh {
        host: String,
        user: String,
        auth: SshAuth,
    },
    Ssm {
        instance_id: String,
        region: Option<String>,
        profile: Option<String>,
    },
}

/// One connection request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectSpec {
    pub target: Target,
    /// Config file in use, so the new window (and the password helper) read
    /// the same file as the TUI.
    pub config_path: Option<String>,
    /// New window only: shell to drop into once the session has ended, so the
    /// window stays open and errors remain readable.
    pub shell: Option<String>,
    /// New window only: wait for Enter before the window closes (used where
    /// there is no shell to drop into).
    pub pause: bool,
    /// New window only: environment to restore, see [`FORWARDED_ENV`].
    pub env: Vec<(String, String)>,
}

impl ConnectSpec {
    pub fn new(target: Target) -> Self {
        Self {
            target,
            config_path: None,
            shell: None,
            pause: false,
            env: Vec::new(),
        }
    }

    /// Serialises the spec into arguments for `e2s __connect`.
    ///
    /// Every token is a single `--name=value` word made only of characters
    /// that no shell, terminal emulator or Windows command line treats
    /// specially (see [`is_safe_token`]).
    pub fn to_tokens(&self) -> Vec<String> {
        let mut tokens = Vec::new();
        match &self.target {
            Target::Ssh { host, user, auth } => {
                tokens.push(token("ssh", host));
                tokens.push(token("user", user));
                match auth {
                    SshAuth::Key(path) => tokens.push(token("key", path)),
                    SshAuth::Password { autofill } => {
                        tokens.push(token("password", if *autofill { "saved" } else { "ask" }))
                    }
                }
            }
            Target::Ssm {
                instance_id,
                region,
                profile,
            } => {
                tokens.push(token("ssm", instance_id));
                if let Some(region) = region {
                    tokens.push(token("region", region));
                }
                if let Some(profile) = profile {
                    tokens.push(token("profile", profile));
                }
            }
        }
        if let Some(path) = &self.config_path {
            tokens.push(token("config", path));
        }
        if let Some(shell) = &self.shell {
            tokens.push(token("shell", shell));
        }
        for (name, value) in &self.env {
            tokens.push(token("env", &format!("{name}={value}")));
        }
        if self.pause {
            tokens.push("--pause".to_string());
        }
        tokens
    }

    /// Inverse of [`ConnectSpec::to_tokens`].
    pub fn from_tokens<I, S>(tokens: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut host = None;
        let mut user = None;
        let mut key = None;
        let mut password = None;
        let mut instance_id = None;
        let mut region = None;
        let mut profile = None;
        let mut config_path = None;
        let mut shell = None;
        let mut pause = false;
        let mut env = Vec::new();

        for token in tokens {
            let token = token.as_ref();
            let unknown = || format!("unexpected argument `{token}`");
            let body = token.strip_prefix("--").ok_or_else(unknown)?;
            let Some((name, raw)) = body.split_once('=') else {
                match body {
                    "pause" => pause = true,
                    _ => return Err(unknown()),
                }
                continue;
            };
            let value = decode(raw).map_err(|err| format!("`{token}`: {err}"))?;
            if value.is_empty() {
                return Err(format!("`--{name}` has no value"));
            }
            match name {
                "ssh" => host = Some(value),
                "user" => user = Some(value),
                "key" => key = Some(value),
                "password" => password = Some(value),
                "ssm" => instance_id = Some(value),
                "region" => region = Some(value),
                "profile" => profile = Some(value),
                "config" => config_path = Some(value),
                "shell" => shell = Some(value),
                "env" => {
                    let (name, value) = value
                        .split_once('=')
                        .ok_or_else(|| format!("`{token}`: expected NAME=VALUE"))?;
                    env.push((name.to_string(), value.to_string()));
                }
                _ => return Err(unknown()),
            }
        }

        let target = match (host, instance_id) {
            (Some(host), None) => {
                let user = user.ok_or("`--user` is missing")?;
                let auth = match (key, password.as_deref()) {
                    (Some(path), None) => SshAuth::Key(path),
                    (None, Some("saved")) => SshAuth::Password { autofill: true },
                    (None, Some("ask")) => SshAuth::Password { autofill: false },
                    _ => return Err("expected exactly one of `--key` or `--password`".into()),
                };
                Target::Ssh { host, user, auth }
            }
            (None, Some(instance_id)) => Target::Ssm {
                instance_id,
                region,
                profile,
            },
            _ => return Err("expected exactly one of `--ssh` or `--ssm`".into()),
        };

        Ok(Self {
            target,
            config_path,
            shell,
            pause,
            env,
        })
    }

    /// Builds the process to run. Nothing is spawned yet.
    pub fn prepare(&self) -> io::Result<Prepared> {
        let mut ticket = None;
        let command = match &self.target {
            Target::Ssh { host, user, auth } => {
                // A value that starts with `-` would be read by ssh as an
                // option. `-` alone is what the instance list shows when an
                // instance has no such address.
                if host.is_empty() || host.starts_with('-') {
                    return Err(invalid(
                        "the selected instance has no address for this mode",
                    ));
                }
                if user.is_empty() || user.starts_with('-') {
                    return Err(invalid("no valid SSH user selected"));
                }
                let mut ssh = Command::new("ssh");
                match auth {
                    SshAuth::Key(path) => {
                        ssh.arg("-i").arg(path);
                        ssh.arg(format!("{user}@{host}"));
                    }
                    SshAuth::Password { autofill } => {
                        // Go straight to the password: offering every key of
                        // the agent first can exhaust the server's
                        // MaxAuthTries before the password is even asked for.
                        ssh.args([
                            "-o",
                            "PreferredAuthentications=keyboard-interactive,password",
                        ]);
                        // `-l` keeps logins such as `name@domain` intact.
                        ssh.arg("-l").arg(user);
                        ssh.arg(host);
                        if *autofill {
                            ticket = askpass::arm(&mut ssh, &format!("{user}@{host}"));
                        }
                    }
                }
                ssh
            }
            Target::Ssm {
                instance_id,
                region,
                profile,
            } => {
                let mut aws = Command::new("aws");
                aws.args(["ssm", "start-session", "--target", instance_id]);
                if let Some(region) = region {
                    aws.args(["--region", region]);
                }
                if let Some(profile) = profile {
                    aws.args(["--profile", profile]);
                }
                aws
            }
        };
        Ok(Prepared { command, ticket })
    }

    /// Restores, inside a new window, the environment carried by the spec.
    fn apply_env(&self) {
        if let Some(path) = &self.config_path {
            env::set_var("E2S_CONFIG", path);
        }
        for (name, value) in &self.env {
            if name == "PATH" {
                env::set_var("PATH", merge_path(value, env::var_os("PATH")));
            } else {
                env::set_var(name, value);
            }
        }
    }

    /// What happens in a new window once the session is over.
    fn finish(&self, code: i32) -> i32 {
        #[cfg(unix)]
        if let Some(shell) = &self.shell {
            use std::os::unix::process::CommandExt;
            // Replaces this process; only returns on failure.
            let err = Command::new(shell).exec();
            eprintln!("e2s: could not start {shell}: {err}");
            pause_for_enter();
            return code;
        }
        if self.pause {
            pause_for_enter();
        }
        code
    }
}

/// A command ready to run, plus what has to be tidied up afterwards.
pub struct Prepared {
    pub command: Command,
    /// One-shot permission for the password helper, see [`askpass`].
    ticket: Option<PathBuf>,
}

impl Prepared {
    /// Runs the command in the current terminal and waits for it.
    pub fn run(mut self) -> io::Result<ExitStatus> {
        let status = self.command.status().map_err(|err| {
            let program = self.command.get_program().to_string_lossy().into_owned();
            io::Error::new(err.kind(), format!("could not start `{program}`: {err}"))
        });
        if let Some(ticket) = &self.ticket {
            // Still there when ssh never asked for the password.
            let _ = std::fs::remove_file(ticket);
        }
        status
    }
}

/// The running e2s binary, which ssh and new terminal windows start again.
/// Fails if it was replaced or removed since e2s started (an update,
/// `cargo install`): it could not be started then.
pub fn current_exe() -> io::Result<PathBuf> {
    let exe = env::current_exe()?;
    if exe.is_file() {
        Ok(exe)
    } else {
        Err(io::Error::other(
            "the e2s program file was replaced while e2s is running - restart e2s",
        ))
    }
}

/// Runs the connection in the current terminal and waits for it to end.
pub fn run(spec: &ConnectSpec) -> io::Result<ExitStatus> {
    spec.prepare()?.run()
}

/// Entry point of `e2s __connect <tokens>`: what a new terminal window runs.
/// Returns the process exit code.
pub fn trampoline<I, S>(tokens: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let spec = match ConnectSpec::from_tokens(tokens) {
        Ok(spec) => spec,
        Err(err) => {
            eprintln!("e2s: cannot open the session: {err}");
            pause_for_enter();
            return 2;
        }
    };
    spec.apply_env();
    let code = match run(&spec) {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!("e2s: {err}");
            1
        }
    };
    spec.finish(code)
}

/// Environment worth carrying into a new window. `skip_windows_mounts` drops
/// the Windows directories WSL appends to `PATH`: they are added again on the
/// other side and would only make the command line longer.
pub fn forwarded_env(skip_windows_mounts: bool) -> Vec<(String, String)> {
    let mut vars = Vec::new();
    // On Windows the new window gets the right PATH by itself.
    if cfg!(unix) {
        if let Some(path) = env::var_os("PATH") {
            let mut dirs: Vec<PathBuf> = Vec::new();
            for dir in env::split_paths(&path) {
                let skip = dir.as_os_str().is_empty()
                    || (skip_windows_mounts && dir.starts_with("/mnt"))
                    || dirs.contains(&dir);
                if !skip {
                    dirs.push(dir);
                }
            }
            let joined = env::join_paths(dirs)
                .ok()
                .and_then(|p| p.into_string().ok());
            if let Some(joined) = joined.filter(|p| !p.is_empty()) {
                vars.push(("PATH".to_string(), joined));
            }
        }
    }
    for name in FORWARDED_ENV {
        if let Ok(value) = env::var(name) {
            if !value.is_empty() {
                vars.push((name.to_string(), value));
            }
        }
    }
    vars
}

/// `forwarded` first, then whatever else the current `PATH` has.
fn merge_path(forwarded: &str, current: Option<OsString>) -> OsString {
    let mut dirs: Vec<PathBuf> = env::split_paths(forwarded).collect();
    if let Some(current) = current {
        for dir in env::split_paths(&current) {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    env::join_paths(dirs).unwrap_or_else(|_| OsString::from(forwarded))
}

fn pause_for_enter() {
    eprint!("\n[e2s] Press Enter to close this window... ");
    let _ = io::stderr().flush();
    let _ = io::stdin().read_line(&mut String::new());
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn token(name: &str, value: &str) -> String {
    format!("--{name}={}", encode(value))
}

/// True if `text` is made only of characters that survive every way a
/// terminal emulator may pass a command along (argument list, joined with
/// spaces, re-read by a shell, typed through AppleScript, Windows command
/// line).
pub fn is_safe_token(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| is_safe_byte(b) || b == b'=' || b == b'%')
}

fn is_safe_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'/' | b':' | b'-')
}

/// Percent-encodes everything outside `[A-Za-z0-9._/:-]`.
pub fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for &byte in value.as_bytes() {
        if is_safe_byte(byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Inverse of [`encode`].
pub fn decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes
                .get(i + 1..i + 3)
                .filter(|pair| pair.iter().all(u8::is_ascii_hexdigit))
                .ok_or("bad percent-encoding")?;
            // Two ASCII hex digits: always valid UTF-8 and always parse.
            let hex = std::str::from_utf8(hex).map_err(|_| "bad percent-encoding")?;
            out.push(u8::from_str_radix(hex, 16).map_err(|_| "bad percent-encoding")?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "not valid UTF-8".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn args(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    fn envs(command: &Command) -> Vec<String> {
        command
            .get_envs()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect()
    }

    fn key_spec() -> ConnectSpec {
        ConnectSpec::new(Target::Ssh {
            host: "10.0.1.11".into(),
            user: "ubuntu".into(),
            auth: SshAuth::Key("/home/me/.ssh/dev key (old).pem".into()),
        })
    }

    fn ad_spec(autofill: bool) -> ConnectSpec {
        ConnectSpec::new(Target::Ssh {
            host: "10.0.1.11".into(),
            user: "jane.doe@corp.example.local".into(),
            auth: SshAuth::Password { autofill },
        })
    }

    #[test]
    fn encode_leaves_plain_text_alone_and_escapes_the_rest() {
        assert_eq!(
            encode("/home/me/.ssh/id_ed25519"),
            "/home/me/.ssh/id_ed25519"
        );
        assert_eq!(encode("a b;c"), "a%20b%3Bc");
        assert_eq!(encode("jane@corp"), "jane%40corp");
        assert_eq!(encode(r"CORP\jane"), "CORP%5Cjane");
        assert_eq!(encode("100%"), "100%25");
        assert_eq!(encode("é"), "%C3%A9");
    }

    #[test]
    fn decode_reverses_encode() {
        for text in [
            "",
            "plain",
            "a b;c",
            "quote ' double \" back ` dollar $ amp & pipe |",
            r"C:\Users\Jane Doe\.ssh\key.pem",
            "tab\there\nnewline",
            "%41 literal percent",
            "ünïcödé 日本語 🔑",
            "--looks=like-a-flag",
        ] {
            let encoded = encode(text);
            assert!(encoded.is_empty() || is_safe_token(&encoded), "{encoded}");
            assert_eq!(decode(&encoded).as_deref(), Ok(text));
        }
    }

    #[test]
    fn decode_rejects_malformed_input() {
        assert!(decode("%").is_err());
        assert!(decode("%4").is_err());
        assert!(decode("%zz").is_err());
        assert!(decode("%+1").is_err());
        assert!(decode("%C3").is_err()); // lone UTF-8 lead byte
    }

    #[test]
    fn tokens_round_trip() {
        let mut ssm = ConnectSpec::new(Target::Ssm {
            instance_id: "i-0123456789abcdef0".into(),
            region: Some("eu-west-1".into()),
            profile: Some("my profile".into()),
        });
        ssm.config_path = Some("/home/me/My Configs/e2s.toml".into());
        ssm.shell = Some("/usr/bin/zsh".into());
        ssm.pause = true;
        ssm.env = vec![
            ("PATH".into(), "/home/me/.local/bin:/usr/bin".into()),
            ("AWS_PROFILE".into(), "a=b c".into()),
        ];

        for spec in [key_spec(), ad_spec(true), ad_spec(false), ssm] {
            let tokens = spec.to_tokens();
            assert_eq!(ConnectSpec::from_tokens(&tokens), Ok(spec));
        }
    }

    /// The property the whole design rests on: whatever the user's key path,
    /// login or profile looks like, no token needs quoting anywhere.
    #[test]
    fn tokens_never_need_quoting() {
        let mut spec = ConnectSpec::new(Target::Ssh {
            host: "10.0.1.11".into(),
            user: r#"CORP\jane doe;rm -rf $HOME "x" 'y' `z`"#.into(),
            auth: SshAuth::Key(r"C:\Users\Jane Doe\.ssh\key (1).pem".into()),
        });
        spec.config_path = Some("~/my configs/e2s.toml".into());
        spec.shell = Some("/opt/my shells/zsh".into());
        spec.env = vec![("AWS_PROFILE".into(), "prod & co".into())];
        spec.pause = true;

        let tokens = spec.to_tokens();
        for token in &tokens {
            assert!(is_safe_token(token), "unsafe token: {token}");
        }
        // Joined with spaces and split again - what the worst launchers do.
        let flattened = tokens.join(" ");
        let reparsed = ConnectSpec::from_tokens(flattened.split(' '));
        assert_eq!(reparsed, Ok(spec));
    }

    #[test]
    fn from_tokens_rejects_incomplete_or_unknown_input() {
        let parse = |tokens: &[&str]| ConnectSpec::from_tokens(tokens);
        assert!(parse(&[]).is_err());
        assert!(parse(&["--ssh=10.0.0.1"]).is_err());
        assert!(parse(&["--ssh=10.0.0.1", "--user=ubuntu"]).is_err());
        assert!(parse(&["--ssh=10.0.0.1", "--user=u", "--key=k", "--password=ask"]).is_err());
        assert!(parse(&["--ssh=10.0.0.1", "--user=u", "--password=nope"]).is_err());
        assert!(parse(&["--ssh=10.0.0.1", "--ssm=i-1", "--user=u", "--key=k"]).is_err());
        assert!(parse(&["--ssm=i-1", "--bogus=1"]).is_err());
        assert!(parse(&["--ssm=i-1", "stray"]).is_err());
        assert!(parse(&["--ssm="]).is_err());
        assert!(parse(&["--ssm=i-1", "--env=NOEQUALS"]).is_err());
    }

    #[test]
    fn key_login_runs_plain_ssh() {
        let prepared = key_spec().prepare().unwrap();
        assert_eq!(prepared.command.get_program(), "ssh");
        assert_eq!(
            args(&prepared.command),
            ["-i", "/home/me/.ssh/dev key (old).pem", "ubuntu@10.0.1.11"]
        );
        assert!(envs(&prepared.command).is_empty());
        assert!(prepared.ticket.is_none());
    }

    #[test]
    fn ad_login_uses_dash_l_and_password_auth() {
        let prepared = ad_spec(false).prepare().unwrap();
        assert_eq!(
            args(&prepared.command),
            [
                "-o",
                "PreferredAuthentications=keyboard-interactive,password",
                "-l",
                "jane.doe@corp.example.local",
                "10.0.1.11"
            ]
        );
        // No stored password: ssh asks on the terminal as usual.
        assert!(envs(&prepared.command).is_empty());
        assert!(prepared.ticket.is_none());
    }

    #[test]
    fn ad_login_with_saved_password_keeps_the_password_out_of_the_command() {
        let prepared = ad_spec(true).prepare().unwrap();
        // Same ssh command with or without a saved password...
        assert_eq!(
            args(&prepared.command),
            args(&ad_spec(false).prepare().unwrap().command)
        );
        // ...the helper is wired up through the environment only (not at all
        // while a refused password is on hold, see `askpass`).
        let names = envs(&prepared.command);
        assert_eq!(
            prepared.ticket.is_some(),
            names.iter().any(|n| n == "SSH_ASKPASS")
        );
        assert!(!format!("{:?}", prepared.command)
            .to_lowercase()
            .contains("password="));
        if let Some(ticket) = &prepared.ticket {
            std::fs::remove_file(ticket).unwrap();
        }
    }

    #[test]
    fn ssm_session_passes_region_and_profile() {
        let spec = ConnectSpec::new(Target::Ssm {
            instance_id: "i-0abc".into(),
            region: Some("us-east-1".into()),
            profile: None,
        });
        let prepared = spec.prepare().unwrap();
        assert_eq!(prepared.command.get_program(), "aws");
        assert_eq!(
            args(&prepared.command),
            [
                "ssm",
                "start-session",
                "--target",
                "i-0abc",
                "--region",
                "us-east-1"
            ]
        );
    }

    #[test]
    fn missing_address_is_refused_instead_of_reaching_ssh() {
        for host in ["-", "", "-oProxyCommand=evil"] {
            let spec = ConnectSpec::new(Target::Ssh {
                host: host.into(),
                user: "ubuntu".into(),
                auth: SshAuth::Key("/k".into()),
            });
            assert!(spec.prepare().is_err(), "{host:?}");
        }
    }

    #[test]
    fn forwarded_path_goes_first_without_duplicates() {
        let merged = merge_path(
            "/home/me/.local/bin:/usr/bin",
            Some(OsString::from("/usr/bin:/bin:/mnt/c/Windows")),
        );
        assert_eq!(merged, "/home/me/.local/bin:/usr/bin:/bin:/mnt/c/Windows");
    }
}
