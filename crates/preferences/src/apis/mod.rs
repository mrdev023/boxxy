use crate::config::Settings;
use gtk4 as gtk;
use libadwaita as adw;
use gtk::prelude::*;
use adw::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;

pub fn setup_apis_page(
    builder: &gtk::Builder,
    settings_rc: Rc<RefCell<Settings>>,
    on_change: Rc<dyn Fn(Settings) + 'static>,
) -> Box<dyn Fn(&str) -> bool> {
    let dynamic_apis_group: adw::PreferencesGroup = builder.object("dynamic_apis_group").unwrap();
    let ollama_base_url_entry: adw::EntryRow = builder.object("ollama_base_url_entry").unwrap();
    let group_ollama_api: adw::PreferencesGroup = builder.object("group_ollama_api").unwrap();

    let providers = boxxy_model_selection::get_providers();
    let mut dynamic_rows = Vec::new();

    for prov in &providers {
        if prov.requires_api_key() {
            let row = adw::PasswordEntryRow::builder()
                .title(format!("{} API Key", prov.name()))
                .build();

            let initial_key = settings_rc.borrow().api_keys.get(prov.name()).cloned().unwrap_or_default();
            row.set_text(&initial_key);

            let s_rc = settings_rc.clone();
            let cb = on_change.clone();
            let prov_name = prov.name().to_string();
            row.connect_changed(move |entry| {
                let mut s = s_rc.borrow_mut();
                let new_val = entry.text().to_string();
                if s.api_keys.get(&prov_name) != Some(&new_val) {
                    s.api_keys.insert(prov_name.clone(), new_val);
                    s.save();
                    cb(s.clone());
                }
            });

            dynamic_apis_group.add(&row);
            dynamic_rows.push((prov.name().to_string(), row));
        }
    }

    ollama_base_url_entry.set_text(&settings_rc.borrow().ollama_base_url);
    let s_rc3 = settings_rc.clone();
    let cb3 = on_change.clone();
    ollama_base_url_entry.connect_changed(move |entry| {
        let mut s = s_rc3.borrow_mut();
        if s.ollama_base_url != entry.text().as_str() {
            s.ollama_base_url = entry.text().to_string();
            s.save();
            cb3(s.clone());
        }
    });

    let ollama_base_url_entry_clone = ollama_base_url_entry.clone();

    Box::new(move |query: &str| {
        let match_row = |r: &gtk::Widget, text: &str| {
            let m = query.is_empty() || text.to_lowercase().contains(query);
            r.set_visible(m);
            m
        };

        let mut any_visible = false;
        for (name, row) in &dynamic_rows {
            if match_row(row.upcast_ref(), &format!("{} api key", name)) {
                any_visible = true;
            }
        }
        
        let ollama_visible = match_row(ollama_base_url_entry_clone.upcast_ref(), "base url ollama");
        group_ollama_api.set_visible(ollama_visible);
        dynamic_apis_group.set_visible(any_visible);

        any_visible || ollama_visible
    })
}