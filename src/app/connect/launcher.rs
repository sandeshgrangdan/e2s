//! Opening a connection in a new terminal window.
//!
//! Every terminal is asked to run the same thing, `e2s __connect <tokens>`
//! (see the parent module for why), so all this module has to know about a
//! terminal is how it is told to run a program.

use std::env;
use std::io;
use std::process::{Command, Stdio};

use super::{forwarded_env, is_safe_token, ConnectSpec, SUBCOMMAND};

/// Where e2s is running, as far as starting a terminal is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOs,
    Windows,
    /// Linux inside WSL. The terminal is normally a Windows program there,
    /// and has to come back into this distribution through `wsl.exe`.
    Wsl {
        distro: Option<String>,
    },
}

/// What [`plan`] needs to know about the machine. Gathered by
/// [`Host::detect`]; spelled out in tests.
#[derive(Debug, Clone)]
pub struct Host {
    pub platform: Platform,
    /// The running e2s binary, which the new window starts again.
    pub exe: String,
    /// Shell to leave the new window in when the session ends.
    pub shell: Option<String>,
    /// Environment to carry into the new window.
    pub env: Vec<(String, String)>,
    /// Whether a program of that name is on `PATH`.
    pub has_program: fn(&str) -> bool,
}

impl Host {
    pub fn detect() -> io::Result<Self> {
        let platform = if cfg!(target_os = "macos") {
            Platform::MacOs
        } else if cfg!(windows) {
            Platform::Windows
        } else if is_wsl() {
            Platform::Wsl {
                distro: env::var("WSL_DISTRO_NAME")
                    .ok()
                    .filter(|name| is_safe_token(name)),
            }
        } else {
            Platform::Linux
        };

        let exe = super::current_exe()?
            .into_os_string()
            .into_string()
            .map_err(|_| io::Error::other("the path of the e2s binary is not valid UTF-8"))?;

        let shell = if cfg!(unix) {
            env::var("SHELL")
                .ok()
                .filter(|shell| !shell.is_empty())
                .or_else(|| Some("/bin/sh".to_string()))
        } else {
            None
        };

        let env = forwarded_env(matches!(platform, Platform::Wsl { .. }));

        Ok(Self {
            platform,
            exe,
            shell,
            env,
            has_program: on_path,
        })
    }
}

/// The process to start in order to get a new terminal window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
}

/// How a terminal emulator is told to run a program in a new window.
struct Emulator {
    /// Executable name (`.exe` is appended for the Windows build under WSL).
    program: &'static str,
    /// Arguments that go before the program to run.
    run: &'static [&'static str],
    /// Whether a Windows build exists.
    windows: bool,
}

