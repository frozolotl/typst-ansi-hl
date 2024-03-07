use std::{
    io::{Read, Write},
    path::PathBuf,
};

use clap::Parser;
use color_eyre::eyre::{Context as _, Result};
use termcolor::ColorSpec;
use typst_ansi_hl_lib::{highlight, DeferredWriter, Options};

#[derive(clap::Parser)]
struct Args {
    /// The input path. If unset, stdin is used.
    input: Option<PathBuf>,

    /// Whether the input should be formatted to be Discord-compatible.
    #[clap(short, long)]
    discord: bool,
}

struct Context {
    args: Args,
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

    let ctx = Context { args };

    let mut out = DeferredWriter::new(termcolor::Ansi::new(std::io::stdout().lock()));

    if ctx.args.discord {
        writeln!(out, "```ansi")?;
    }

    let parsed = typst_syntax::parse(&input);
    let highlight_options = Options {
        discord: ctx.args.discord,
    };
    highlight(
        &highlight_options,
        &mut out,
        &mut ColorSpec::new(),
        &typst_syntax::LinkedNode::new(&parsed),
    )
    .wrap_err("failed to highlight input")?;

    if ctx.args.discord {
        writeln!(out, "```")?;
    }

    Ok(())
}
