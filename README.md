# Coldbrew

A Homebrew-compatible package manager in Rust - user-controlled, fast, and reproducible.

## Features

- **Fast**: Written in Rust for maximum performance
- **Reproducible**: Project lockfiles ensure consistent installs
- **Multi-version**: Install multiple versions of packages side-by-side
- **Shim-based**: Like mise, resolves versions based on project configuration
- **Homebrew-compatible**: Uses Homebrew's formula index and bottles

## Design Docs

- Core principles, CLI behavior, and Homebrew integration: `docs/design/core-principles-homebrew.md`
- Performance roadmap and zerobrew-inspired ideas: `docs/design/zerobrew-performance.md`

## Installation

```bash
cargo install coldbrew
```

Or build from source:

```bash
git clone https://github.com/swiftlysingh/coldbrew
cd coldbrew
cargo build --release
```

## Quick Start

```bash
# Update the package index
coldbrew update

# Search for packages
coldbrew search jq

# Install a package
coldbrew install jq

# List installed packages
coldbrew list

# Show package info
coldbrew info jq
```

## Project Configuration

Create a `coldbrew.toml` in your project:

```toml
[packages]
node = "22"
python = "3.12"
jq = "1.7"

[dev_packages]
rust = "1.75"
```

Then run:

```bash
coldbrew lock    # Generate lockfile
coldbrew install # Install from lockfile
```

## Commands

| Command | Description |
|---------|-------------|
| `update` | Update the package index |
| `search <query>` | Search for packages |
| `info <package>` | Show package details |
| `install <packages>` | Install packages |
| `uninstall <packages>` | Uninstall packages |
| `upgrade [packages]` | Upgrade packages |
| `list` | List installed packages |
| `which <binary>` | Show which package provides a binary |
| `pin <package>` | Pin a package version |
| `default <package@version>` | Set default version |
| `deps <package>` | Show dependencies |
| `init` | Create coldbrew.toml |
| `lock` | Generate lockfile |
| `tap <user/repo>` | Add third-party repository |
| `cache clean` | Clean download cache |
| `gc` | Garbage collection |
| `doctor` | Check for problems |
| `shell` | Shell integration setup |

## Shell Integration

Add Coldbrew to your PATH:

```bash
# bash/zsh
export PATH="$HOME/.coldbrew/bin:$PATH"

# fish
fish_add_path ~/.coldbrew/bin
```

## License

MIT
