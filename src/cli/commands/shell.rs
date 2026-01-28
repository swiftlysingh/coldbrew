//! Shell command - set up shell integration

use crate::cli::output::Output;
use crate::error::Result;
use crate::storage::Paths;

/// Execute the shell command
pub async fn execute(shell: Option<&str>, output: &Output) -> Result<()> {
    let paths = Paths::new()?;
    let bin_dir = paths.bin_dir();

    let shell_name = shell.unwrap_or_else(|| detect_shell());

    output.info(&format!("Shell integration for {}", shell_name));
    println!();

    let add_path = format!(
        r#"# Add Coldbrew to PATH
export PATH="{}:$PATH""#,
        bin_dir.display()
    );

    match shell_name {
        "bash" => {
            println!("Add the following to your ~/.bashrc or ~/.bash_profile:\n");
            println!("{}", add_path);
            println!();
            output.hint("Then restart your shell or run: source ~/.bashrc");
        }
        "zsh" => {
            println!("Add the following to your ~/.zshrc:\n");
            println!("{}", add_path);
            println!();
            output.hint("Then restart your shell or run: source ~/.zshrc");
        }
        "fish" => {
            println!("Add the following to your ~/.config/fish/config.fish:\n");
            println!(
                "# Add Coldbrew to PATH\nfish_add_path {}",
                bin_dir.display()
            );
            println!();
            output.hint("Then restart your shell or run: source ~/.config/fish/config.fish");
        }
        _ => {
            output.warning(&format!("Unknown shell: {}", shell_name));
            println!("Add {} to your PATH", bin_dir.display());
        }
    }

    Ok(())
}

/// Detect the current shell
fn detect_shell() -> &'static str {
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("zsh") {
            return "zsh";
        } else if shell.contains("fish") {
            return "fish";
        } else if shell.contains("bash") {
            return "bash";
        }
    }
    "bash"
}
