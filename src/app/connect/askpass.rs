//! Typing the saved Active Directory password for the user.
//!
//! OpenSSH does not take a password from its arguments or its environment.
//! What it offers is `SSH_ASKPASS`: a program it runs whenever it needs an
//! answer, with the question as the first argument, reading the answer from
//! that program's standard output. With `SSH_ASKPASS_REQUIRE=force`
//! (OpenSSH 8.4 and later) it does so even when a terminal is available.
//!
//! e2s points `SSH_ASKPASS` at itself ([`arm`]). When ssh then runs e2s with
//! [`ENV_MODE`] set, [`run`] answers:
//!
//! * the password question for the login e2s asked for - once - with the
//!   password from the config file;
//! * everything else (unknown host key, one-time codes, an expired password,
//!   a jump host's password, the password again after it was refused) by
//!   passing the question on to the person at the terminal.
//!
//! Sending a wrong password repeatedly locks Active Directory accounts, so
//! the saved password is sent sparingly:
//!
//! * once per connection - a ticket (an empty file created before ssh starts)
//!   is deleted when the password is handed out;
//! * not at all for a while after it was refused ([`refused_recently`]),
//!   until the config file is edited.
//!
//! The password is never placed on a command line or in the environment; the
//! helper reads it from the config file when it is asked for it.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, UNIX_EPOCH};

use rand::{distributions::Alphanumeric, Rng};

use crate::app::input::ad;
use crate::utils::config::get_config_path;

/// Set (to `1`) in the environment of ssh so that e2s, when started by ssh as
/// its askpass program, knows that is what it is being used for.
pub const ENV_MODE: &str = "E2S_ASKPASS";
/// Path of the one-shot ticket file.
const ENV_TICKET: &str = "E2S_ASKPASS_TICKET";
/// `user@host` the saved password belongs to.
const ENV_LOGIN: &str = "E2S_ASKPASS_LOGIN";

/// How long a refused password is not sent again. Long enough for the usual
/// Active Directory lockout counters to reset in between.
const REFUSAL_PAUSE: Duration = Duration::from_secs(30 * 60);

#[cfg(unix)]
const TTY_IN: &str = "/dev/tty";
#[cfg(unix)]
const TTY_OUT: &str = "/dev/tty";
#[cfg(not(unix))]
const TTY_IN: &str = "CONIN$";
#[cfg(not(unix))]
const TTY_OUT: &str = "CONOUT$";

/// True when this process was started by ssh to answer a question.
pub fn requested() -> bool {
    env::var_os(ENV_MODE).is_some()
}

/// Makes `ssh` ask e2s instead of the terminal for the password of `login`
/// (`user@host`). Returns the ticket file so the caller can remove it if ssh
/// never asked.
///
/// If the helper cannot be used ssh is left alone and asks on the terminal.
pub fn arm(ssh: &mut Command, login: &str) -> Option<PathBuf> {
    arm_unless(refused_recently(), ssh, login)
}

fn arm_unless(paused: bool, ssh: &mut Command, login: &str) -> Option<PathBuf> {
    if paused {
        eprintln!(
            "e2s: the saved password was refused a short while ago and is not sent again, so \
             that the account does not get locked.\n     Type it below, and correct `password` \
             under [ad] in the e2s config file."
        );
        return None;
    }

    let prepared = super::current_exe().and_then(|exe| Ok((exe, create_ticket()?)));
    match prepared {
        Ok((exe, ticket)) => {
            ssh.env("SSH_ASKPASS", exe)
                .env("SSH_ASKPASS_REQUIRE", "force")
                .env(ENV_MODE, "1")
                .env(ENV_LOGIN, login)
                .env(ENV_TICKET, &ticket);
            Some(ticket)
        }
        Err(err) => {
            eprintln!("e2s: cannot fill in the saved password ({err}); ssh will ask for it.");
            None
        }
    }
}

/// Creates the ticket as a new, private, empty file with an unguessable name.
fn create_ticket() -> io::Result<PathBuf> {
    let dir = dirs::runtime_dir()
        .filter(|dir| dir.is_dir())
        .unwrap_or_else(env::temp_dir);
    let name: String = rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(24)
        .map(char::from)
        .collect();
    let path = dir.join(format!("e2s-askpass-{name}"));

    let mut options = OpenOptions::new();
    // `create_new` refuses to follow a link or reuse an existing file.
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(&path)?;
    Ok(path)
}

/// Uses up the ticket. True only the first time.
fn take_ticket() -> bool {
    env::var_os(ENV_TICKET).is_some_and(|path| fs::remove_file(path).is_ok())
}

/// Where a refusal of the saved password is remembered.
fn refusal_file() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("e2s").join("ad-password-refused"))
}

