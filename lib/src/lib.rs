//! `typst-ansi-hl` highlights your Typst code using ANSI escape sequences.
//!
//! ```
//! # use typst_ansi_hl::Highlighter;
//! let output = Highlighter::default()
//!     .for_discord()
//!     .with_soft_limit(2000)
//!     .highlight("This is _Typst_ #underline[code].");
//! ```
use std::io::Write;

use once_cell::sync::Lazy;
use syntect::{
    easy::HighlightLines, highlighting::FontStyle, parsing::SyntaxSet, util::LinesWithEndings,
};
use termcolor::{Color, ColorSpec, WriteColor};
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName};
use typst_syntax::{
    ast::{self, AstNode},
    LinkedNode, Tag,
};

/// Module with external dependencies exposed by this library.
pub mod ext {
    pub use syntect;
    pub use termcolor;
    pub use typst_syntax;
}

const ZERO_WIDTH_JOINER: char = '\u{200D}';

/// Any error returned by this library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("syntax mode is not one of `code`, `markup`, `math`")]
    UnknownMode,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Syntect(#[from] syntect::Error),
}

/// The kind of input syntax.
#[derive(Debug, Clone, Copy)]
pub enum SyntaxMode {
    Code,
    Markup,
    Math,
}

#[derive(Debug, Clone, Copy)]
pub struct Highlighter {
    discord: bool,
    syntax_mode: SyntaxMode,
    soft_limit: Option<usize>,
}

impl Default for Highlighter {
    fn default() -> Self {
        Highlighter {
            discord: false,
            syntax_mode: SyntaxMode::Markup,
            soft_limit: None,
        }
    }
}

impl Highlighter {
    /// Enable output specifically for Discord.
    ///
    /// If enabled, output will be surrounded by a `ansi` language code block.
    /// Additionally, any code blocks will be escaped.
    /// The output might not look like the input.
    ///
    /// Default: `false`.
    pub fn for_discord(&mut self) -> &mut Self {
        self.discord = true;
        self
    }

    /// When parsing, how the input should be interpreted.
    ///
    /// Default: [`SyntaxMode::Markup`].
    pub fn with_syntax_mode(&mut self, mode: SyntaxMode) -> &mut Self {
        self.syntax_mode = mode;
        self
    }

    /// Softly enforce a byte size limit.
    ///
    /// This means that if the size limit is exceeded, less colors are used
    /// in order to get below that size limit.
    /// If it is not possible to get below that limit, the text is printed anyway.
    pub fn with_soft_limit(&mut self, soft_limit: usize) -> &mut Self {
        self.soft_limit = Some(soft_limit);
        self
    }

    /// Highlight Typst code and return the highlighted string.
    pub fn highlight(&self, input: &str) -> Result<String, Error> {
        let mut out = termcolor::Ansi::new(Vec::new());
        self.highlight_to(input, &mut out)?;
        Ok(String::from_utf8(out.into_inner()).expect("the output should be entirely UTF-8"))
    }

    /// Highlight Typst code and write it to the given output.
    pub fn highlight_to<W: WriteColor>(&self, input: &str, out: W) -> Result<(), Error> {
        let parsed = match self.syntax_mode {
            SyntaxMode::Code => typst_syntax::parse_code(input),
            SyntaxMode::Markup => typst_syntax::parse(input),
            SyntaxMode::Math => typst_syntax::parse_math(input),
        };
        let linked = typst_syntax::LinkedNode::new(&parsed);
        self.highlight_node_to(&linked, out)
    }

