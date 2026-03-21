pub mod code;
pub mod text;

use crate::parser::blocks::ContentBlock;
use gtk4 as gtk;

/// A trait for mapping an abstract `ContentBlock` into a native GTK Widget.
pub trait BlockRenderer {
    /// Returns true if this renderer knows how to handle the given block.
    fn can_render(&self, block: &ContentBlock) -> bool;

    /// Renders the block into a new GTK widget.
    fn render(&self, block: &ContentBlock) -> gtk::Widget;
}