/// Changes whenever the config file is edited. Deliberately not derived from
/// the password itself.
fn config_stamp() -> Option<String> {
    let meta = fs::metadata(get_config_path()?).ok()?;
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(format!(
        "{}.{}.{}",
        modified.as_secs(),
        modified.subsec_nanos(),
        meta.len()
    ))
}

fn remember_refusal() {
    if let (Some(path), Some(stamp)) = (refusal_file(), config_stamp()) {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(path, stamp);
    }
}

/// True if the password currently in the config file was refused by a server
/// within the last [`REFUSAL_PAUSE`].
pub fn refused_recently() -> bool {
    let (Some(path), Some(stamp)) = (refusal_file(), config_stamp()) else {
        return false;
    };
    let recent = fs::metadata(&path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| at.elapsed().ok())
        .is_some_and(|age| age < REFUSAL_PAUSE);
    recent && fs::read_to_string(&path).is_ok_and(|saved| saved == stamp)
}

/// The kinds of question ssh asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prompt {
    /// The password of the login e2s is connecting with.
    LoginPassword,
    /// ssh's own yes/no question about an unknown host key.
    HostKey,
    /// Anything else: one-time code, new password, another host's password...
    Other,
}

/// Decides what ssh is asking. `login` is the `user@host` the saved password
/// belongs to. The saved password must only ever answer the plain "what is
/// your password" question for that login, so when in doubt this returns
/// [`Prompt::Other`] and the person at the terminal answers instead.
fn classify(prompt: &str, login: &str) -> Prompt {
    let text = prompt.trim();
    let lower = text.to_lowercase();

    // Questions from a server arrive as "(<user>@<host>) <question>"; ssh's
    // own questions never start that way. Only the latter may be answered
    // visibly: a server must not get a typed password echoed by imitating
    // the host key question.
    if !text.starts_with('(')
        && (lower.contains("(yes/no") || lower.starts_with("please type 'yes'"))
    {
        return Prompt::HostKey;
    }

    if login.is_empty() {
        return Prompt::Other;
    }

    // ssh's own question for the `password` method.
    if text == format!("{login}'s password:") {
        return Prompt::LoginPassword;
    }

    // keyboard-interactive. The server may put a notice before the question
    // itself, which is the last line.
    let Some(question) = text.strip_prefix(&format!("({login}) ")) else {
        return Prompt::Other;
    };
    let question = question.lines().last().unwrap_or_default().to_lowercase();
    let words: Vec<&str> = question
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    // Other questions that say "password": changing it, one-time passwords.
    const NOT_A_LOGIN: [&str; 21] = [
        "new",
        "retype",
        "reenter",
        "re",
        "again",
        "current",
        "old",
        "confirm",
        "repeat",
        "expired",
        "change",
        "changing",
        "one",
        "time",
        "otp",
        "token",
        "code",
        "passcode",
        "pin",
        "factor",
        "verification",
    ];
    if words.contains(&"password") && !words.iter().any(|word| NOT_A_LOGIN.contains(word)) {
        Prompt::LoginPassword
    } else {
        Prompt::Other
    }
}

/// Entry point when ssh runs e2s as its askpass program. Returns the exit
/// code; ssh treats anything but 0 as "no answer".
pub fn run() -> i32 {
    let ssh = Parent::current();
    let prompt = env::args_os()
        .nth(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .unwrap_or_default();

    let failure = match answer(&prompt) {
        Ok(answer) => {
            let mut stdout = io::stdout();
            match writeln!(stdout, "{answer}").and_then(|()| stdout.flush()) {
                Ok(()) => return 0,
                Err(err) => err,
            }
        }
        Err(err) => err,
    };

    // There is no answer (Ctrl-C, Ctrl-D, no terminal to ask on). ssh takes
    // that for an empty one: asked for a password it would send an empty
    // password and ask again. Stop it instead, which is what Ctrl-C at ssh's
    // own prompt does.
    let cancelled = matches!(
        failure.kind(),
        io::ErrorKind::Interrupted | io::ErrorKind::UnexpectedEof
    );
    if !cancelled {
        eprintln!("e2s: cannot ask on the terminal: {failure}");
    }
    ssh.stop();
    1
}

fn answer(prompt: &str) -> io::Result<String> {
    // ssh only wants something shown (e.g. "touch your security key").
    let hint = env::var("SSH_ASKPASS_PROMPT").unwrap_or_default();
    if hint == "none" {
        let _ = tty_print(&format!("{}\n", printable(prompt)));
        return Ok(String::new());
    }

    let login = env::var(ENV_LOGIN).unwrap_or_default();
    let kind = classify(prompt, &login);
    if kind == Prompt::LoginPassword {
        if take_ticket() {
            if let Some(password) = ad::stored_password() {
                return Ok(password);
            }
        } else {
            // Asked a second time: the saved password was refused.
            remember_refusal();
            tty_print(
                "e2s: the saved password was not accepted. Type it in, or press Ctrl-C to cancel.\n",
            )?;
        }
    }

    // What was typed before the question appeared is not an answer to it: a
    // second Enter when starting the connection must not turn down a host
    // key or be sent as an empty password.
    discard_typeahead();
    tty_print(&printable(prompt))?;
    if kind == Prompt::HostKey || hint == "confirm" {
        tty_read_line()
    } else {
        // Never echo what may be a secret.
        read_secret()
    }
}

/// The process that started this helper: ssh.
struct Parent {
    #[cfg(unix)]
    pid: u32,
}

impl Parent {
    fn current() -> Self {
        Self {
            #[cfg(unix)]
            pid: std::os::unix::process::parent_id(),
        }
    }

    /// Interrupts ssh, as Ctrl-C would.
    fn stop(&self) {
        // Only while ssh is still the parent: had it exited, this helper
        // would have been adopted by an unrelated process.
        #[cfg(unix)]
        if self.pid > 1 && std::os::unix::process::parent_id() == self.pid {
            // SAFETY: kill(2) on a process id that was just checked.
            unsafe {
                libc::kill(self.pid as libc::pid_t, libc::SIGINT);
            }
        }
    }
}

/// Reads a line from the terminal without showing it.
///
/// While the line is being read the terminal delivers Ctrl-C as a plain
/// character instead of interrupting ssh and this helper. The read then
/// fails with `Interrupted`, and [`run`] stops ssh.
fn read_secret() -> io::Result<String> {
    #[cfg(unix)]
    // SAFETY: only changes how this process reacts to SIGINT (ignore it, so
    // that the helper lives long enough to stop ssh); no handler is installed.
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
    }
    rpassword::read_password()
}

