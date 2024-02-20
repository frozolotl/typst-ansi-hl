# typst-ansi-hl
typst-ansi-hl highlights your Typst code using ANSI escape sequences.

It is especially intended to be used to provide the otherwise missing Typst syntax highlighting in Discord.

## Installation
```sh
cargo install --path .
```

## Usage
Simply run `typst-ansi-hl main.typ` or pipe the source code into `typst-ansi-hl`.
You could use `typst-ansi-hl -d main.typ | xclip -selection clipboard` to copy Discord-compatible output to your clipboard.

```
Usage: typst-ansi-hl [OPTIONS] [INPUT]

Arguments:
  [INPUT]  The input path. If unset, stdin is used

Options:
  -d, --discord  Whether the input should be formatted to be Discord-compatible
  -h, --help     Print help
```

### Clipboard-based Workflow
You can bind one of the following commands to a certain key bind for improved ease-of-use:
```sh
# Linux X11 (Bash/Zsh/Fish/Nushell)
xclip -selection clipboard -out | typst-ansi-hl --discord | xclip -selection clipboard -in

# Windows (PowerShell)
Get-Clipboard | typst-ansi-hl --discord | Set-Clipboard
```

## Legal
This software is not affiliated with Typst, the brand.
