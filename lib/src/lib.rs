use std::io::Write;
use termcolor::{Color, ColorSpec, WriteColor};
use typst_syntax::ast::AstNode;
use typst_syntax::{ast, LinkedNode, Tag};

/// Module with external dependencies exposed by this library.
pub mod ext {
    pub use bat;
    pub use termcolor;
    pub use typst_syntax;
}

const ZERO_WIDTH_JOINER: char = '\u{200D}';

/// Highlight options.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Options {
    /// Whether the output will be used in Discord.
    /// This results in backtick escaping and other conveniences.
    /// One will still have to manually add the \`\`\`ansi \`\`\` fences around
    /// the output.
    pub discord: bool,
}

/// Any error returned by this library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Bat(#[from] bat::error::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Highlight a Typst linked node.
///
/// Use [`typst_syntax::parse`] to parse a string into a [`SyntaxNode`], and then
/// use [`LinkedNode::new`] on the parsed syntax node to obtain a [`LinkedNode`]
/// you can use with this function.
///
/// [`SyntaxNode`]: typst_syntax::SyntaxNode
pub fn highlight<W: WriteColor>(
    options: &Options,
    out: &mut DeferredWriter<W>,
    color: &mut ColorSpec,
    node: &LinkedNode,
) -> Result<()> {
    let prev_color = color.clone();

    if let Some(tag) = typst_syntax::highlight(node) {
        *color = ColorSpec::default();
        match tag {
            Tag::Comment => color.set_fg(Some(Color::Black)).set_dimmed(true),
            Tag::Punctuation => color.set_fg(None),
            Tag::Escape => color.set_fg(Some(Color::Cyan)),
            Tag::Strong => color.set_fg(Some(Color::Yellow)).set_bold(true),
            Tag::Emph => color.set_fg(Some(Color::Yellow)).set_italic(true),
            Tag::Link => color.set_fg(Some(Color::Blue)).set_underline(true),
            Tag::Raw => color, // This is handled within [`highlight_raw`].
            Tag::Label => color.set_fg(Some(Color::Blue)).set_underline(true),
            Tag::Ref => color.set_fg(Some(Color::Blue)).set_underline(true),
            Tag::Heading => color.set_fg(Some(Color::Cyan)).set_bold(true),
            Tag::ListMarker => color.set_fg(Some(Color::Cyan)),
            Tag::ListTerm => color.set_fg(Some(Color::Cyan)),
            Tag::MathDelimiter => color.set_fg(Some(Color::Cyan)),
            Tag::MathOperator => color.set_fg(Some(Color::Cyan)),
            Tag::Keyword => color.set_fg(Some(Color::Magenta)),
            Tag::Operator => color.set_fg(Some(Color::Cyan)),
            Tag::Number => color.set_fg(Some(Color::Yellow)),
            Tag::String => color.set_fg(Some(Color::Green)),
            Tag::Function => color.set_fg(Some(Color::Blue)).set_italic(true),
            Tag::Interpolated => color.set_fg(Some(Color::White)),
            Tag::Error => color.set_fg(Some(Color::Red)),
        };
        out.set_color(color)?;
    }

    if let Some(raw) = ast::Raw::from_untyped(node) {
        highlight_raw(options, out, raw)?;
    } else if node.text().is_empty() {
        for child in node.children() {
            highlight(options, out, color, &child)?;
        }
    } else {
        write!(out, "{}", node.text())?;
    }

    out.set_color(&prev_color)?;
    *color = prev_color;

    Ok(())
}

fn highlight_raw<W: WriteColor>(
    options: &Options,
    out: &mut DeferredWriter<W>,
    raw: ast::Raw<'_>,
) -> Result<()> {
    let mut color = ColorSpec::new();
    color.set_fg(Some(Color::White));

    let text = raw.to_untyped().text();

    // Collect backticks and escape if discord is enabled.
    let fence: String = {
        let backticks = text.chars().take_while(|&c| c == '`');
        if options.discord {
            let mut fence: String = backticks.flat_map(|c| [c, ZERO_WIDTH_JOINER]).collect();
            fence.pop();
            fence
        } else {
            backticks.collect()
        }
    };

    // Write opening fence.
    out.set_color(&color)?;
    write!(out, "{fence}")?;

    let lang = raw.lang().unwrap_or("");
    write!(out, "{}", lang)?;

    // Trim starting fences.
    let mut inner = text.trim_start_matches('`');
    // Trim closing fences.
    inner = &inner[..inner.len() - (text.len() - inner.len())];
    // Trim language.
    inner = &inner[lang.len()..];

    if raw.lang().is_some() {
        bat::PrettyPrinter::new()
            .input_from_bytes(inner.as_bytes())
            .language(lang)
            .theme("ansi")
            .print()?;
    } else {
        write!(out, "{inner}")?;
    }

    // HACK: Reset the color the writer thinks it has.
    // Necessary because [`bat::PrettyPrinter`] does not use [`out`].
    out.current_color = ColorSpec::default();
    out.set_color(&color)?;

    // Write closing fence.
    write!(out, "{fence}")?;

    Ok(())
}

/// A writer that only sets the color when content is written.
/// This is intended to lessen the size impact of unnecessary escape codes.
pub struct DeferredWriter<W> {
    pub inner: W,
    current_color: ColorSpec,
    next_color: Option<ColorSpec>,
}

impl<W> DeferredWriter<W> {
    /// Create a deferred writer.
    /// Use this with the highlight functions.
    pub fn new(writer: W) -> DeferredWriter<W> {
        DeferredWriter {
            inner: writer,
            current_color: ColorSpec::new(),
            next_color: None,
        }
    }

    /// Extracts the inner writer from this DeferredWriter.
    pub fn into_inner(self) -> W {
        self.inner
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
