use gtk4 as gtk;
use libadwaita as adw;
use adw::prelude::*;

#[allow(deprecated)]
pub fn populate_shortcuts_page(page: &adw::PreferencesPage) -> Vec<(adw::PreferencesGroup, Vec<adw::ActionRow>)> {
    let categories = boxxy_keybindings::get_shortcuts_by_category();
    let mut elements = Vec::new();

    for category in categories {
        let group = adw::PreferencesGroup::builder()
            .title(category.name)
            .build();
            
        let mut rows = Vec::new();

        for (title, keybinding) in category.items {
            let row = adw::ActionRow::builder()
                .title(title)
                .build();

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
    
    elements
}