    /// Highlight a linked syntax node and write it to the given output.
    ///
    /// Use [`typst_syntax::parse`] to parse a string into a [`SyntaxNode`], and then
    /// use [`LinkedNode::new`] on the parsed syntax node to obtain a [`LinkedNode`]
    /// you can use with this function.
    ///
    /// [`SyntaxNode`]: typst_syntax::SyntaxNode
    pub fn highlight_node_to<W: WriteColor>(
        &self,
        node: &LinkedNode,
        mut out: W,
    ) -> Result<(), Error> {
        fn inner_highlight_node<W: WriteColor>(
            highlighter: &Highlighter,
            hl_level: HighlightLevel,
            node: &LinkedNode,
            out: &mut DeferredWriter<W>,
            color: &mut ColorSpec,
        ) -> Result<(), Error> {
            let prev_color = color.clone();

            if let Some(tag) = typst_syntax::highlight(node) {
                out.set_color(&highlighter.tag_to_color(hl_level, tag))?;
            }

            if let Some(raw) = ast::Raw::from_untyped(node) {
                highlighter.highlight_raw(hl_level, out, raw)?;
            } else if node.text().is_empty() {
                for child in node.children() {
                    inner_highlight_node(highlighter, hl_level, &child, out, color)?;
                }
            } else {
                write!(out, "{}", node.text())?;
            }

            out.set_color(&prev_color)?;
            *color = prev_color;

            Ok(())
        }

        fn inner<W: WriteColor>(
            highlighter: &Highlighter,
            node: &LinkedNode,
            out: W,
            hl_level: HighlightLevel,
        ) -> Result<(), Error> {
            let mut out = DeferredWriter::new(out);
            if highlighter.discord {
                writeln!(out, "```ansi")?;
            }

            inner_highlight_node(highlighter, hl_level, node, &mut out, &mut ColorSpec::new())?;

            if highlighter.discord {
                // Make sure that the closing fences are on their own line.
                let mut last_leaf = node.clone();
                while let Some(child) = last_leaf.children().last() {
                    last_leaf = child;
                }
                if !last_leaf.text().ends_with('\n') {
                    writeln!(out)?;
                }
                writeln!(out, "```")?;
            }
            Ok(())
        }

        if let Some(soft_limit) = self.soft_limit {
            // Because a soft limit is given, we highlight everything to an in-memory buffer
            // and check whether the output length is less than the limit.
            // If the limit was reached, we lower the highlight level.
            // Otherwise, we write it to the real output.
            // If the highlight level was reached, we _always_ write the output without highlighting.
            let mut buf_out = termcolor::Ansi::new(Vec::new());
            let mut level = HighlightLevel::All;
            loop {
                inner(self, node, &mut buf_out, level)?;
                let mut buf = buf_out.into_inner();
                if buf.len() < soft_limit || level == HighlightLevel::Off {
                    out.write_all(&buf)?;
                    break;
                } else {
                    buf.clear();
                    buf_out = termcolor::Ansi::new(buf);
                    level = level.restrict();
                }
            }
        } else {
            inner(self, node, out, HighlightLevel::All)?;
        }

        Ok(())
    }

    fn highlight_raw<W: WriteColor>(
        &self,
        hl_level: HighlightLevel,
        out: &mut DeferredWriter<W>,
        raw: ast::Raw<'_>,
    ) -> Result<(), Error> {
        let text = raw.to_untyped().clone().into_text();

        // Collect backticks and escape if discord is enabled.
        let backticks: String = text.chars().take_while(|&c| c == '`').collect();
        let (fence, is_pure_fence) = {
            if self.discord && backticks.len() >= 3 {
                let mut fence: String = backticks
                    .chars()
                    .flat_map(|c| [c, ZERO_WIDTH_JOINER])
                    .collect();
                fence.pop();
                (fence, false)
            } else {
                (backticks, true)
            }
        };

        // Write opening fence.
        if self.discord && !is_pure_fence {
            out.set_color(&self.tag_to_color(hl_level, Tag::Comment))?;
            write!(out, "/* when copying, remove and retype these --> */")?;
        }
        out.set_color(&self.tag_to_color(hl_level, Tag::Raw))?;
        write!(out, "{fence}")?;

        if let Some(lang) = raw.lang() {
            write!(out, "{}", lang.get())?;
        }

        // Trim starting fences.
        let mut inner = text.trim_start_matches('`');
        // Trim closing fences.
        inner = &inner[..inner.len() - (text.len() - inner.len())];

        if let Some(lang) = raw.lang().filter(|_| hl_level >= HighlightLevel::WithRaw) {
            let lang = lang.get();
            inner = &inner[lang.len()..]; // Trim language tag.
            highlight_lang(inner, lang, out)?;
        } else {
            write!(out, "{inner}")?;
        }

        // Write closing fence.
        out.set_color(&self.tag_to_color(hl_level, Tag::Raw))?;
        write!(out, "{fence}")?;
        if self.discord && !is_pure_fence {
            out.set_color(&self.tag_to_color(hl_level, Tag::Comment))?;
            write!(out, "/* <-- when copying, remove and retype these */")?;
        }

        Ok(())
    }

