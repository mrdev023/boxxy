use boxxy_claw_protocol::AgentStatus;
use gtk::prelude::*;
use gtk4 as gtk;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
pub struct ClawIndicator {
    container: gtk::Box,       // Detailed indicator (for drawer)
    badge_container: gtk::Box, // Small badge (always in terminal)
    badge_box: gtk::Box,
    badge_label: gtk::Label,
    badge_revealer: gtk::Revealer,
    clock_icon: gtk::Image,
    sleep_icon: gtk::Image,
    lock_icon: gtk::Image,
    revealer: gtk::Revealer,
    spinner: gtk::Image,
    icon: gtk::Image,
    label: gtk::Label,
    main_btn: gtk::Button,
    cancel_btn: gtk::Button,
    action_type: Rc<RefCell<u8>>, // 0 = none, 1 = lazy, 2 = proactive
    is_active: Rc<Cell<bool>>,
    is_evicted: Rc<Cell<bool>>,
    has_tasks: Rc<Cell<bool>>,
    badge_provider: gtk::CssProvider,
    pill_provider: gtk::CssProvider,
}

impl ClawIndicator {
    pub fn new() -> Self {
        let base_css = "
            .indicator-container {
                background: transparent;
                border: none;
                box-shadow: none;
                padding: 0;
                margin: 0;
            }
            .badge-label {
                border-radius: 9999px;
                padding: 1px 8px;
                font-weight: bold;
                font-size: 0.68rem;
                color: white;
            }
            .badge-label image {
                -gtk-icon-size: 10px;
            }
            .badge-label.warning {
                color: #fff3cd;
            }
            .badge-label.evicted {
                filter: grayscale(100%);
                opacity: 0.25;
            }
            .indicator-container revealer {
                padding: 0;
                margin: 0;
            }
            .indicator-pill {
                border-radius: 9999px;
                border: 1px solid rgba(255,255,255,0.12);
                box-shadow: 0 4px 18px rgba(0,0,0,0.5);
                padding: 4px 6px 4px 10px;
            }
            .indicator-pill button.flat {
                border-radius: 9999px;
                padding: 2px 6px;
                min-height: 0;
                min-width: 0;
                color: white;
            }
            .indicator-pill button.flat:hover {
                background-color: rgba(255,255,255,0.15);
            }
            .indicator-pill button.circular {
                border-radius: 9999px;
                padding: 2px;
                min-height: 0;
                min-width: 0;
                opacity: 0.75;
                color: white;
            }
            .indicator-pill button.circular:hover { opacity: 1.0; }
            .indicator-pill image.accent  { color: rgba(255,255,255,0.90); }
            .indicator-pill image.warning { color: #fff3cd; }
            .indicator-pill image.success { color: #d4f7d4; }
            .status-label {
                font-size: 0.82rem;
                font-weight: bold;
                color: white;
            }
        ";
        let base_provider = gtk::CssProvider::new();
        base_provider.load_from_string(base_css);
        #[allow(deprecated)]
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &base_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // 1. Small Badge Widget (Always in terminal)
        let badge_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .halign(gtk::Align::End)
            .valign(gtk::Align::Start)
            .margin_top(12)
            .margin_end(12)
            .css_classes(["indicator-container"])
            .spacing(0)
            .build();

        // 2. Detailed Indicator Widget (For drawer)
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        container.set_valign(gtk::Align::Center);

        let revealer = gtk::Revealer::new();
        revealer.set_transition_type(gtk::RevealerTransitionType::SlideRight);
        revealer.set_transition_duration(280);

        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        hbox.add_css_class("indicator-pill");
        hbox.set_valign(gtk::Align::Center);

        let svg = gtk::Svg::from_resource("/dev/boxxy/BoxxyTerminal/icons/boxxy-spinner.gpa");
        svg.play();

        let spinner = gtk::Image::builder().paintable(&svg).pixel_size(20).build();
        spinner.add_css_class("claw-spinner");

        spinner.connect_map({
            let svg = svg.clone();
            move |widget| {
                if let Some(native) = widget.native() {
                    if let Some(surface) = native.surface() {
                        svg.set_frame_clock(&surface.frame_clock());
                    }
                }
            }
        });

        hbox.append(&spinner);

        let icon = gtk::Image::from_icon_name("boxxy-boxxyclaw-symbolic");
        icon.add_css_class("accent");
        icon.set_pixel_size(13);
        hbox.append(&icon);

        let label = gtk::Label::new(Some("Working"));
        label.add_css_class("status-label");
        hbox.append(&label);

        let main_btn = gtk::Button::builder()
            .label("Action")
            .css_classes(["flat"])
            .build();
        main_btn.set_visible(false);
        hbox.append(&main_btn);

        let cancel_btn = gtk::Button::builder()
            .icon_name("boxxy-window-close-symbolic")
            .css_classes(["flat", "circular"])
            .tooltip_text("Cancel")
            .build();
        cancel_btn.set_valign(gtk::Align::Center);

        hbox.append(&cancel_btn);
        revealer.set_child(Some(&hbox));
        container.append(&revealer);

        let action_type = Rc::new(RefCell::new(0));

        // Shared Badge logic. Start hidden — the badge only surfaces once
        // the daemon sends us an Identity event with a real agent name.
        // Without an identity, drawing a placeholder "CLAW" pill is noisy.
        let badge_revealer = gtk::Revealer::new();
        badge_revealer.set_transition_type(gtk::RevealerTransitionType::Crossfade);
        badge_revealer.set_transition_duration(280);
        badge_revealer.set_reveal_child(false);

        let badge_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        badge_box.add_css_class("badge-label");
        badge_box.set_valign(gtk::Align::Center);
        badge_box.set_height_request(20);

        let badge_label = gtk::Label::builder().valign(gtk::Align::Center).build();
        badge_box.append(&badge_label);

        let clock_icon = gtk::Image::builder()
            .icon_name("boxxy-timer-symbolic")
            .visible(false)
            .build();
        badge_box.append(&clock_icon);

        let sleep_icon = gtk::Image::builder()
            .icon_name("boxxy-bedtime-symbolic")
            .visible(false)
            .css_classes(["warning"])
            .build();
        badge_box.append(&sleep_icon);

        let lock_icon = gtk::Image::builder()
            .icon_name("boxxy-lock-symbolic")
            .visible(false)
            .build();
        badge_box.append(&lock_icon);

        badge_revealer.set_child(Some(&badge_box));
        badge_container.append(&badge_revealer);

        let badge_provider = gtk::CssProvider::new();
        #[allow(deprecated)]
        badge_box
            .style_context()
            .add_provider(&badge_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let pill_provider = gtk::CssProvider::new();
        #[allow(deprecated)]
        hbox.style_context()
            .add_provider(&pill_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        Self {
            container,
            badge_container,
            badge_box,
            badge_label,
            badge_revealer,
            clock_icon,
            sleep_icon,
            lock_icon,
            revealer,
            spinner,
            icon,
            label,
            main_btn,
            cancel_btn,
            action_type,
            is_active: Rc::new(Cell::new(false)),
            is_evicted: Rc::new(Cell::new(false)),
            has_tasks: Rc::new(Cell::new(false)),
            badge_provider,
            pill_provider,
        }
    }

    /// Detailed indicator for the Drawer
    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    /// Persistent badge for the Terminal Overlay
    pub fn badge(&self) -> &gtk::Box {
        &self.badge_container
    }

    pub fn set_callbacks<F1: Fn() + 'static, F2: Fn() + 'static, F3: Fn() + 'static>(
        &self,
        on_cancel: F1,
        on_lazy_click: F2,
        on_proactive_click: F3,
    ) {
        let p_clone = self.revealer.clone();
        let badge_revealer_clone = self.badge_revealer.clone();
        let clock_clone = self.clock_icon.clone();
        let has_tasks_clone = self.has_tasks.clone();

        self.cancel_btn.connect_clicked(move |_| {
            p_clone.set_reveal_child(false);
            badge_revealer_clone.set_reveal_child(true);
            clock_clone.set_visible(has_tasks_clone.get());
            on_cancel();
        });

        let on_lazy_click = Rc::new(on_lazy_click);
        let on_proactive_click = Rc::new(on_proactive_click);
        let action_type = self.action_type.clone();

        self.main_btn.connect_clicked(move |_| {
            let at = *action_type.borrow();
            if at == 1 {
                on_lazy_click();
            } else if at == 2 {
                on_proactive_click();
            }
        });
    }

    pub fn set_visible(&self, visible: bool) {
        self.badge_container.set_visible(visible);
        self.container.set_visible(visible);
    }

    pub fn set_evicted(&self, evicted: bool) {
        self.is_evicted.set(evicted);
        if evicted {
            self.badge_box.add_css_class("evicted");
            self.badge_label.set_label("EVICTED");
        } else {
            self.badge_box.remove_css_class("evicted");
        }
    }

    pub fn set_mode(&self, status: AgentStatus) {
        self.clock_icon.set_visible(false);
        self.sleep_icon.set_visible(false);
        self.lock_icon.set_visible(false);
        self.badge_box.remove_css_class("warning");

        // Status only drives the status-icon row + warning CSS class, not
        // the label text. The label is owned by `set_identity()` so the
        // badge always reads as the agent's real name (once known) rather
        // than the generic "CLAW" / "WORKING" placeholder.
        match status {
            AgentStatus::Waiting | AgentStatus::Working => {}
            AgentStatus::Sleep => {
                self.sleep_icon.set_visible(true);
                self.badge_box.add_css_class("warning");
            }
            AgentStatus::Locking { .. } => {
                self.lock_icon.set_visible(true);
            }
            AgentStatus::Faulted { .. } => {
                self.badge_box.add_css_class("warning");
            }
            AgentStatus::Off => {
                // Dormant session — retract the badge so an "off" pane has
                // no visible indicator. It comes back on the next status
                // change that isn't Off.
                self.badge_revealer.set_reveal_child(false);
                return;
            }
        }

        // Any non-Off status: make sure the badge is revealed (if we
        // already have an identity to show).
        if !self.badge_label.text().is_empty() {
            self.badge_revealer.set_reveal_child(true);
        }
    }

    pub fn show_thinking(&self) {
        self.is_active.set(true);
        self.label.set_label("Thinking..");
        self.main_btn.set_visible(false);
        self.revealer.set_reveal_child(true);
        // The small identity badge and the detailed pill live in
        // different places (badge = pane overlay, pill = drawer
        // header), so they don't crowd each other — the agent's name
        // should stay visible while it works. Same for the other
        // `show_*` states below.
        self.spinner.set_visible(true);
        self.icon.set_visible(false);

        #[allow(deprecated)]
        self.pill_provider
            .load_from_string(".indicator-pill { background-color: rgba(38, 162, 105, 0.85); }");
    }

    pub fn show_diagnosis_ready(&self) {
        self.is_active.set(true);
        self.label.set_label("Diagnosis Ready");
        self.main_btn.set_label("View");
        self.main_btn.set_visible(true);
        *self.action_type.borrow_mut() = 2; // proactive
        self.revealer.set_reveal_child(true);
        self.spinner.set_visible(false);
        self.icon.set_visible(true);
        self.icon.set_icon_name(Some("boxxy-bug-symbolic"));

        #[allow(deprecated)]
        self.pill_provider
            .load_from_string(".indicator-pill { background-color: rgba(53, 132, 228, 0.85); }");
    }

    pub fn show_lazy_error(&self) {
        self.is_active.set(true);
        self.label.set_label("Error Detected");
        self.main_btn.set_label("Diagnose");
        self.main_btn.set_visible(true);
        *self.action_type.borrow_mut() = 1; // lazy
        self.revealer.set_reveal_child(true);
        self.spinner.set_visible(false);
        self.icon.set_visible(true);
        self.icon
            .set_icon_name(Some("boxxy-dialog-warning-symbolic"));

        #[allow(deprecated)]
        self.pill_provider
            .load_from_string(".indicator-pill { background-color: rgba(165, 29, 45, 0.85); }");
    }

    pub fn hide(&self) {
        self.is_active.set(false);
        self.revealer.set_reveal_child(false);
        // Only surface the small badge if we actually have an identity to
        // display — otherwise hiding the detailed pill shouldn't cause an
        // empty placeholder badge to appear.
        let has_identity = !self.badge_label.text().is_empty();
        self.badge_revealer.set_reveal_child(has_identity);
        self.clock_icon.set_visible(self.has_tasks.get());
    }

    pub fn set_has_tasks(&self, has: bool) {
        self.has_tasks.set(has);
        if !self.is_active.get() {
            self.clock_icon.set_visible(has);
        }
    }

    pub fn set_identity(&self, name: &str) {
        self.is_evicted.set(false);
        self.badge_box.remove_css_class("evicted");
        self.badge_label.set_text(name);

        if name.is_empty() {
            // Nothing to display yet — keep the pill retracted so we don't
            // render an empty colored circle over the terminal.
            self.badge_revealer.set_reveal_child(false);
            return;
        }

        let color = self.generate_color(name);

        #[allow(deprecated)]
        self.badge_provider
            .load_from_string(&format!(".badge-label {{ background-color: {}; }}", color));

        // First real identity — reveal the badge.
        self.badge_revealer.set_reveal_child(true);
    }

    fn generate_color(&self, name: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        let hash = hasher.finish();

        let r = (hash & 0xFF) as u8 % 150 + 50;
        let g = ((hash >> 8) & 0xFF) as u8 % 150 + 50;
        let b = ((hash >> 16) & 0xFF) as u8 % 150 + 50;

        format!("rgb({}, {}, {})", r, g, b)
    }

    pub fn update_settings(&self) {
        // No-op for now
    }
}
