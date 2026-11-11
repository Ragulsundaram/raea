---
title: "Install raea"
summary: "Install raea on macOS, Linux or Windows, update it, pin a version, and know which files it puts where."
order: 1
sources: ["apps/site/public/install", "apps/site/public/install.ps1", "engine/cli/src/skill.rs"]
---

After this page you will have the `raea` command on your PATH and the raea skill installed for your coding agent.

This page installs the engine. The desktop app, where you make video and games with your AI, downloads from
[raea.io](https://raea.io) for the Mac, Windows and Linux, and tells you when a new version is out.

## Install

On macOS with Apple Silicon, or on Linux on x86_64:

```sh
curl -fsSL https://raea.io/install | sh
```

On Windows 10 or 11, in PowerShell:

```powershell
irm https://raea.io/install.ps1 | iex
```

Then check that it worked:

```sh
raea --version     # prints the version you installed
raea help          # lists every verb
```

The installer needs no account and no API key. Rendering, playing and checking all run on your own computer.

## What goes where

| Path | What it holds |
|---|---|
| `~/.raea/` | The engine, its runtime and SDK, and your settings file (`~/.raea/env`) |
| `~/.local/bin/raea` | A link to the `raea` command, so it is on your PATH |
| `~/.claude/skills/raea/` | The raea skill, so Claude Code knows how to write and render with raea |

If `~/.local/bin` is not on your PATH, the installer says so and prints the line to add to your shell's profile.

## Update

Run the installer again. It replaces the engine with the newest release that has a build for your machine and
refreshes the skill:

```sh
curl -fsSL https://raea.io/install | sh
```

Then see what changed in [What's new](../whats-new/index.md).

## Pick a version

Set `RAEA_VERSION` to install a particular release instead of the newest:

```sh
RAEA_VERSION=0.1.1 curl -fsSL https://raea.io/install | sh
```

On Windows:

```powershell
$env:RAEA_VERSION = "0.1.1"; irm https://raea.io/install.ps1 | iex
```

## Installer settings

| Setting | What it does |
|---|---|
| `RAEA_VERSION` | Installs that release instead of the newest |
| `RAEA_PREFIX` | Installs somewhere other than `~/.raea` |
| `RAEA_NO_SKILLS=1` | Leaves the agent skill out |
| `RAEA_DOWNLOAD` | Downloads the release from a mirror or a local folder instead of GitHub |

## The skill for other agents

The installer adds the skill for Claude Code. For another agent, install it where that agent reads skills:

```sh
raea skills install --agent codex     # also: claude, kimi, gemini
```

For Claude Code this also adds a hook that runs `raea check --quick` after each composition is saved and hands the
agent what it finds. `--no-hooks` leaves the hook out.

If an agent's copy of the skill names a different version than `raea --version` prints, it should read `raea skill`
instead: that is the skill exactly as your installed raea ships it.

## Build from source

From a checkout of the repository:

```sh
cargo build --release -p raea-cli     # the binary is target/release/raea
```

## Supported systems

| System | Status |
|---|---|
| macOS on Apple Silicon | Supported |
| Linux on x86_64 | Supported. An app's window needs a display (X11, Wayland, or Xvfb on a server) |
| Windows 10 and 11 on x86_64 | Supported through `install.ps1` |
| Intel Macs, Linux on ARM | Not yet |
