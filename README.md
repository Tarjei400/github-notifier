# 🛎️ GitHub Notifier

> A minimal Rust desktop app that keeps you updated with GitHub notifications via native desktop popups.

This tool periodically polls GitHub’s notification API and shows a native notification for each unread event — with clickable actions to **open in browser** or **mark as read**.

---

## ⚙️ Features

- ✅ Native desktop notifications with icons
- 🔐 Authenticated GitHub access using `GITHUB_TOKEN`
- 📦 Easily build, install, and run using `make`
- 🔁 Autostarts with your desktop session
- 📂 Persists last read timestamp to avoid duplicate notifications
- 🧪 Actionable notifications (open PR, issue, or mark as read)

---

## 🧰 Setup

### 🔧 Prerequisites

- **Rust toolchain**: Install via [rustup.rs](https://rustup.rs)
- A GitHub [Personal Access Token](https://github.com/settings/tokens) with `notifications` and `repo` scopes
- A Linux desktop environment (for autostart & native notifications) For the time being tested only on Ubuntu.

---

## 🚀 Installation

```bash
export GITHUB_TOKEN=your_personal_access_token
make install
```

This will:

- Build the binary in release mode using Cargo
- Copy the binary to /usr/local/bin/github-notifier
- Copy notification assets (icons) to ~/.config/github-notifier/assets
- Generate and install an autostart .desktop entry at ~/.config/autostart/github-notifier.desktop
- Automatically launch on login, with your GitHub token and asset path injected via the desktop entry

## 🧹 Uninstallation

To completely remove GitHub Notifier:

```bash
make uninstall

```
This will:
- Delete the binary from /usr/local/bin/github-notifier
-  the autostart entry from ~/.config/autostart/github-notifier.desktop

## Logging
Logs (if any) are saved at /tmp/github-notifier.log
You can simply tap into logs for autostarted app using

```bash
make logs
```

## ❤️ Contributions
PRs and feedback welcome! Want to add tray support, configurable polling, or Windows support? Let’s make it happen.

## 📄 License
MIT License — do whatever you want, but give some credit. 😉