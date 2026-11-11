# e2s Configuration Guide

This guide covers all configuration options available in e2s.

## Configuration File Location

The configuration file is located at:
- **Linux/macOS:** `~/.config/e2s/config.toml`
- **Windows:** `%APPDATA%\e2s\config.toml`

## Quick Start

e2s works without any configuration using sensible defaults. Create a config file only if you want to customize behavior.

To create a configuration file:

```bash
mkdir -p ~/.config/e2s
touch ~/.config/e2s/config.toml
```

## Complete Configuration Example

```toml
connect_mode = "ssm" # ssm | private | public
 # Connection mode: ssm (AWS SSM), private (private IP SSH), public (public IP SSH)

[users]
# Default user - will be auto-selected when app starts
# If not specified, the first user in the list will be selected
default_user = "ubuntu"

# Additional SSH users for different EC2 instances
# These will appear as options in the user selection
additional_users = [
    "ec2-user",   # Amazon Linux
    "admin",      # Debian
    "ubuntu",     # Ubuntu
    "centos",     # CentOS
    "debian",     # Debian
    "fedora",     # Fedora
    "rocky",      # Rocky Linux
    "almalinux",  # AlmaLinux
    "root",       # Root access (use carefully)
    "azureuser",  # Azure VMs
    "bitnami",    # Bitnami instances
]

[keys]
# Default SSH key - can be just filename or full path
# If just filename, e2s will look in ~/.ssh/ directory
default_key = "dev-key.pem"

# Alternative: Use full path
# default_key = "/home/username/.ssh/my-key.pem"

# Additional keys from other locations (optional)
# Useful if you store keys in multiple directories
additional_keys = [
    "/home/username/work/production-key.pem",
    "~/Documents/keys/staging-key.pem",
    "/opt/secure/keys/backup-key.pem",
]

[terminal]
# Specify which terminal emulator to use
# If not specified, uses same TUI terminal
emulator = "alacritty"

[ad]
# Optional: log in with an Active Directory account and its password
# instead of an SSH key (press `a` in e2s to switch to it)
username = "jane.doe@corp.example.local"
# Optional: let e2s type the password for you (see the [ad] section below)
password = 'your-ad-password'
```

## Configuration Sections

### `connect_mode` (optional) – Method used to connect to instances
* **Available options:**

  * `ssm` – Connect using AWS Systems Manager (recommended; works for private and public instances)
  * `private` – Connect via SSH using the private IP (**default**)
  * `public` – Connect via SSH using the public IP   

### `[users]` (optional) - SSH User Configuration

Controls which SSH users are available for connecting to instances.

#### `default_user`
- **Type:** String
- **Default:** First user in `additional_users` list
- **Description:** The SSH user that will be pre-selected when the application starts

```toml
default_user = "ubuntu"
```

#### `additional_users`
- **Type:** Array of strings
- **Default:** Empty list
- **Description:** List of SSH usernames that can be selected when connecting to instances

```toml
additional_users = ["ec2-user", "ubuntu", "admin"]
```

**Common Users by Distribution:**

| Distribution | User(s) |
|--------------|---------|
| Amazon Linux 2023 | `ec2-user` |
| Amazon Linux 2 | `ec2-user` |
| Ubuntu | `ubuntu` |
| Debian | `admin`, `debian` |
| CentOS Stream | `centos` |
| RHEL | `ec2-user` |
| Rocky Linux | `rocky` |
| AlmaLinux | `almalinux` |
| Fedora | `fedora` |
| SUSE/openSUSE | `ec2-user` |
| Bitnami | `bitnami` |

### `[keys]` (optional) - SSH Key Configuration

Manages SSH private keys used for authentication.

#### `default_key`
- **Type:** String
- **Default:** Auto-detect from `~/.ssh/`
- **Description:** The SSH key that will be tried first when connecting

**Option 1: Filename only** (key must be in `~/.ssh/`)
```toml
default_key = "my-key.pem"
```

