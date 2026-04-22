//! Outer shell of the Claw sidebar ("Claw" page in the right-hand
//! sidebar stack).
//!
//! This component is window-scoped: there is exactly one per
//! `AppWindow`, and it re-targets itself at whichever pane is currently
//! active by calling `set_history_widget(list_view, ...)`. The per-pane
//! list itself is built by `create_claw_message_list()` and owned by
//! the pane.
//!
//! The sidebar is a strictly read-only debug log. It renders identity,
//! token usage, a pending-tasks drawer, and a "Clear" button that
//! empties the visible list and emits a soft-clear to the engine.

use boxxy_claw_protocol::{ScheduledTask, TaskStatus, TaskType};
use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

pub struct ClawSidebarComponent {
    widget: gtk::Box,
    status_page: adw::StatusPage,
    scroll: gtk::ScrolledWindow,
    usage_lbl: gtk::Label,
    command_panel: gtk::Box,
    current_list: Rc<RefCell<Option<gtk::ListView>>>,
    tasks_expander: gtk::Expander,
    tasks_list: gtk::ListBox,
    on_cancel_task: Rc<dyn Fn(uuid::Uuid) + 'static>,
}

impl ClawSidebarComponent {
    #[must_use]
    pub fn new<FC: Fn(uuid::Uuid) + 'static, FS: Fn() + 'static>(
        on_cancel_task: FC,
        on_soft_clear: FS,
    ) -> Self {
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 6);
        widget.set_margin_top(6);
        widget.set_margin_bottom(6);
        widget.set_margin_start(6);
        widget.set_margin_end(6);

        let status_page = adw::StatusPage::builder()
            .title("Claw Mode")
            .description("System-eccentric agentic control.")
            .icon_name("boxxyclaw")
            .vexpand(true)
            .build();
        widget.append(&status_page);

        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .visible(false)
            .build();
        widget.append(&scroll);

        let current_list: Rc<RefCell<Option<gtk::ListView>>> = Rc::new(RefCell::new(None));

        let command_panel = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        command_panel.set_valign(gtk::Align::End);
        command_panel.set_margin_top(4);

        let usage_lbl = gtk::Label::builder()
            .label("Context: 0 tokens")
            .css_classes(["caption", "dim-label"])
            .hexpand(true)
            .halign(gtk::Align::Center)
            .visible(false)
            .build();
        command_panel.append(&usage_lbl);

        let clear_btn = gtk::Button::builder()
            .icon_name("boxxy-edit-clear-symbolic")
            .css_classes(["flat", "image-button"])
            .tooltip_text("Clear Screen")
            .halign(gtk::Align::End)
            .build();

        let current_list_clear = current_list.clone();
        let status_page_clone = status_page.clone();
        let scroll_clone = scroll.clone();
        clear_btn.connect_clicked(move |_| {
            // Drop visible rows. Engine still has the underlying history —
            // soft-clear marks a timestamp in the DB so restored sessions
            // skip pre-clear events.
            if let Some(list) = current_list_clear.borrow().as_ref() {
                if let Some(store) = list
                    .model()
                    .and_then(|m| m.downcast::<gtk::NoSelection>().ok())
                    .and_then(|s| s.model())
                    .and_then(|m| m.downcast::<gtk::gio::ListStore>().ok())
                {
                    store.remove_all();
                }
            }
            status_page_clone.set_visible(true);
            scroll_clone.set_visible(false);
            on_soft_clear();
        });
        command_panel.append(&clear_btn);
        widget.append(&command_panel);

        let tasks_list = gtk::ListBox::builder()
            .css_classes(["boxed-list"])
            .selection_mode(gtk::SelectionMode::None)
            .build();
        let tasks_expander = gtk::Expander::builder()
            .label("Pending Tasks")
            .child(&tasks_list)
            .visible(false)
            .build();
        widget.append(&tasks_expander);

