# 🚀 EC2 TUI

A blazingly fast Terminal User Interface (TUI) for managing AWS EC2 instances, built with Rust. Seamlessly list your EC2 instances and SSH into them with just a few keystrokes.

![EC2 TUI Demo](./.github/assets/e2s.png)


## ✨ Features

- 📋 **Interactive Instance List** - View all your EC2 instances in a beautiful TUI
- 🔐 **Quick SSH Access** - Connect to instances using your local SSH keys
- 👤 **Multi-User Support** - Configure multiple SSH users for different distributions
- ⚙️ **Configurable** - Easy TOML-based configuration
- ⚡ **Fast & Lightweight** - Built with Rust for optimal performance

## 📦 Installation

### Linux & macOS

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/sandeshgrangdan/e2s/releases/download/v0.0.1/e2s-installer.sh | sh
```

### Windows

```powershell
powershell -c "irm https://github.com/sandeshgrangdan/e2s/releases/download/v0.0.1/e2s-installer.ps1 | iex"
```

## 🔧 Prerequisites

Before using EC2 TUI, ensure you have:

1. **SSH Keys Setup** - Your SSH keys must be configured in `~/.ssh/` directory
2. **AWS Credentials** - Valid AWS credentials configured (via AWS CLI or environment variables)
3. **Network Access** - Ability to reach your EC2 instances via SSH

## ⚙️ [Optional] Configuration 

EC2 TUI uses a TOML configuration file located at `~/.config/e2s/config.toml`.

### Example Configuration

```toml
[users]
# Default user - will be auto-selected when app starts
# If not specified, the first user in the list will be selected
default_user = "ubuntu"

# Additional SSH users for different EC2 instances
# Common users for different Linux distributions:
# - Amazon Linux: ec2-user
# - Ubuntu: ubuntu
# - Debian: admin or debian
# - CentOS/RHEL: centos or ec2-user
# - Rocky Linux: rocky
additional_users = [
    "admin",
    "root",
    "centos",
    "debian",
    "fedora",
    "rocky",
    "azureuser",  # For Azure VMs
    "bitnami",    # For Bitnami instances
]
```

### Configuration Options

- **`default_user`** - The SSH user that will be pre-selected when the application starts
- **`additional_users`** - List of additional SSH users to choose from when connecting to instances

## 🎯 Usage

Simply run the command:

```bash
ec2
```

The TUI will launch and display all your EC2 instances. Navigate through the list and select an instance to SSH into it.

## 🗺️ Common SSH Users by Distribution

| Distribution | Default User |
|-------------|--------------|
| Amazon Linux | `ec2-user` |
| Ubuntu | `ubuntu` |
| Debian | `admin` or `debian` |
| CentOS/RHEL | `centos` or `ec2-user` |
| Rocky Linux | `rocky` |
| Fedora | `fedora` |
| Bitnami AMIs | `bitnami` |

## 🤝 Contributing

Contributions are welcome! Feel free to open issues or submit pull requests on [GitHub](https://github.com/sandeshgrangdan/e2s).

## 📝 License

This project is open source and available under the MIT License.

## 🐛 Issues & Support

If you encounter any issues or have questions, please [open an issue](https://github.com/sandeshgrangdan/e2s/issues) on GitHub.

---

Made with ❤️ and Rust