**Option 2: Full path**
```toml
default_key = "/home/username/.ssh/production-key.pem"
```

**Option 3: Tilde expansion**
```toml
default_key = "~/Documents/keys/my-key.pem"
```

#### `additional_keys`
- **Type:** Array of strings
- **Default:** Empty list
- **Description:** Additional SSH keys to try if the default key fails

```toml
additional_keys = [
    "/opt/keys/backup-key.pem",
    "~/secure/emergency-key.pem",
]
```

**Key Requirements:**
- Keys must have correct permissions (`chmod 400` or `chmod 600`)
- Supported formats: PEM, OpenSSH
- Both RSA and Ed25519 keys are supported

### `[ad]` (optional) - Active Directory Login

Logs in with a domain account and its **password** instead of an SSH key - the
same as running:

```bash
ssh -l jane.doe@corp.example.local 10.0.1.25
```

```toml
[ad]
username = "jane.doe@corp.example.local"
password = 'your-ad-password'   # optional
```

#### `username`
- **Type:** String
- **Description:** The login name, exactly as you would give it to `ssh -l`.
  Use the `user@domain` form. (`DOMAIN\user` is refused by current OpenSSH
  clients: "remote username contains invalid characters".)

#### `password`
- **Type:** String
- **Default:** Not set - `ssh` asks for the password every time
- **Description:** If set, e2s answers the server's password question for you.

Write the password between **single quotes**. Inside single quotes TOML takes
every character literally, so backslashes and double quotes need no escaping:

```toml
password = 'p@ss\w"rd'          # single quotes: taken as written
password = '''it's-a-secret'''    # contains a single quote: use three
```