    fn tag_to_color(&self, hl_level: HighlightLevel, tag: Tag) -> ColorSpec {
        let mut color = ColorSpec::default();
        let l1 = hl_level >= HighlightLevel::L1;
        let l2 = hl_level >= HighlightLevel::L2;
        let with_styles = hl_level >= HighlightLevel::WithStyles;
        match tag {
            Tag::Comment => {
                if self.discord {
                    color.set_fg(Some(Color::Black))
                } else {
                    color.set_dimmed(true)
                }
            }
            Tag::Punctuation if l1 => color.set_fg(None),
            Tag::Escape => color.set_fg(Some(Color::Cyan)),
            Tag::Strong if l1 => color.set_fg(Some(Color::Yellow)).set_bold(with_styles),
            Tag::Emph if l1 => color.set_fg(Some(Color::Yellow)).set_italic(with_styles),
            Tag::Link if l1 => color.set_fg(Some(Color::Blue)).set_underline(with_styles),
            Tag::Raw => color.set_fg(Some(Color::White)),
            Tag::Label => color.set_fg(Some(Color::Blue)).set_underline(with_styles),
            Tag::Ref => color.set_fg(Some(Color::Blue)).set_underline(with_styles),
            Tag::Heading => color.set_fg(Some(Color::Cyan)).set_bold(with_styles),
            Tag::ListMarker => color.set_fg(Some(Color::Cyan)),
            Tag::ListTerm => color.set_fg(Some(Color::Cyan)),
            Tag::MathDelimiter if l2 => color.set_fg(Some(Color::Cyan)),
            Tag::MathOperator => color.set_fg(Some(Color::Cyan)),
            Tag::Keyword => color.set_fg(Some(Color::Magenta)),
            Tag::Operator if l2 => color.set_fg(Some(Color::Cyan)),
            Tag::Number => color.set_fg(Some(Color::Yellow)),
            Tag::String => color.set_fg(Some(Color::Green)),
            Tag::Function if l2 => color.set_fg(Some(Color::Blue)).set_italic(with_styles),
            Tag::Interpolated if l2 => color.set_fg(Some(Color::White)),
            Tag::Error => color.set_fg(Some(Color::Red)),
            _ => &mut color,
        };
        color
    }
}

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(two_face::syntax::extra_newlines);
static THEME_SET: Lazy<EmbeddedLazyThemeSet> = Lazy::new(two_face::theme::extra);

fn highlight_lang<W: WriteColor>(
    input: &str,
    lang: &str,
    out: &mut DeferredWriter<W>,
) -> Result<(), Error> {
    let Some(syntax) = SYNTAX_SET.find_syntax_by_token(lang) else {
        write!(out, "{input}")?;
        return Ok(());
    };
    let ansi_theme = THEME_SET.get(EmbeddedThemeName::Base16);

    let mut highlighter = HighlightLines::new(syntax, ansi_theme);
    for line in LinesWithEndings::from(input) {
        let ranges = highlighter.highlight_line(line, &SYNTAX_SET)?;
        for (styles, text) in ranges {
            let fg = styles.foreground;
            let fg = convert_rgb_to_ansi_color(fg.r, fg.g, fg.b, fg.a);
            let mut color = ColorSpec::new();
            color.set_fg(fg);

            let font_style = styles.font_style;
            color.set_bold(font_style.contains(FontStyle::BOLD));
            color.set_italic(font_style.contains(FontStyle::ITALIC));
            color.set_underline(font_style.contains(FontStyle::UNDERLINE));

            out.set_color(&color)?;
            write!(out, "{text}")?;
        }
    }

    Ok(())
}

