pub mod parser;
pub mod registry;
pub mod renderer;
pub mod widget;

pub use parser::blocks::ContentBlock;
pub use parser::markdown::{escape_pango, parse_markdown};
pub use registry::ViewerRegistry;
pub use renderer::BlockRenderer;
pub use widget::StructuredViewer;
