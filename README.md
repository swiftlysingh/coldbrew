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
| `migrate` | Migrate Homebrew-installed formulas |
| `uninstall <packages>` | Uninstall packages |
| `upgrade [packages]` | Upgrade packages |
| `list` | List installed packages |
| `which <binary>` | Show which package provides a binary |
| `pin <package>` | Pin a package version |
| `unpin <package>` | Unpin a package |
| `default <package@version>` | Set default version |
| `dependents <package>` | Show packages that depend on a package |
| `init` | Create coldbrew.toml |
| `lock` | Generate lockfile |
| `tap <user/repo>` | Add or remove a tap |
| `space` | Show disk usage and cleanup candidates |
| `space clean [--all] [--dry-run]` | Interactive cleanup |
| `link <package>` | Force-link a keg-only package |
| `unlink <package>` | Remove links for a package |
| `doctor` | Check for problems |
| `completions <shell>` | Generate shell completions |

## Migration

Bring your Homebrew-installed formulas into Coldbrew:

```bash
crew migrate
```

Notes:
- Homebrew casks are not supported and will be skipped with a warning.
- Only Homebrew core formulas are migrated.
- After a successful migrate, crew prompts to remove matching Homebrew formulas in interactive sessions; non-interactive runs skip cleanup with a warning.

## Shell Integration

Add Coldbrew to your PATH:

```bash
# bash/zsh
export PATH="$HOME/.coldbrew/bin:$PATH"

# fish
fish_add_path ~/.coldbrew/bin
```

## Man Page

The repository includes a manual page at `docs/man/crew.1`.

Install it locally:

```bash
mkdir -p ~/.local/share/man/man1
cp docs/man/crew.1 ~/.local/share/man/man1/
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
