# typst-ansi-hl
typst-ansi-hl highlights your Typst code using ANSI escape sequences.

It is especially intended to be used to provide the otherwise missing Typst syntax highlighting in Discord.

## Installation
```sh
cargo install --path .
```

## Usage
Simply run `typst-ansi-hl main.typ` or pipe the source code into `typst-ansi-hl`.
You could use `typst-ansi-hl --discord main.typ | xclip -selection clipboard` to copy Discord-compatible output to your clipboard.

```
Usage: typst-ansi-hl [OPTIONS] [INPUT]

Arguments:
  [INPUT]
          The input path. If unset, stdin is used

Options:
  -d, --discord
          Whether the input should be formatted to be Discord-compatible

  -l, --soft-limit <SOFT_LIMIT>
          Softly enforce a byte size limit.

          This means that if the size limit is exceeded, less colors are used in order to get below that size limit. If it is not possible to get below that limit, the text is printed anyway.

  -m, --mode <MODE>
          The kind of input syntax

          [default: markup]
          [possible values: code, markup, math]

  -h, --help
          Print help (see a summary with '-h')
```

### Clipboard-based Workflow
You can bind one of the following commands to a certain key bind for improved ease-of-use:
```sh
# Linux X11 (Bash/Zsh/Fish/Nushell)
xclip -selection clipboard -out | typst-ansi-hl --discord --soft-limit 2000 | xclip -selection clipboard -in

# Linux Wayland (Bash/Zsh/Fish/Nushell)
wl-paste | typst-ansi-hl --discord --soft-limit 2000 | wl-copy

# Windows (PowerShell)
Get-Clipboard | typst-ansi-hl --discord --soft-limit 2000 | Set-Clipboard
```

### Library
You can also use this crate as a library.
See the [documentation](https://docs.rs/typst-ansi-hl/latest) for further details.

## Legal
This software is not affiliated with Typst, the brand.
