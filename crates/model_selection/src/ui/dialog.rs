use crate::models::ModelProvider;
use crate::ui::selector::SingleModelSelector;
use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita::prelude::*;

#[derive(Clone)]
pub struct GlobalModelSelectorDialog {
    dialog: libadwaita::Dialog,
    pub ai_chat_selector: SingleModelSelector,
    pub claw_selector: SingleModelSelector,
    pub memory_selector: SingleModelSelector,
}

impl GlobalModelSelectorDialog {
    pub fn new<F1, F2, F3>(
        init_ai: ModelProvider,
        init_apps: ModelProvider,
        init_memory: Option<ModelProvider>,
        ollama_url: String,
        on_ai_change: F1,
        on_apps_change: F2,
        on_memory_change: F3,
    ) -> Self
    where
        F1: Fn(ModelProvider) + 'static,
        F2: Fn(ModelProvider) + 'static,
        F3: Fn(Option<ModelProvider>) + 'static,
    {
        let dialog = libadwaita::Dialog::builder()
            .title("Models Selection")
            .content_width(450)
            .content_height(350)
            .build();

        let stack = gtk::Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
        stack.set_hhomogeneous(true);
        stack.set_vhomogeneous(true);

        let ai_chat_selector = SingleModelSelector::new(init_ai, ollama_url.clone(), on_ai_change);
        let claw_selector = SingleModelSelector::new(init_apps.clone(), ollama_url.clone(), on_apps_change);
        
        let mem_initial = init_memory.unwrap_or(init_apps);
        let memory_selector = SingleModelSelector::new(mem_initial, ollama_url, move |new_prov| {
            on_memory_change(Some(new_prov));
        });

        stack.add_titled(ai_chat_selector.widget(), Some("ai"), "AI Assistant");
        stack.add_titled(claw_selector.widget(), Some("claw"), "Boxxy Claw");
        stack.add_titled(memory_selector.widget(), Some("memory"), "Memories");

        let switcher = gtk::StackSwitcher::new();
        switcher.set_stack(Some(&stack));
        switcher.set_margin_top(6);
        switcher.set_margin_start(10);
        switcher.set_margin_end(10);
        switcher.set_margin_bottom(6);
        switcher.set_halign(gtk::Align::Center);

        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.append(&switcher);
        container.append(&stack);

        let close_btn = gtk::Button::builder()
            .label("Done")
            .margin_start(10)
            .margin_end(10)
            .margin_bottom(10)
            .halign(gtk::Align::Center)
            .css_classes(["suggested-action", "pill"])
            .build();
        container.append(&close_btn);

        let d_clone = dialog.clone();
        close_btn.connect_clicked(move |_| {
            d_clone.close();
        });

        dialog.set_child(Some(&container));

        Self {
            dialog,
            ai_chat_selector,
            claw_selector,
            memory_selector,
        }
    }

    pub fn dialog(&self) -> &libadwaita::Dialog {
        &self.dialog
    }

    pub fn present(&self, parent: Option<&impl IsA<gtk::Widget>>) {
        self.dialog.present(parent);
    }
}