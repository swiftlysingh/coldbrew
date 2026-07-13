import Foundation
import ArgumentParser
import ColdbrewKit

struct NotImplemented: Error, CustomStringConvertible {
    let command: String

    var description: String {
        "\(command) is not implemented in the Swift migration yet"
    }
}

struct GlobalOutputOptions: ParsableArguments {
    @Flag(name: .shortAndLong, help: "Enable verbose output")
    var verbose = false

    @Flag(name: .shortAndLong, help: "Suppress non-error output")
    var quiet = false
}

func notImplemented(_ command: String, options: GlobalOutputOptions) throws {
    if options.verbose {
        FileHandle.standardError.write(Data("[verbose] \(command)\n".utf8))
    }
    throw NotImplemented(command: command)
}

enum CompletionShell: String, ExpressibleByArgument {
    case bash
    case elvish
    case fish
    case powershell
    case zsh
}

@main
struct Crew: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "crew",
        abstract: "Coldbrew - A Homebrew-compatible package manager",
        version: ColdbrewKit.version,
        subcommands: [
            Update.self,
            Search.self,
            Info.self,
            Install.self,
            Uninstall.self,
            Upgrade.self,
            List.self,
            Which.self,
            Pin.self,
            Unpin.self,
            Default.self,
            Dependents.self,
            Init.self,
            Lock.self,
            Tap.self,
            Space.self,
            Link.self,
            Unlink.self,
            Doctor.self,
            Completions.self,
            Exec.self,
        ]
    )

    @OptionGroup var output: GlobalOutputOptions

    func run() throws {
        throw CleanExit.helpRequest(self)
    }
}

struct Update: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Update the package index from Homebrew")

    @OptionGroup var output: GlobalOutputOptions

    func run() throws {
        try notImplemented("update", options: output)
    }
}

struct Search: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Search for packages")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Search query")
    var query: String

    @Flag(name: .shortAndLong, help: "Show extended information")
    var extended = false

    func run() throws {
        try notImplemented("search", options: output)
    }
}

struct Info: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Show information about a package")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Package name")
    var package: String

    @Option(name: .shortAndLong, help: "Output format (text, json)")
    var format = "text"

    func run() throws {
        try notImplemented("info", options: output)
    }
}

struct Install: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Install packages")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Packages to install (e.g., jq, node@22)")
    var packages: [String]

    @Flag(name: .long, help: "Install from coldbrew.lock instead of individual packages")
    var lock = false

    @Flag(name: .long, help: "Skip dependency installation")
    var skipDeps = false

    @Flag(name: .shortAndLong, help: "Force reinstall even if already installed")
    var force = false

    func validate() throws {
        if lock && !packages.isEmpty {
            throw ValidationError("--lock cannot be used with package arguments")
        }
        if packages.isEmpty && !lock {
            throw ValidationError("Missing expected argument '<packages>'")
        }
    }

    func run() throws {
        try notImplemented("install", options: output)
    }
}

struct Uninstall: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Uninstall packages")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Packages to uninstall")
    var packages: [String]

    @Flag(name: .shortAndLong, help: "Remove all versions")
    var all = false

    @Flag(name: .long, help: "Also remove unused dependencies")
    var withDeps = false

    func validate() throws {
        if packages.isEmpty {
            throw ValidationError("Missing expected argument '<packages>'")
        }
    }

    func run() throws {
        try notImplemented("uninstall", options: output)
    }
}

struct Upgrade: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Upgrade installed packages")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Packages to upgrade (all if not specified)")
    var packages: [String] = []

    @Flag(name: .shortAndLong, help: "Skip interactive selection")
    var yes = false

    func run() throws {
        try notImplemented("upgrade", options: output)
    }
}

struct List: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "List installed packages")

    @OptionGroup var output: GlobalOutputOptions

    @Flag(name: .shortAndLong, help: "Show only package names")
    var namesOnly = false

    @Option(name: .long, help: "Show versions for a specific package")
    var versions: String?

    func run() throws {
        try notImplemented("list", options: output)
    }
}

