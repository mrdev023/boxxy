use crate::types::{ChatMessage, Role};
use boxxy_viewer::{StructuredViewer, ViewerRegistry};
use gtk::prelude::*;
use gtk4 as gtk;
use std::rc::Rc;

pub fn build_message_widget(msg: &ChatMessage) -> gtk::Box {
    let registry = Rc::new(ViewerRegistry::new_with_defaults());
    let viewer = StructuredViewer::new(registry);

    // We use set_content here as the sidebar currently receives full messages.
    // In the future, we can change AiSidebarComponent to use append_markdown_stream for real-time updates.
    viewer.set_content(&msg.content);

    let container = gtk::Box::new(gtk::Orientation::Vertical, 6);
    container.set_margin_top(4);
    container.set_margin_bottom(4);

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    let bubble_box = viewer.widget();
    bubble_box.add_css_class("message-bubble");

    if msg.role == Role::User {
        row.set_halign(gtk::Align::End);
        row.set_margin_start(48);
        row.set_margin_end(8);
        bubble_box.add_css_class("user-message");
    } else {
        row.set_halign(gtk::Align::Start);
        row.set_margin_start(8);
        row.set_margin_end(48);
        bubble_box.add_css_class("assistant-message");
    }

    row.append(bubble_box);
    container.append(&row);

    container
}