/// Converts an RGB color from the theme to a [`Color`].
///
/// Inspired by an equivalent function in `bat`[^1].
/// [^1]: https://github.com/sharkdp/bat/blob/07c26adc357f70a48f2b412008d5c37d43e084c5/src/terminal.rs#L6
fn convert_rgb_to_ansi_color(r: u8, g: u8, b: u8, a: u8) -> Option<Color> {
    match a {
        0 => Some(match r {
            // Use predefined colors for wider support.
            0x00 => Color::Black,
            0x01 => Color::Red,
            0x02 => Color::Green,
            0x03 => Color::Yellow,
            0x04 => Color::Blue,
            0x05 => Color::Magenta,
            0x06 => Color::Cyan,
            0x07 => Color::White,
            _ => Color::Ansi256(r),
        }),
        1 => None,
        _ => Some(Color::Ansi256(ansi_colours::ansi256_from_rgb((r, g, b)))),
    }
}

/// What things to highlight.
/// Lower values mean less highlighting.
///
/// Used when a soft limit is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HighlightLevel {
    Off,
    L1,
    L2,
    /// Highlight raw blocks.
    WithRaw,
    /// Use styles like bold, italic, underline.
    WithStyles,
    All,
}

impl HighlightLevel {
    fn restrict(self) -> HighlightLevel {
        match self {
            HighlightLevel::Off => HighlightLevel::Off,
            HighlightLevel::L1 => HighlightLevel::Off,
            HighlightLevel::L2 => HighlightLevel::L1,
            HighlightLevel::WithRaw => HighlightLevel::L2,
            HighlightLevel::WithStyles => HighlightLevel::WithRaw,
            HighlightLevel::All => HighlightLevel::WithStyles,
        }
    }
}

/// A writer that only sets the color when content is written.
/// This is intended to lessen the size impact of unnecessary escape codes.
struct DeferredWriter<W> {
    inner: W,
    current_color: ColorSpec,
    next_color: Option<ColorSpec>,
}

impl<W> DeferredWriter<W> {
    fn new(writer: W) -> DeferredWriter<W> {
        DeferredWriter {
            inner: writer,
            current_color: ColorSpec::new(),
            next_color: None,
        }
    }
}

impl<W: WriteColor> Write for DeferredWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(color) = self.next_color.take() {
            self.inner.set_color(&color)?;
            self.current_color = color;
        }
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl<W: WriteColor> WriteColor for DeferredWriter<W> {
    fn supports_color(&self) -> bool {
        self.inner.supports_color()
    }

    fn set_color(&mut self, spec: &ColorSpec) -> std::io::Result<()> {
        if &self.current_color == spec {
            self.next_color = None;
        } else {
            self.next_color = Some(spec.clone());
        }
        Ok(())
    }

    fn reset(&mut self) -> std::io::Result<()> {
        let mut color = ColorSpec::new();
        color.set_reset(true);
        self.next_color = Some(color);
        Ok(())
    }

    fn is_synchronous(&self) -> bool {
        self.inner.is_synchronous()
    }

    fn set_hyperlink(&mut self, link: &termcolor::HyperlinkSpec) -> std::io::Result<()> {
        self.inner.set_hyperlink(link)
    }

    fn supports_hyperlinks(&self) -> bool {
        self.inner.supports_hyperlinks()
    }
}
