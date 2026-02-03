# crew command reference

## Global help
```
A Homebrew-compatible package manager - user-controlled, fast, and reproducible

Usage: crew [OPTIONS] [COMMAND]

Commands:
  update       Update the package index from Homebrew
  search       Search for packages
  info         Show information about a package
  install      Install packages
  migrate      Migrate Homebrew-installed formulas
  uninstall    Uninstall packages
  upgrade      Upgrade installed packages
  list         List installed packages
  which        Show which package provides a binary
  pin          Pin a package to prevent upgrades
  unpin        Unpin a package to allow upgrades
  default      Set or show the default version for a package
  dependents   Show packages that depend on a package
  init         Initialize a new coldbrew.toml in the current directory
  lock         Generate a lockfile from coldbrew.toml
  tap          Add or remove taps (third-party repositories)
  space        Disk usage and cleanup commands
  link         Force-link a keg-only package
  unlink       Remove links for a package
  doctor       Check system for potential problems
  completions  Generate shell completions
  help         Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## update
```
Update the package index from Homebrew

Usage: crew update [OPTIONS]

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## search
```
Search for packages

Usage: crew search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Search query

Options:
  -e, --extended  Show extended information
  -v, --verbose   Enable verbose output
  -q, --quiet     Suppress non-error output
  -h, --help      Print help
  -V, --version   Print version
```

## info
```
Show information about a package

Usage: crew info [OPTIONS] <PACKAGE>

Arguments:
  <PACKAGE>  Package name

Options:
  -f, --format <FORMAT>  Output format (text, json) [default: text]
  -v, --verbose          Enable verbose output
  -q, --quiet            Suppress non-error output
  -h, --help             Print help
  -V, --version          Print version
```

## install
```
Install packages

Usage: crew install [OPTIONS] <PACKAGES>...

Arguments:
  <PACKAGES>...  Packages to install (e.g., jq, node@22)

Options:
      --skip-deps  Skip dependency installation
  -v, --verbose    Enable verbose output
  -f, --force      Force reinstall even if already installed
  -q, --quiet      Suppress non-error output
  -h, --help       Print help
  -V, --version    Print version
```

## migrate
```
Migrate Homebrew-installed formulas

Usage: crew migrate [OPTIONS]

Options:
      --brew <BREW>  Path to Homebrew brew binary
      --dry-run      Show what would be migrated
  -v, --verbose      Enable verbose output
  -q, --quiet        Suppress non-error output
  -h, --help         Print help
  -V, --version      Print version
```

Interactive runs prompt to remove migrated Homebrew formulas after success; non-interactive sessions skip cleanup with a warning.

## uninstall
```
Uninstall packages

Usage: crew uninstall [OPTIONS] <PACKAGES>...

Arguments:
  <PACKAGES>...  Packages to uninstall

Options:
  -a, --all        Remove all versions
  -v, --verbose    Enable verbose output
  -q, --quiet      Suppress non-error output
      --with-deps  Also remove unused dependencies
  -h, --help       Print help
  -V, --version    Print version
```

## upgrade
```
Upgrade installed packages

Usage: crew upgrade [OPTIONS] [PACKAGES]...

Arguments:
  [PACKAGES]...  Packages to upgrade (all if not specified)

Options:
  -v, --verbose  Enable verbose output
  -y, --yes      Skip interactive selection
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## list
```
List installed packages

Usage: crew list [OPTIONS]

Options:
  -n, --names-only           Show only package names
  -v, --verbose              Enable verbose output
  -q, --quiet                Suppress non-error output
  -v, --versions <VERSIONS>  Show versions for a specific package
  -h, --help                 Print help
  -V, --version              Print version
```

## which
```
Show which package provides a binary

Usage: crew which [OPTIONS] <BINARY>

Arguments:
  <BINARY>  Binary name

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## pin
```
Pin a package to prevent upgrades

Usage: crew pin [OPTIONS] <PACKAGE>

Arguments:
  <PACKAGE>  Package to pin

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## unpin
```
Unpin a package to allow upgrades

Usage: crew unpin [OPTIONS] <PACKAGE>

Arguments:
  <PACKAGE>  Package to unpin

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## default
```
Set or show the default version for a package

Usage: crew default [OPTIONS] <PACKAGE>

Arguments:
  <PACKAGE>  Package name (e.g., node@22 or just node to show current)

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## dependents
```
Show packages that depend on a package

Usage: crew dependents [OPTIONS] <PACKAGE>

Arguments:
  <PACKAGE>  Package name

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## init
```
Initialize a new coldbrew.toml in the current directory

Usage: crew init [OPTIONS]

Options:
  -f, --force    Force overwrite if file exists
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## lock
```
Generate a lockfile from coldbrew.toml

Usage: crew lock [OPTIONS]

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## tap
```
Add or remove taps (third-party repositories)

Usage: crew tap [OPTIONS] [TAP]

Arguments:
  [TAP]  Tap to add (user/repo format)

Options:
  -r, --remove   Remove a tap instead of adding
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## space
```
Disk usage and cleanup commands

Usage: crew space [COMMAND]

Commands:
  show   Show disk usage and cleanup candidates
  clean  Cleanup old versions, cache, and other unused data

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## space show
```
Show disk usage and cleanup candidates

Usage: crew space show [OPTIONS]

Options:
  -d, --details  Show itemized details
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## space clean
```
Cleanup old versions, cache, and other unused data

Usage: crew space clean [OPTIONS]

Options:
  -a, --all      Clean everything without prompts
  -v, --verbose  Enable verbose output
  -d, --dry-run  Dry run - show what would be removed
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## link
```
Force-link a keg-only package

Usage: crew link [OPTIONS] <PACKAGE>

Arguments:
  <PACKAGE>  Package to link

Options:
  -f, --force    Force overwrite existing files
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## unlink
```
Remove links for a package

Usage: crew unlink [OPTIONS] <PACKAGE>

Arguments:
  <PACKAGE>  Package to unlink

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## doctor
```
Check system for potential problems

Usage: crew doctor [OPTIONS]

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## completions
```
Generate shell completions

Usage: crew completions [OPTIONS] <SHELL>

Arguments:
  <SHELL>  Shell to generate completions for [possible values: bash, elvish, fish, powershell, zsh]

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```

## help
```
A Homebrew-compatible package manager - user-controlled, fast, and reproducible

Usage: crew [OPTIONS] [COMMAND]

Commands:
  update       Update the package index from Homebrew
  search       Search for packages
  info         Show information about a package
  install      Install packages
  uninstall    Uninstall packages
  upgrade      Upgrade installed packages
  list         List installed packages
  which        Show which package provides a binary
  pin          Pin a package to prevent upgrades
  unpin        Unpin a package to allow upgrades
  default      Set or show the default version for a package
  dependents   Show packages that depend on a package
  init         Initialize a new coldbrew.toml in the current directory
  lock         Generate a lockfile from coldbrew.toml
  tap          Add or remove taps (third-party repositories)
  space        Disk usage and cleanup commands
  link         Force-link a keg-only package
  unlink       Remove links for a package
  doctor       Check system for potential problems
  completions  Generate shell completions
  help         Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose  Enable verbose output
  -q, --quiet    Suppress non-error output
  -h, --help     Print help
  -V, --version  Print version
```
