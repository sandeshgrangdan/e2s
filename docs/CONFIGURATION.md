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
(optional)
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
- **Best for:** Multiple simultaneous connections, keeping TUI visible

#### Supported Terminal Emulators

| Terminal | Platform | Description |
|----------|----------|-------------|
| `ghostty` | Linux, macOS | Modern, fast GPU-accelerated terminal |
| `alacritty` | All | GPU-accelerated, cross-platform |
| `kitty` | Linux, macOS | GPU-accelerated with advanced features |
| `wezterm` | All | GPU-accelerated with multiplexing |
| `gnome-terminal` | Linux | GNOME desktop default |
| `konsole` | Linux | KDE desktop default |
| `terminator` | Linux | Multiple terminals in one window |
| `tilix` | Linux | Tiling terminal emulator |
| `xterm` | Linux | Classic, lightweight |
| `foot` | Linux | Lightweight Wayland terminal |
| `rio` | All | GPU-accelerated, written in Rust |
| `iterm2` | macOS | Feature-rich macOS terminal |
| `terminal.app` | macOS | macOS default terminal |

**Note:** If you don't specify a terminal emulator, SSH sessions will run in the same terminal as the TUI, and you'll return to the TUI after disconnecting.

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
   - Verify the terminal is installed: `which kitty`
   - Check the terminal is in your system's PATH
   - Try a different terminal emulator

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