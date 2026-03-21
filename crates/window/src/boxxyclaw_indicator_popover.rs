use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;
use std::rc::Rc;

pub struct BoxxyclawIndicatorPopover {
    popover: gtk::Popover,
    enable_btn: gtk::Switch,
    proactive_btn: gtk::Switch,
}

impl BoxxyclawIndicatorPopover {
    pub fn new<F1: Fn(bool) + 'static, F2: Fn(bool) + 'static>(
        on_enable_toggled: F1,
        on_proactive_toggled: F2,
    ) -> Self {
        let popover = gtk::Popover::new();
        popover.add_css_class("menu");

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.set_margin_top(8);
        vbox.set_margin_bottom(8);
        vbox.set_margin_start(8);
        vbox.set_margin_end(8);

        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .build();

        // 1. Enable Toggle
        let enable_btn = gtk::Switch::builder().valign(gtk::Align::Center).build();
        let enable_row = adw::ActionRow::builder()
            .title("Enable Claw Agent")
            .activatable_widget(&enable_btn)
            .build();
        enable_row.add_suffix(&enable_btn);

        let on_enable_rc = Rc::new(on_enable_toggled);
        enable_btn.connect_state_set(move |_, state| {
            on_enable_rc(state);
            gtk::glib::Propagation::Proceed
        });
        list_box.append(&enable_row);

        // 2. Proactive Toggle
        let proactive_btn = gtk::Switch::builder().valign(gtk::Align::Center).build();
        let proactive_row = adw::ActionRow::builder()
            .title("Proactive Mode")
            .subtitle("Analyze background commands")
            .activatable_widget(&proactive_btn)
            .build();
        proactive_row.add_suffix(&proactive_btn);

        let on_proactive_rc = Rc::new(on_proactive_toggled);
        proactive_btn.connect_state_set(move |_, state| {
            on_proactive_rc(state);
            gtk::glib::Propagation::Proceed
        });
        list_box.append(&proactive_row);

        vbox.append(&list_box);
        popover.set_child(Some(&vbox));

        Self {
            popover,
            enable_btn,
            proactive_btn,
        }
    }

    pub fn popover(&self) -> &gtk::Popover {
        &self.popover
    }

    pub fn show(&self, parent: &gtk::Widget) {
        self.popover.set_parent(parent);
        self.popover.popup();
    }

    pub fn update_ui(&self, active: bool, proactive: bool) {
        self.enable_btn.set_active(active);
        self.proactive_btn.set_active(proactive);
    }
}
