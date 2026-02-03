# Coldbrew

A Homebrew-compatible package manager in Rust - user-controlled, fast, and reproducible.

## Features

- **Fast**: Written in Rust for maximum performance
- **Reproducible**: Project lockfiles ensure consistent installs
- **Multi-version**: Install multiple versions of packages side-by-side
- **Shim-based**: Like mise, resolves versions based on project configuration
- **Homebrew-compatible**: Uses Homebrew's formula index and bottles

## Design Docs

- Core principles, CLI behavior, and Homebrew integration: `docs/core-principles-homebrew.md`
- Parallel install pipeline plan: `docs/parallel-pipeline.md`

## Installation

Coldbrew is not published on crates.io yet. Install from source:

```bash
git clone https://github.com/swiftlysingh/coldbrew
cd coldbrew
cargo install --path .
```


## Quick Start

```bash
# Update the package index
crew update

# Search for packages
crew search jq

# Install a package
crew install jq

# List installed packages
crew list

# Show package info
crew info jq
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
crew lock    # Generate lockfile
crew install # Install from lockfile
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
| `init` | Create coldbrew.toml |
| `lock` | Generate lockfile |
| `tap <user/repo>` | Add third-party repository |
| `space` | Show disk usage and cleanup candidates |
| `space clean [--all] [--dry-run]` | Interactive cleanup |
| `doctor` | Check for problems |

## Man Page

The repository includes a manual page at `docs/crew.1`.

Install it locally:

```bash
mkdir -p ~/.local/share/man/man1
cp docs/crew.1 ~/.local/share/man/man1/
man crew
```

## Development

Enable the repository hooks (run once per clone):

```bash
git config core.hooksPath .githooks
```

The pre-commit hook runs `cargo fmt` and re-stages Rust files that were
already staged so formatting issues are fixed before you commit. If
`cargo fmt` is missing, install rustfmt with:

```bash
rustup component add rustfmt
```

## License

MIT