struct Which: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Show which package provides a binary")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Binary name")
    var binary: String

    func run() throws {
        try notImplemented("which", options: output)
    }
}

struct Pin: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Pin a package to prevent upgrades")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Package to pin")
    var package: String

    func run() throws {
        try notImplemented("pin", options: output)
    }
}

struct Unpin: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Unpin a package to allow upgrades")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Package to unpin")
    var package: String

    func run() throws {
        try notImplemented("unpin", options: output)
    }
}

struct Default: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Set or show the default version for a package")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Package name (e.g., node@22 or just node to show current)")
    var package: String

    func run() throws {
        try notImplemented("default", options: output)
    }
}

struct Dependents: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Show packages that depend on a package")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Package name")
    var package: String

    func run() throws {
        try notImplemented("dependents", options: output)
    }
}

struct Init: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Initialize a new coldbrew.toml in the current directory")

    @OptionGroup var output: GlobalOutputOptions

    @Flag(name: .shortAndLong, help: "Force overwrite if file exists")
    var force = false

    func run() throws {
        try notImplemented("init", options: output)
    }
}

struct Lock: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Generate a lockfile from coldbrew.toml")

    @OptionGroup var output: GlobalOutputOptions

    func run() throws {
        try notImplemented("lock", options: output)
    }
}

struct Tap: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Add or remove taps (third-party repositories)")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Tap to add (user/repo format)")
    var tap: String?

    @Flag(name: .shortAndLong, help: "Remove a tap instead of adding")
    var remove = false

    func run() throws {
        try notImplemented("tap", options: output)
    }
}

struct Space: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Disk usage and cleanup commands",
        subcommands: [Show.self, Clean.self]
    )

    @OptionGroup var output: GlobalOutputOptions

    func run() throws {
        try notImplemented("space", options: output)
    }

    struct Show: ParsableCommand {
        static let configuration = CommandConfiguration(abstract: "Show disk usage and cleanup candidates")

        @OptionGroup var output: GlobalOutputOptions

        @Flag(name: .shortAndLong, help: "Show itemized details")
        var details = false

        func run() throws {
            try notImplemented("space show", options: output)
        }
    }

    struct Clean: ParsableCommand {
        static let configuration = CommandConfiguration(abstract: "Cleanup old versions, cache, and other unused data")

        @OptionGroup var output: GlobalOutputOptions

        @Flag(name: .shortAndLong, help: "Clean everything without prompts")
        var all = false

        @Flag(name: .shortAndLong, help: "Dry run - show what would be removed")
        var dryRun = false

        func run() throws {
            try notImplemented("space clean", options: output)
        }
    }
}

struct Link: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Force-link a keg-only package")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Package to link")
    var package: String

    @Flag(name: .shortAndLong, help: "Force overwrite existing files")
    var force = false

    func run() throws {
        try notImplemented("link", options: output)
    }
}

struct Unlink: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Remove links for a package")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Package to unlink")
    var package: String

    func run() throws {
        try notImplemented("unlink", options: output)
    }
}

struct Doctor: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Check system for potential problems")

    @OptionGroup var output: GlobalOutputOptions

    func run() throws {
        try notImplemented("doctor", options: output)
    }
}

struct Completions: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Generate shell completions")

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Shell to generate completions for")
    var shell: CompletionShell

    func run() throws {
        try notImplemented("completions", options: output)
    }
}

struct Exec: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Execute a binary from a package (internal use)",
        shouldDisplay: false
    )

    @OptionGroup var output: GlobalOutputOptions

    @Argument(help: "Package name")
    var package: String

    @Argument(help: "Binary name")
    var binary: String

    @Argument(parsing: .unconditionalRemaining, help: "Arguments to pass to the binary")
    var args: [String] = []

    func run() throws {
        try notImplemented("exec", options: output)
    }
}
