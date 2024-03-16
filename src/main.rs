use std::{io::Read, path::PathBuf};

use clap::Parser;
use color_eyre::eyre::{Context as _, Result};
use typst_ansi_hl::{Highlighter, SyntaxMode};

#[derive(clap::Parser)]
struct Args {
    /// The input path. If unset, stdin is used.
    input: Option<PathBuf>,

    /// Whether the input should be formatted to be Discord-compatible.
    #[clap(short, long)]
    discord: bool,

    /// The kind of input syntax.
    #[clap(short, long, default_value_t = SyntaxMode::Markup)]
    mode: SyntaxMode,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();
    let mut input = String::new();
    if let Some(path) = &args.input {
        std::fs::File::open(path)
            .wrap_err_with(|| format!("failed to open file `{}`", path.display()))?
            .read_to_string(&mut input)
            .wrap_err_with(|| format!("failed to read file `{}`", path.display()))?;
    } else {
        std::io::stdin()
            .read_to_string(&mut input)
            .wrap_err("failed to read from stdin")?;
    }

    let out = termcolor::Ansi::new(std::io::stdout().lock());
    let mut highlighter = Highlighter::default();
    if args.discord {
        highlighter.for_discord();
    }
    highlighter
        .highlight_to(&input, out)
        .wrap_err("failed to highlight input")?;

    Ok(())
}
