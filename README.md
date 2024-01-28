# typst2ansi
typst2ansi highlights your Typst code.

It is especially intended to be used to provide the otherwise missing Typst syntax highlighting in Discord.

## Installation
```sh
cargo install --path .
```

## Usage
Simply run `typst2ansi main.typ` or pipe the source code into `typst2ansi`.
You could use `typst2ansi -d main.typ | xclip -selection clipboard` to copy Discord-compatible output to your clipboard.

```
Usage: typst2ansi [OPTIONS] [INPUT]

Arguments:
  [INPUT]  The input path. If unset, stdin is used

Options:
  -d, --discord  Whether the input should be formatted to be Discord-compatible
  -h, --help     Print help
```

## Legal
This software is not affiliated with Typst, the brand.
