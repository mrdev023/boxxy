/// The Abstract Syntax Tree (AST) representing structured content.
///
/// This enum represents a sequence of logical visual blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentBlock {
    /// A header element with a level (1-6) and its Pango-formatted markup.
    Heading { level: u8, markup: String },

    /// A standard paragraph of text, already formatted with Pango markup
    /// (e.g. `<b>`, `<i>`, `<tt>`). Escaping of raw text has already occurred.
    Paragraph(String),

    /// A blockquote containing Pango-formatted markup.
    Blockquote(String),

    /// A list, either ordered or unordered, containing items formatted with Pango markup.
    List { ordered: bool, items: Vec<String> },

    /// A fenced code block. `code` is preserved *raw* (unescaped)
    /// to ensure copy-to-clipboard functionality works properly.
    Code { lang: String, code: String },

    /// Out-of-band structured data (like a tool execution result).
    /// `schema` defines what kind of data it is (e.g., "list_processes"),
    /// and `raw_payload` contains the stringified JSON or raw data.
    Custom { schema: String, raw_payload: String },
}
