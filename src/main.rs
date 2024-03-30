use std::{io::Read, path::PathBuf};

use clap::{Parser, ValueEnum};
use color_eyre::eyre::{Context as _, Result};
use typst_ansi_hl::Highlighter;

#[derive(clap::Parser)]
struct Args {
    /// The input path. If unset, stdin is used.
    input: Option<PathBuf>,

    /// Whether the input should be formatted to be Discord-compatible.
    #[clap(short, long)]
    discord: bool,

    /// Softly enforce a byte size limit.
    ///
    /// This means that if the size limit is exceeded, less colors are used
    /// in order to get below that size limit.
    /// If it is not possible to get below that limit, the text is printed anyway.
    #[clap(short = 'l', long)]
    soft_limit: Option<usize>,

    /// The kind of input syntax.
    #[clap(short, long, default_value = "markup")]
    mode: SyntaxMode,
}

/// The kind of input syntax.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SyntaxMode {
    Code,
    Markup,
    Math,
}

impl From<SyntaxMode> for typst_ansi_hl::SyntaxMode {
    fn from(value: SyntaxMode) -> Self {
        match value {
            SyntaxMode::Code => typst_ansi_hl::SyntaxMode::Code,
            SyntaxMode::Markup => typst_ansi_hl::SyntaxMode::Markup,
            SyntaxMode::Math => typst_ansi_hl::SyntaxMode::Math,
        }
    }
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
    highlighter.with_syntax_mode(args.mode.into());
    if let Some(soft_limit) = args.soft_limit {
        highlighter.with_soft_limit(soft_limit);
    }
    highlighter
        .highlight_to(&input, out)
        .wrap_err("failed to highlight input")?;

    Ok(())
}
