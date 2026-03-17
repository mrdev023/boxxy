use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

#[allow(deprecated)]
pub fn populate_shortcuts_page(page: &adw::PreferencesPage) -> Box<dyn Fn(&str) -> bool> {
    let categories = boxxy_keybindings::get_shortcuts_by_category();
    let mut elements = Vec::new();

    for category in categories {
        let group = adw::PreferencesGroup::builder()
            .title(category.name)
            .build();

        let mut rows = Vec::new();

        for (title, keybinding) in category.items {
            let row = adw::ActionRow::builder().title(title).build();

            let shortcut_label = gtk::ShortcutLabel::builder()
                .accelerator(keybinding.trigger)
                .valign(gtk::Align::Center)
                .build();

            row.add_suffix(&shortcut_label);
            group.add(&row);
            rows.push(row);
        }

        page.add(&group);
        elements.push((group, rows));
    }

    Box::new(move |query: &str| {
        let mut page_visible = false;
        for (group, rows) in &elements {
            let mut group_visible = false;
            for row in rows {
                let title = row.title().to_lowercase();
                let m = query.is_empty() || title.contains(query);
                row.set_visible(m);
                if m {
                    group_visible = true;
                }
            }
            group.set_visible(group_visible);
            if group_visible {
                page_visible = true;
            }
        }
        page_visible
    })
}