const fn emulator(program: &'static str, run: &'static [&'static str]) -> Emulator {
    Emulator {
        program,
        run,
        windows: false,
    }
}

/// Terminals that take the program to run on their command line.
fn command_line_emulator(name: &str) -> Option<Emulator> {
    Some(match name {
        "alacritty" => Emulator {
            windows: true,
            ..emulator("alacritty", &["-e"])
        },
        "wezterm" => Emulator {
            windows: true,
            ..emulator("wezterm", &["start", "--"])
        },
        "rio" => Emulator {
            windows: true,
            ..emulator("rio", &["-e"])
        },
        "ghostty" => emulator("ghostty", &["-e"]),
        "kitty" => emulator("kitty", &[]),
        "foot" => emulator("foot", &[]),
        "gnome-terminal" => emulator("gnome-terminal", &["--"]),
        "konsole" => emulator("konsole", &["-e"]),
        "xterm" => emulator("xterm", &["-e"]),
        "rxvt" => emulator("rxvt", &["-e"]),
        "urxvt" => emulator("urxvt", &["-e"]),
        "st" => emulator("st", &["-e"]),
        "terminator" => emulator("terminator", &["-x"]),
        "tilix" => emulator("tilix", &["-e"]),
        _ => return None,
    })
}

/// Names accepted for `[terminal] emulator`, for error messages.
const SUPPORTED: &str = "alacritty, ghostty, kitty, wezterm, rio, foot, gnome-terminal, konsole, \
    xterm, urxvt, st, tilix, terminator, wt (Windows Terminal), iterm2, terminal (macOS), warp";

/// Works out how to open `spec` in a new window of `emulator`.
pub fn plan(emulator: &str, spec: &ConnectSpec, host: &Host) -> Result<Launch, String> {
    let name = emulator.trim().to_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    let name = match name {
        "iterm" => "iterm2",
        "terminal.app" => "terminal",
        "windows-terminal" => "wt",
        other => other,
    };

    // What the new window runs. Terminals started with a command leave no
    // shell behind, so e2s drops into one itself once the session is over.
    let command = |spec: &ConnectSpec| -> Vec<String> {
        let mut spec = spec.clone();
        spec.env = host.env.clone();
        if host.platform == Platform::Windows {
            spec.pause = true;
        } else {
            spec.shell = host.shell.clone();
        }
        let mut argv = vec![host.exe.clone(), SUBCOMMAND.to_string()];
        argv.extend(spec.to_tokens());
        argv
    };

    // Windows programs started from WSL run on the Windows side: to get back
    // into this distribution the command has to go through wsl.exe.
    let through_wsl = |home: bool, argv: Vec<String>| -> Vec<String> {
        let mut hop = vec!["wsl.exe".to_string()];
        if home {
            // Start in the home directory; has to be the first argument.
            hop.push("~".to_string());
        }
        if let Platform::Wsl {
            distro: Some(distro),
        } = &host.platform
        {
            hop.extend(["-d".to_string(), distro.clone()]);
        }
        hop.push("-e".to_string());
        hop.extend(argv);
        hop
    };

    let is_wsl = matches!(host.platform, Platform::Wsl { .. });

    if name == "wt" {
        let argv = match host.platform {
            Platform::Wsl { .. } => through_wsl(true, command(spec)),
            Platform::Windows => command(spec),
            _ => return Err("Windows Terminal is only available on Windows and WSL".to_string()),
        };
        let mut args = vec!["--".to_string()];
        args.extend(argv);
        return Ok(Launch {
            program: "wt.exe".to_string(),
            args,
        });
    }

    if let Some(emulator) = command_line_emulator(name) {
        let windows_build = format!("{}.exe", emulator.program);
        let use_windows_build = is_wsl && emulator.windows && (host.has_program)(&windows_build);

        if name == "ghostty" && host.platform == Platform::MacOs {
            // The ghostty command cannot open windows on macOS; the app has
            // to be started through `open`.
            let mut args: Vec<String> = ["-na", "Ghostty.app", "--args", "-e"]
                .map(String::from)
                .to_vec();
            args.extend(command(spec));
            return Ok(Launch {
                program: "open".to_string(),
                args,
            });
        }

        let (program, argv) = if use_windows_build {
            (windows_build, through_wsl(false, command(spec)))
        } else {
            (emulator.program.to_string(), command(spec))
        };

        let mut args: Vec<String> = emulator.run.iter().map(|arg| arg.to_string()).collect();
        if name == "wezterm" && use_windows_build {
            // Start in the directory e2s was started from.
            args = vec!["start".into(), "--cwd".into(), ".".into(), "--".into()];
        }
        args.extend(argv);
        return Ok(Launch { program, args });
    }

    if host.platform == Platform::MacOs {
        let line = typed_line(spec, host);
        let script = match name {
            "iterm2" => Some(format!(
                r#"tell application "iTerm"
    create window with default profile
    tell current session of current window
        write text "{line}"
    end tell
end tell"#
            )),
            "terminal" => Some(format!(
                r#"tell application "Terminal"
    do script "{line}"
    activate
end tell"#
            )),
            "warp" => Some(format!(
                r#"tell application "Warp"
    activate
end tell
delay 0.5
tell application "System Events"
    keystroke "t" using {{command down}}
    delay 0.2
    keystroke "{line}"
    keystroke return
end tell"#
            )),
            _ => None,
        };
        if let Some(script) = script {
            return Ok(apple_script(script));
        }
    }

    Err(format!(
        "unknown or unsupported terminal emulator `{emulator}` (supported: {SUPPORTED})"
    ))
}

/// The command as text typed into the shell of a freshly opened terminal
/// tab, already escaped for use inside an AppleScript string. That shell
/// stays around afterwards, so nothing has to keep the window open, and it
/// has the user's usual environment.
fn typed_line(spec: &ConnectSpec, host: &Host) -> String {
    let mut words = vec![shell_quote(&host.exe), SUBCOMMAND.to_string()];
    words.extend(spec.to_tokens());
    words.join(" ").replace('\\', "\\\\").replace('"', "\\\"")
}

fn apple_script(script: String) -> Launch {
    Launch {
        program: "osascript".to_string(),
        args: vec!["-e".to_string(), script],
    }
}

/// Quotes `word` for a POSIX shell, unless it needs none.
fn shell_quote(word: &str) -> String {
    if is_safe_token(word) {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

/// Opens `spec` in a new window of the configured terminal emulator.
pub fn open(emulator: &str, spec: &ConnectSpec) -> io::Result<()> {
    let host = Host::detect()?;
    let launch = plan(emulator, spec, &host).map_err(io::Error::other)?;

    let mut child = Command::new(&launch.program)
        .args(&launch.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| io::Error::new(err.kind(), format!("`{}`: {err}", launch.program)))?;

    // Collect the exit status whenever the terminal goes away, so that closed
    // windows do not pile up as zombie processes while e2s keeps running.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn on_path(program: &str) -> bool {
    env::var_os("PATH")
        .is_some_and(|path| env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/version").is_ok_and(|version| {
        let version = version.to_lowercase();
        version.contains("microsoft") || version.contains("wsl")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::connect::{SshAuth, Target};
    use pretty_assertions::assert_eq;

    const EXE: &str = "/home/me/.cargo/bin/e2s";

    fn spec() -> ConnectSpec {
        ConnectSpec::new(Target::Ssh {
            host: "10.0.1.11".into(),
            user: "ubuntu".into(),
            auth: SshAuth::Key("/home/me/.ssh/dev key.pem".into()),
        })
    }

    fn host(platform: Platform, has_program: fn(&str) -> bool) -> Host {
        Host {
            platform,
            exe: EXE.into(),
            shell: Some("/usr/bin/zsh".into()),
            env: vec![("PATH".into(), "/home/me/.local/bin:/usr/bin".into())],
            has_program,
        }
    }

    fn wsl() -> Platform {
        Platform::Wsl {
            distro: Some("Ubuntu-24.04".into()),
        }
    }

    fn everything(_: &str) -> bool {
        true
    }

    fn nothing(_: &str) -> bool {
        false
    }

    /// What the new window is expected to run on Linux/WSL/macOS.
    fn command() -> Vec<String> {
        [
            EXE,
            "__connect",
            "--ssh=10.0.1.11",
            "--user=ubuntu",
            "--key=/home/me/.ssh/dev%20key.pem",
            "--shell=/usr/bin/zsh",
            "--env=PATH%3D/home/me/.local/bin:/usr/bin",
        ]
        .map(String::from)
        .to_vec()
    }

    fn launch(program: &str, prefix: &[&str]) -> Launch {
        let mut args: Vec<String> = prefix.iter().map(|arg| arg.to_string()).collect();
        args.extend(command());
        Launch {
            program: program.into(),
            args,
        }
    }

    const ALL: [&str; 17] = [
        "alacritty",
        "ghostty",
        "kitty",
        "wezterm",
        "rio",
        "foot",
        "gnome-terminal",
        "konsole",
        "xterm",
        "rxvt",
        "urxvt",
        "st",
        "tilix",
        "terminator",
        "wt",
        "iterm2",
        "terminal",
    ];

    #[test]
    fn linux_terminals_run_e2s_directly() {
        let host = host(Platform::Linux, everything);
        let cases: [(&str, &str, &[&str]); 13] = [
            ("tilix", "tilix", &["-e"]),
            ("alacritty", "alacritty", &["-e"]),
            ("ghostty", "ghostty", &["-e"]),
            ("kitty", "kitty", &[]),
            ("wezterm", "wezterm", &["start", "--"]),
            ("rio", "rio", &["-e"]),
            ("foot", "foot", &[]),
            ("gnome-terminal", "gnome-terminal", &["--"]),
            ("konsole", "konsole", &["-e"]),
            ("xterm", "xterm", &["-e"]),
            ("urxvt", "urxvt", &["-e"]),
            ("st", "st", &["-e"]),
            ("terminator", "terminator", &["-x"]),
        ];
        for (name, program, prefix) in cases {
            assert_eq!(
                plan(name, &spec(), &host),
                Ok(launch(program, prefix)),
                "{name}"
            );
        }
    }

    #[test]
    fn name_is_case_insensitive_and_tolerates_exe_suffix() {
        let host = host(Platform::Linux, everything);
        for name in ["Alacritty", " alacritty ", "ALACRITTY.EXE"] {
            assert_eq!(
                plan(name, &spec(), &host),
                Ok(launch("alacritty", &["-e"])),
                "{name}"
            );
        }
    }

    #[test]
    fn wsl_goes_back_through_wsl_exe_for_windows_terminals() {
        let host = host(wsl(), everything);
        let hop = ["wsl.exe", "-d", "Ubuntu-24.04", "-e"];

        let alacritty = plan("alacritty", &spec(), &host).unwrap();
        assert_eq!(
            alacritty,
            launch("alacritty.exe", &[&["-e"], &hop[..]].concat())
        );

        let wezterm = plan("wezterm", &spec(), &host).unwrap();
        assert_eq!(
            wezterm,
            launch(
                "wezterm.exe",
                &[&["start", "--cwd", ".", "--"], &hop[..]].concat()
            )
        );

        let wt = plan("windows-terminal", &spec(), &host).unwrap();
        assert_eq!(
            wt,
            launch(
                "wt.exe",
                &["--", "wsl.exe", "~", "-d", "Ubuntu-24.04", "-e"]
            )
        );
    }

    #[test]
    fn wsl_without_distro_name_uses_the_default_distribution() {
        let host = host(Platform::Wsl { distro: None }, everything);
        let alacritty = plan("alacritty", &spec(), &host).unwrap();
        assert_eq!(alacritty, launch("alacritty.exe", &["-e", "wsl.exe", "-e"]));
    }

    #[test]
    fn wsl_uses_the_linux_build_when_there_is_no_windows_one() {
        // Installed inside WSL (WSLg) and not on Windows.
        let host = host(wsl(), nothing);
        assert_eq!(
            plan("alacritty", &spec(), &host),
            Ok(launch("alacritty", &["-e"]))
        );
        // No Windows build exists at all.
        let host = self::host(wsl(), everything);
        assert_eq!(plan("kitty", &spec(), &host), Ok(launch("kitty", &[])));
        assert_eq!(
            plan("ghostty", &spec(), &host),
            Ok(launch("ghostty", &["-e"]))
        );
    }

    #[test]
    fn windows_waits_for_enter_instead_of_starting_a_shell() {
        let mut host = host(Platform::Windows, nothing);
        host.exe = r"C:\Users\me\.cargo\bin\e2s.exe".into();
        host.shell = None;
        host.env = Vec::new();
        let expected: Vec<String> = [
            r"C:\Users\me\.cargo\bin\e2s.exe",
            "__connect",
            "--ssh=10.0.1.11",
            "--user=ubuntu",
            "--key=/home/me/.ssh/dev%20key.pem",
            "--pause",
        ]
        .map(String::from)
        .to_vec();

        let alacritty = plan("alacritty", &spec(), &host).unwrap();
        assert_eq!(alacritty.program, "alacritty");
        assert_eq!(
            alacritty.args,
            [vec!["-e".to_string()], expected.clone()].concat()
        );

        let wt = plan("wt", &spec(), &host).unwrap();
        assert_eq!(wt.program, "wt.exe");
        assert_eq!(wt.args, [vec!["--".to_string()], expected].concat());
    }

    #[test]
    fn macos_scripted_terminals_type_a_short_readable_line() {
        let host = host(Platform::MacOs, everything);
        let line = format!(
            "{EXE} __connect --ssh=10.0.1.11 --user=ubuntu --key=/home/me/.ssh/dev%20key.pem"
        );
        for (name, needle) in [
            ("iterm2", format!("write text \"{line}\"")),
            ("iTerm", format!("write text \"{line}\"")),
            ("terminal", format!("do script \"{line}\"")),
            ("Terminal.app", format!("do script \"{line}\"")),
            ("warp", format!("keystroke \"{line}\"")),
        ] {
            let launched = plan(name, &spec(), &host).unwrap();
            assert_eq!(launched.program, "osascript", "{name}");
            assert_eq!(launched.args[0], "-e", "{name}");
            assert!(
                launched.args[1].contains(&needle),
                "{name}: {}",
                launched.args[1]
            );
        }
    }

    #[test]
    fn macos_ghostty_is_started_through_open() {
        let host = host(Platform::MacOs, everything);
        assert_eq!(
            plan("ghostty", &spec(), &host),
            Ok(launch("open", &["-na", "Ghostty.app", "--args", "-e"]))
        );
    }

    #[test]
    fn typed_line_quotes_an_awkward_binary_path() {
        let mut host = host(Platform::MacOs, everything);
        host.exe = r#"/Users/Jane "JD" O'Neil/bin/e2s"#.into();
        let line = typed_line(&spec(), &host);
        // Shell quoting first, AppleScript string escaping on top.
        assert!(
            line.starts_with(r#"'/Users/Jane \"JD\" O'\\''Neil/bin/e2s' __connect "#),
            "{line}"
        );
    }

    #[test]
    fn macos_only_terminals_are_refused_elsewhere() {
        for platform in [Platform::Linux, wsl(), Platform::Windows] {
            let host = host(platform, everything);
            for name in ["iterm2", "terminal", "warp"] {
                assert!(plan(name, &spec(), &host).is_err(), "{name}");
            }
        }
        for platform in [Platform::Linux, Platform::MacOs] {
            assert!(plan("wt", &spec(), &host(platform, everything)).is_err());
        }
    }

    #[test]
    fn unknown_terminal_is_an_error_that_lists_the_choices() {
        let error = plan("notaterm", &spec(), &host(Platform::Linux, everything)).unwrap_err();
        assert!(error.contains("notaterm"), "{error}");
        assert!(error.contains("alacritty"), "{error}");
    }

    /// The reason this module exists: some terminals (or what they go
    /// through) join the command with spaces and split it again, or hand it
    /// to a shell. Whatever is selected, the command must survive that.
    #[test]
    fn command_survives_being_joined_and_split_again() {
        let mut spec = ConnectSpec::new(Target::Ssh {
            host: "10.0.1.11".into(),
            user: r#"CORP\jane doe;$(reboot) "x" 'y'"#.into(),
            auth: SshAuth::Password { autofill: true },
        });
        spec.config_path = Some("/home/me/my configs/e2s.toml".into());

        let platforms = [Platform::Linux, wsl(), Platform::MacOs, Platform::Windows];
        for platform in platforms {
            for available in [everything as fn(&str) -> bool, nothing] {
                let mut host = host(platform.clone(), available);
                host.env.push(("AWS_PROFILE".into(), "prod & co".into()));
                for name in ALL {
                    let Ok(launched) = plan(name, &spec, &host) else {
                        continue;
                    };
                    if launched.program == "osascript" {
                        continue; // typed into a shell, checked separately
                    }
                    let start = launched
                        .args
                        .iter()
                        .position(|arg| arg.contains("__connect"))
                        .unwrap_or_else(|| panic!("{name}: {launched:?}"));
                    // Joined with spaces and split again.
                    let flattened = launched.args[start + 1..].join(" ");
                    let words: Vec<&str> = flattened.split(' ').collect();
                    for word in &words {
                        assert!(is_safe_token(word), "{name} on {platform:?}: {word}");
                    }
                    let reparsed = ConnectSpec::from_tokens(&words)
                        .unwrap_or_else(|err| panic!("{name} on {platform:?}: {err}"));
                    assert_eq!(reparsed.target, spec.target, "{name} on {platform:?}");
                    assert_eq!(reparsed.config_path, spec.config_path);
                }
            }
        }
    }
}
