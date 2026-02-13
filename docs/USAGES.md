# EC2 TUI (e2s)

`e2s` is a fast terminal-based tool for discovering AWS EC2 instances and connecting to them via SSH with minimal setup.

This document focuses on installation and configuration to help you get started quickly.

### Linux & macOS

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/sandeshgrangdan/e2s/releases/download/v0.1.5/e2s-installer.sh | sh
```

### Windows

```powershell
powershell -c "irm https://github.com/sandeshgrangdan/e2s/releases/download/v0.1.5/e2s-installer.ps1 | iex"
```

### Cargo

```bash
cargo install e2s
```

## 🔧 Prerequisites

Before using e2s, ensure you have:

1. **SSH Keys** - Your SSH private keys configured in the `~/.ssh/` directory
2. **AWS Credentials** - Valid AWS credentials configured via:
   - AWS CLI (`aws configure`)
   - Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
   - IAM role (for EC2 instances)
3. **Network Access** - Security groups allowing SSH access (port 22) to your instances
4. **Permissions** - AWS IAM permissions to describe EC2 instances

## Quick Start

Launch the TUI with a single command:

```bash
e2s
```

### Command Line Options

```bash
# Use default AWS profile and region
e2s

# Specify AWS profile
e2s --profile production

# Specify AWS region
e2s --region us-west-2

# Combine profile and region
e2s --profile staging --region eu-west-1

# Show help
e2s --help
```

**Available Options:**
- `-p, --profile <PROFILE>` - Name of your AWS profile (default: uses AWS default profile)
- `-r, --region <REGION>` - AWS region to scan (default: uses AWS default region from config)
- `-h, --help` - Display help information
- `-V, --version` - Show version number

**Note:** By default, e2s scans only your default AWS region. To scan other regions, specify them explicitly using the `--region` flag.

### Basic Usage

1. **Launch** - Run `e2s` to see EC2 instances in your default region
2. **Navigate** - Use arrow keys (↑/↓) to move through the instance list
3. **Connect** - Press `Enter` to SSH into the selected instance
4. **Filter** - Start typing to search/filter instances by name or ID
5. **Quit** - Press `q` or `Esc` to exit and return to your terminal

## Configuration

e2s works out of the box with sensible defaults, but you can customize its behavior through a configuration file.

**Configuration Location:** `~/.config/e2s/config.toml`

For detailed configuration options and examples, see [CONFIGURATION.md](https://github.com/sandeshgrangdan/e2s/blob/main/docs/CONFIGURATION.md).

### Quick Configuration Example

```toml
[users]
default_user = "ubuntu"
additional_users = ["ec2-user", "admin", "centos"]

[keys]
default_key = "my-key.pem"

[terminal]
emulator = "alacritty"
```

## Common SSH Users by Distribution

| Distribution | Default SSH User |
|--------------|------------------|
| Amazon Linux 2023 | `ec2-user` |
| Amazon Linux 2 | `ec2-user` |
| Ubuntu | `ubuntu` |
| Debian | `admin` or `debian` |
| CentOS/RHEL | `centos` or `ec2-user` |
| Rocky Linux | `rocky` |
| AlmaLinux | `almalinux` |
| Fedora | `fedora` |
| SUSE/openSUSE | `ec2-user` |
| Bitnami AMIs | `bitnami` |

## Troubleshooting

### Connection Issues

**Problem:** "Permission denied (publickey)"
- Ensure the correct SSH key is being used
- Verify the key has correct permissions (`chmod 400 key.pem`)
- Check that you're using the correct username for your instance's AMI

**Problem:** "Connection timeout"
- Verify security group allows SSH (port 22) from your IP
- Check that the instance is in a running state
- Ensure the instance has a public IP or you're on the same VPC

**Problem:** "No instances found"
- Verify AWS credentials are configured correctly
- Check that you have EC2 describe permissions
- Ensure you're looking at the correct AWS region

### Getting Help

- Check the [Configuration Guide](https://github.com/sandeshgrangdan/e2s/blob/main/docs/CONFIGURATION.md) for setup issues
- Review your AWS credentials: `aws ec2 describe-instances`
- Verify SSH key permissions: `ls -la ~/.ssh/`

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

[Add your license here]

## 🔗 Links

- [GitHub Repository](https://github.com/sandeshgrangdan/e2s)
- [Issue Tracker](https://github.com/sandeshgrangdan/e2s/issues)
- [Configuration Guide](https://github.com/sandeshgrangdan/e2s/blob/main/docs/CONFIGURATION.md)