If the config file cannot be parsed (for example an unescaped `\` inside
double quotes), e2s shows `Config error at line N ...` at the bottom of the
screen and uses its defaults.

#### Using it

- Press **`a`** to switch to the AD account, and `a` again to go back to the
  previous user. The account is also part of the user list (`n` / `m`).
- The header shows `Auth: AD password (saved)` - or `(ask)` when no password is
  configured - in place of the SSH key, which is not used for this login.
- Press `Enter` to connect. `p` still chooses between the private and the
  public IP. (SSM sessions do not use SSH users at all.)
- To start e2s with the AD account selected:

  ```toml
  [users]
  default_user = "jane.doe@corp.example.local"   # same value as [ad] username
  ```

#### What e2s does with the password

The password stays in the config file. It is never put on a command line or
into an environment variable; `ssh` asks e2s for it at the moment the server
asks (OpenSSH's `SSH_ASKPASS` mechanism).

To keep the account from being **locked out** by repeated failures, and the
password from going anywhere unintended, e2s is deliberately conservative:

- It only answers the password question of *that login on that host*. Anything
  else is shown to you to answer yourself: the "unknown host key" question,
  one-time codes (MFA), an expired-password dialogue, a jump host's password.
- It sends the saved password **once per connection**. If the server refuses
  it, you are asked to type it.
- After a refusal the saved password is **not sent again for 30 minutes**, or
  until you edit the config file, whichever comes first. During that time
  `ssh` simply asks you.

#### Requirements and notes

- **OpenSSH 8.4 or newer** on your machine for the automatic password
  (`ssh -V`). With an older client `ssh` just asks for the password as usual.
- The password is stored **in plain text**. Protect the file, and prefer not
  setting `password` at all on shared machines:

  ```bash
  chmod 600 ~/.config/e2s/config.toml
  ```
- The password is sent to whichever instance you select. Only use the AD
  login with servers that are joined to your domain, and do not accept host
  keys you do not expect.
- The server must allow password logins (`PasswordAuthentication` or
  `KbdInteractiveAuthentication` in its `sshd_config`).

### `[terminal]` - Terminal Emulator Configuration

Specifies which terminal emulator to use for SSH connections.

#### `emulator`
- **Type:** String
- **Default:** Not set (uses same TUI terminal)
- **Description:** The terminal emulator to launch for SSH sessions

**Behavior:**

**Option 1: No emulator specified** (default)
```toml
# [terminal] section omitted or emulator not set
```
- SSH sessions launch directly in the same terminal window
- You stay within the TUI interface
- After SSH disconnection, you automatically return to the TUI
- **Best for:** Quick connections, staying in one terminal

**Option 2: Emulator specified**
```toml
[terminal]
emulator = "iterm2"
```
- SSH sessions launch in a new terminal window
- The TUI remains open in the original terminal
- Each connection opens a separate window
- When the session ends the window stays open with your shell, so that any
  error message remains readable
- **Best for:** Multiple simultaneous connections, keeping TUI visible

#### Supported Terminal Emulators

| Terminal | Platform | Description |
|----------|----------|-------------|
| `ghostty` | Linux, macOS | Modern, fast GPU-accelerated terminal |
| `alacritty` | Linux, macOS, Windows, WSL | GPU-accelerated, cross-platform |
| `kitty` | Linux, macOS | GPU-accelerated with advanced features |
| `wezterm` | Linux, macOS, Windows, WSL | GPU-accelerated with multiplexing |
| `rio` | Linux, macOS, Windows, WSL | GPU-accelerated, written in Rust |
| `wt` (or `windows-terminal`) | Windows, WSL | Windows Terminal |
| `gnome-terminal` | Linux | GNOME desktop default |
| `konsole` | Linux | KDE desktop default |
| `terminator` | Linux | Multiple terminals in one window |
| `tilix` | Linux | Tiling terminal emulator |
| `xterm`, `urxvt`, `st` | Linux | Classic, lightweight |
| `foot` | Linux | Lightweight Wayland terminal |
| `iterm2` | macOS | Feature-rich macOS terminal |
| `terminal` (or `terminal.app`) | macOS | macOS default terminal |
| `warp` | macOS | Warp (needs Accessibility permission) |

**Note:** If you don't specify a terminal emulator, SSH sessions will run in the same terminal as the TUI, and you'll return to the TUI after disconnecting.

**WSL:** for `alacritty`, `wezterm` and `rio` e2s uses the Windows program
(`alacritty.exe`, ...) when it is on your `PATH`, and otherwise the Linux one
installed inside WSL. `wt` always is the Windows program.

**How the new window is started:** the terminal is asked to run e2s itself
(`e2s __connect ...`), which then starts `ssh` or `aws` with exactly the
right arguments. Terminals differ in how they pass a command along, and
handing them the `ssh` command line directly is what made some of them
(Alacritty on Windows/WSL, Ghostty before 1.2) show the `ssh` usage text
instead of connecting in e2s 0.1.5 and earlier.

## Configuration Examples

### Example 1: AWS Multi-Environment Setup

```toml
[users]
default_user = "ec2-user"
additional_users = ["ubuntu", "admin", "centos"]

[keys]
default_key = "production-key.pem"
additional_keys = [
    "~/.ssh/staging-key.pem",
    "~/.ssh/development-key.pem",
]

[terminal]
emulator = "kitty"
```

### Example 2: Ubuntu-Focused Setup

```toml
[users]
default_user = "ubuntu"
additional_users = ["admin", "root"]

[keys]
default_key = "ubuntu-default.pem"

[terminal]
emulator = "alacritty"
```

### Example 3: Mixed Cloud Provider Setup

```toml
[users]
default_user = "ec2-user"
additional_users = [
    "ubuntu",
    "admin",
    "azureuser",  # Azure VMs
    "opc",        # Oracle Cloud
]

[keys]
default_key = "aws-main.pem"
additional_keys = [
    "/keys/azure-key.pem",
    "/keys/oracle-key.pem",
]
```

### Example 4: Minimal Configuration (In-TUI SSH)

```toml
[users]
default_user = "ubuntu"
```

This minimal configuration:
- Uses Ubuntu as the default SSH user
- SSH sessions run in the same terminal (no new windows)
- Returns to TUI after disconnecting from SSH
- Auto-detects SSH keys from `~/.ssh/`

### Example 5: New Window SSH Sessions

```toml
[users]
default_user = "ec2-user"

[terminal]
emulator = "kitty"
```

This configuration:
- Opens each SSH session in a new Kitty terminal window
- Keeps the TUI visible in the original terminal
- Allows multiple simultaneous connections

### Example 6: Active Directory Login in a New Window (WSL)

```toml
connect_mode = "private"

[users]
# Start with the AD account selected
default_user = "jane.doe@corp.example.local"

[ad]
username = "jane.doe@corp.example.local"
password = 'your-ad-password'

[terminal]
emulator = "wezterm"
```

This configuration:
- Connects with `ssh -l jane.doe@corp.example.local <private ip>`
- Types the AD password for you
- Opens each session in a new WezTerm window
- `a` switches to key-based logins (`ec2-user`, `ubuntu`, ...) and back

## 🔍 Troubleshooting Configuration

### Config File Not Found
**Solution:** e2s will use defaults if no config file exists. This is normal and expected.

### Key Permission Errors
```bash
# Fix key permissions
chmod 400 ~/.ssh/your-key.pem
```

### User Authentication Failures
1. Verify the username matches your instance's AMI
2. Check the instance's cloud-init logs: `sudo cat /var/log/cloud-init.log`
3. Try common usernames for your distribution (see table above)

### Terminal Not Launching
1. If no terminal is configured, SSH runs in the same window (this is expected behavior)
2. To use a separate terminal window, add the `[terminal]` section to your config
3. If a terminal is configured but not launching:
   - Read the red message at the bottom of the e2s screen - it says what could
     not be started (for example an unknown terminal name)
   - Verify the terminal is installed: `which kitty`
   - Check the terminal is in your system's PATH
   - Try a different terminal emulator

### New Window Shows the `ssh` Usage Text
```
usage: ssh [-46AaCfGgKkMNnqsTtVvXxYy] [-B bind_interface] ...
```
This happened with some terminals (Alacritty on Windows/WSL, Ghostty before
1.2) in e2s 0.1.5 and earlier. Update e2s.

### Active Directory Login
- **`a` says "No Active Directory account"** - there is no `[ad]` section with
  a `username`, or the config file has a syntax error (see the message shown
  when e2s starts).
- **The password is not typed for you** - check `ssh -V` (8.4 or newer is
  needed); or the saved password was refused in the last 30 minutes (e2s says
  so) - correct it in the config file.
- **`Permission denied`** - the password is wrong, the account is locked or
  expired, or the account is not allowed to log in to that server. Try the
  same login by hand: `ssh -l jane.doe@corp.example.local <ip>`.
- **`remote username contains invalid characters`** - use `user@domain`
  instead of `DOMAIN\user`.
- **Asked "Are you sure you want to continue connecting"** - normal the first
  time you connect to a host; answer it as you would for `ssh`.

## 🔧 Advanced Tips

### Using Multiple Configurations
You can maintain different configs and switch between them:

```bash
# Use custom config location
export E2S_CONFIG="/path/to/custom/config.toml"
e2s
```

### Key Path Best Practices
1. Store production keys in a secure location (`/opt/secure/keys/`)
2. Set restrictive directory permissions (`chmod 700`)
3. Use descriptive key names (`prod-web-servers.pem`, `staging-db.pem`)

### Region-Specific Configuration
By default, e2s scans only your default AWS region. To scan specific regions:

```bash
# Scan a specific region
e2s --region us-west-2

# Use a specific AWS profile
e2s --profile production

# Combine both
e2s --profile production --region eu-central-1

# Help
e2s --help
```

To change your default region permanently:
```bash
aws configure set region us-east-1
```

#📚 Related Documentation

- [AWS EC2 SSH Best Practices](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-key-pairs.html)
- [SSH Key Permissions Guide](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/connection-prereqs.html)