/// Throws away input typed before the question was shown.
fn discard_typeahead() {
    #[cfg(unix)]
    if let Ok(tty) = OpenOptions::new().read(true).open(TTY_IN) {
        use std::os::unix::io::AsRawFd;
        // SAFETY: tcflush(3) on a descriptor that is open for this call.
        unsafe {
            libc::tcflush(tty.as_raw_fd(), libc::TCIFLUSH);
        }
    }
}

/// The question as it is safe to show: a server chooses part of it and must
/// not be able to send control sequences to the terminal.
fn printable(prompt: &str) -> String {
    prompt
        .chars()
        .map(|c| {
            if c.is_control() && c != '\n' && c != '\t' {
                ' '
            } else {
                c
            }
        })
        .collect()
}

fn tty_print(text: &str) -> io::Result<()> {
    let mut tty = OpenOptions::new().write(true).open(TTY_OUT)?;
    tty.write_all(text.as_bytes())?;
    tty.flush()
}

fn tty_read_line() -> io::Result<String> {
    let tty = OpenOptions::new().read(true).open(TTY_IN)?;
    let mut line = String::new();
    // End of input (Ctrl-D) gives an empty answer, which ssh takes for "no".
    BufReader::new(tty).read_line(&mut line)?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOGIN: &str = "jane.doe@corp.example.local@10.0.1.11";

    #[test]
    fn recognises_the_login_password_question() {
        for prompt in [
            "jane.doe@corp.example.local@10.0.1.11's password: ",
            "(jane.doe@corp.example.local@10.0.1.11) Password: ",
            "(jane.doe@corp.example.local@10.0.1.11) Password:",
            "(jane.doe@corp.example.local@10.0.1.11) Password for jane.doe@CORP.EXAMPLE.LOCAL: ",
            "(jane.doe@corp.example.local@10.0.1.11) LDAP Password: ",
            // A notice in front of the question does not change what is asked.
            "(jane.doe@corp.example.local@10.0.1.11) Authorized use only. No one may share \
             credentials at any time.\nPassword: ",
            "(jane.doe@corp.example.local@10.0.1.11) Your password will expire in 3 days.\n\
             Password: ",
        ] {
            assert_eq!(classify(prompt, LOGIN), Prompt::LoginPassword, "{prompt:?}");
        }
    }

    #[test]
    fn words_inside_the_login_do_not_matter() {
        let login = "andrew.newman@old.corp.local@10.0.1.11";
        for prompt in [
            "andrew.newman@old.corp.local@10.0.1.11's password: ",
            "(andrew.newman@old.corp.local@10.0.1.11) Password: ",
        ] {
            assert_eq!(classify(prompt, login), Prompt::LoginPassword, "{prompt:?}");
        }
    }

    /// The saved password belongs to one login on one host. A jump host on
    /// the way, or any other account, has to be answered by the person.
    #[test]
    fn never_offers_the_saved_password_to_another_login_or_host() {
        for prompt in [
            "ec2-user@bastion.example.com's password: ",
            "(ec2-user@bastion.example.com) Password: ",
            "jane.doe@corp.example.local@10.0.9.9's password: ",
            "(jane.doe@corp.example.local@10.0.9.9) Password: ",
            "john.doe@corp.example.local@10.0.1.11's password: ",
            // Old clients pass a server's question without saying whose it is.
            "Password: ",
        ] {
            assert_eq!(classify(prompt, LOGIN), Prompt::Other, "{prompt:?}");
        }
        // And nothing is answered when it is not known whom the password is for.
        let own = "jane.doe@corp.example.local@10.0.1.11's password: ";
        assert_eq!(classify(own, ""), Prompt::Other);
    }

    #[test]
    fn never_offers_the_saved_password_to_other_questions() {
        for question in [
            "New password: ",
            "Retype new password: ",
            "(current) UNIX password: ",
            "Current Password: ",
            "Enter new password again: ",
            "Re-enter password: ",
            "Password expired. Change your password now.\nCurrent Password: ",
            "Verification code: ",
            "Password & verification code: ",
            "Passcode or option (1-3): ",
            "One-time password (OATH) for `jane.doe': ",
            "First Factor: ",
        ] {
            let prompt = format!("({LOGIN}) {question}");
            assert_eq!(classify(&prompt, LOGIN), Prompt::Other, "{prompt:?}");
        }
        for prompt in [
            // ssh's own questions when the password has expired.
            "Enter jane.doe@corp.example.local@10.0.1.11's old password: ",
            "Enter jane.doe@corp.example.local@10.0.1.11's new password: ",
            "Retype jane.doe@corp.example.local@10.0.1.11's new password: ",
            "Enter passphrase for key '/home/me/.ssh/id_ed25519': ",
            "Enter PIN for ED25519-SK key /home/me/.ssh/id_ed25519_sk: ",
            "",
        ] {
            assert_eq!(classify(prompt, LOGIN), Prompt::Other, "{prompt:?}");
        }
    }

    #[test]
    fn recognises_the_host_key_question() {
        let unknown_host =
            "The authenticity of host '10.0.1.11 (10.0.1.11)' can't be established.\n\
             ED25519 key fingerprint is SHA256:abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG.\n\
             This key is not known by any other names.\n\
             Are you sure you want to continue connecting (yes/no/[fingerprint])? ";
        for prompt in [
            unknown_host,
            "Please type 'yes', 'no' or the fingerprint: ",
            "Please type 'yes' or 'no': ",
            "Are you sure you want to continue connecting (yes/no)? ",
        ] {
            assert_eq!(classify(prompt, LOGIN), Prompt::HostKey, "{prompt:?}");
        }
    }

    /// Answers to the host key question are shown while typed. A server must
    /// not be able to get a password shown by wording its question alike.
    #[test]
    fn a_server_cannot_pass_its_question_off_as_the_host_key_question() {
        let prompt = format!("({LOGIN}) Password (yes/no to skip): ");
        assert_ne!(classify(&prompt, LOGIN), Prompt::HostKey);
        let prompt = "(evil@10.0.1.11) Are you sure you want to continue connecting (yes/no)? ";
        assert_ne!(classify(prompt, LOGIN), Prompt::HostKey);
    }

    fn env_names(command: &Command) -> Vec<String> {
        command
            .get_envs()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn arming_points_ssh_at_e2s_without_exposing_the_password() {
        let mut ssh = Command::new("ssh");
        let ticket = arm_unless(false, &mut ssh, LOGIN).expect("ticket");
        assert!(ticket.is_file());
        let names = env_names(&ssh);
        for name in [
            "SSH_ASKPASS",
            "SSH_ASKPASS_REQUIRE",
            ENV_MODE,
            ENV_LOGIN,
            ENV_TICKET,
        ] {
            assert!(names.iter().any(|n| n == name), "{name} not set: {names:?}");
        }
        assert!(!format!("{ssh:?}").to_lowercase().contains("password="));
        fs::remove_file(ticket).unwrap();
    }

    #[test]
    fn a_recently_refused_password_is_not_armed_again() {
        let mut ssh = Command::new("ssh");
        assert!(arm_unless(true, &mut ssh, LOGIN).is_none());
        assert!(env_names(&ssh).is_empty());
    }

    #[test]
    fn strips_control_sequences_from_the_question() {
        assert_eq!(printable("ok\x1b[2Jgone\x07"), "ok [2Jgone ");
        assert_eq!(printable("line one\nline\ttwo: "), "line one\nline\ttwo: ");
    }

    #[test]
    fn ticket_is_private_and_single_use() {
        let ticket = create_ticket().unwrap();
        assert!(ticket.is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&ticket).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        // Same check `take_ticket` performs, without touching the process
        // environment shared with other tests.
        assert!(fs::remove_file(&ticket).is_ok());
        assert!(fs::remove_file(&ticket).is_err());
    }
}