        Self {
            widget,
            status_page,
            scroll,
            usage_lbl,
            command_panel,
            current_list,
            tasks_expander,
            tasks_list,
            on_cancel_task: Rc::new(on_cancel_task),
        }
    }

    pub fn set_history_widget(
        &self,
        list: &gtk::ListView,
        agent_name: &str,
        pinned: bool,
        web_search_enabled: bool,
    ) {
        if !agent_name.is_empty() {
            self.status_page.set_title(&format!("Claw: {}", agent_name));
            let mut desc = String::from("System-eccentric agentic control.");
            if pinned {
                desc.push_str(" (Pinned)");
            }
            if web_search_enabled {
                desc.push_str(" [Web Search ON]");
            }
            self.status_page.set_description(Some(&desc));
        }

        // Reuse path: same pane — nothing to re-parent.
        if let Some(old) = self.current_list.borrow().as_ref()
            && old == list
        {
            self.refresh_visibility();
            return;
        }

        if list.parent().is_some() {
            list.unparent();
        }

        // Virtual-list auto-scroll: only scroll if the user is already at
        // the bottom, so we don't yank them away from reading history.
        let adj = self.scroll.vadjustment();
        if let Some(store) = list
            .model()
            .and_then(|m| m.downcast::<gtk::NoSelection>().ok())
            .and_then(|s| s.model())
            .and_then(|m| m.downcast::<gtk::gio::ListStore>().ok())
        {
            let adj_clone = adj.clone();
            let list_v = list.clone();
            store.connect_items_changed(move |s, _, _, _| {
                let a = adj_clone.clone();
                let lv = list_v.clone();
                let n_items = s.n_items();
                gtk::glib::idle_add_local_once(move || {
                    let at_bottom = a.value() + a.page_size() >= a.upper() - 100.0;
                    if at_bottom && n_items > 0 {
                        lv.scroll_to(n_items - 1, gtk::ListScrollFlags::FOCUS, None);
                    }
                });
            });
        }

        self.scroll.set_child(Some(list));
        *self.current_list.borrow_mut() = Some(list.clone());
        self.refresh_visibility();
    }

    pub fn refresh_visibility(&self) {
        if let Some(list) = self.current_list.borrow().as_ref() {
            let has_items = list.model().map(|m| m.n_items() > 0).unwrap_or(false);
            self.status_page.set_visible(!has_items);
            self.scroll.set_visible(has_items);
            self.command_panel.set_visible(has_items);
        }
    }

    pub fn set_token_usage(&self, tokens: u64) {
        if tokens > 0 {
            self.usage_lbl
                .set_label(&format!("Context: {tokens} tokens"));
            self.usage_lbl.set_visible(true);
        } else {
            self.usage_lbl.set_visible(false);
        }
    }

    pub fn update_tasks(&self, tasks: Vec<ScheduledTask>) {
        while let Some(row) = self.tasks_list.row_at_index(0) {
            self.tasks_list.remove(&row);
        }

        let pending_count = tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .count();
        if pending_count == 0 {
            self.tasks_expander.set_visible(false);
            return;
        }

        self.tasks_expander
            .set_label(Some(&format!("Pending Tasks ({})", pending_count)));
        self.tasks_expander.set_visible(true);

        let on_cancel_rc = self.on_cancel_task.clone();
        for task in tasks {
            if task.status == TaskStatus::Pending {
                let cancel_cb = on_cancel_rc.clone();
                add_task_row(&self.tasks_list, task, move |id| cancel_cb(id));
            }
        }
    }

    #[must_use]
    pub const fn widget(&self) -> &gtk::Box {
        &self.widget
    }
}

impl Default for ClawSidebarComponent {
    fn default() -> Self {
        Self::new(|_| {}, || {})
    }
}

fn add_task_row<F: Fn(uuid::Uuid) + 'static>(
    list: &gtk::ListBox,
    task: ScheduledTask,
    on_cancel: F,
) {
    let row = gtk::ListBoxRow::new();
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 4);
    vbox.set_margin_top(6);
    vbox.set_margin_bottom(6);
    vbox.set_margin_start(6);
    vbox.set_margin_end(6);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let (icon_name, title) = match task.task_type {
        TaskType::Notification => ("boxxy-chat-symbolic", "Reminder"),
        TaskType::Command => ("utilities-terminal-symbolic", "Command"),
        TaskType::Query => ("help-browser-symbolic", "Query"),
    };

    header.append(&gtk::Image::from_icon_name(icon_name));

    let title_lbl = gtk::Label::builder()
        .label(title)
        .css_classes(["heading"])
        .halign(gtk::Align::Start)
        .hexpand(true)
        .build();
    header.append(&title_lbl);

    let cancel_btn = gtk::Button::builder()
        .icon_name("boxxy-cross-small-symbolic")
        .css_classes(["flat", "circular"])
        .tooltip_text("Cancel Task")
        .build();
    let task_id = task.id;
    cancel_btn.connect_clicked(move |_| on_cancel(task_id));
    header.append(&cancel_btn);
    vbox.append(&header);

    let payload_lbl = gtk::Label::builder()
        .label(&task.payload)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .xalign(0.0)
        .halign(gtk::Align::Start)
        .css_classes(["caption"])
        .build();
    vbox.append(&payload_lbl);

    let due_str = format!(
        "Due at: {}",
        task.due_at.with_timezone(&chrono::Local).format("%H:%M:%S")
    );
    let due_lbl = gtk::Label::builder()
        .label(&due_str)
        .xalign(0.0)
        .halign(gtk::Align::Start)
        .css_classes(["caption", "dim-label"])
        .build();
    vbox.append(&due_lbl);

    row.set_child(Some(&vbox));
    list.append(&row);
}
