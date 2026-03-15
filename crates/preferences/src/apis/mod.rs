use crate::config::Settings;
use gtk4 as gtk;
use libadwaita as adw;
use gtk::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;

pub fn setup_apis_page(
    builder: &gtk::Builder,
    settings_rc: Rc<RefCell<Settings>>,
    on_change: Rc<dyn Fn(Settings) + 'static>,
) -> Box<dyn Fn(&str) -> bool> {
    let api_key_entry: adw::PasswordEntryRow = builder.object("api_key_entry").unwrap();
    let ollama_base_url_entry: adw::EntryRow = builder.object("ollama_base_url_entry").unwrap();
    let group_gemini_api: adw::PreferencesGroup = builder.object("group_gemini_api").unwrap();
    let group_ollama_api: adw::PreferencesGroup = builder.object("group_ollama_api").unwrap();

    api_key_entry.set_text(&settings_rc.borrow().gemini_api_key);
    let s_rc = settings_rc.clone();
    let cb = on_change.clone();
    api_key_entry.connect_changed(move |entry| {
        let mut s = s_rc.borrow_mut();
        if s.gemini_api_key != entry.text().as_str() {
            s.gemini_api_key = entry.text().to_string();
            s.save();
            cb(s.clone());
        }
    });

    ollama_base_url_entry.set_text(&settings_rc.borrow().ollama_base_url);
    let s_rc = settings_rc.clone();
    let cb = on_change.clone();
    ollama_base_url_entry.connect_changed(move |entry| {
        let mut s = s_rc.borrow_mut();
        if s.ollama_base_url != entry.text().as_str() {
            s.ollama_base_url = entry.text().to_string();
            s.save();
            cb(s.clone());
        }
    });

    let api_key_entry_clone = api_key_entry.clone();
    let ollama_base_url_entry_clone = ollama_base_url_entry.clone();

    Box::new(move |query: &str| {
        let match_row = |r: &gtk::Widget, text: &str| {
            let m = query.is_empty() || text.to_lowercase().contains(query);
            r.set_visible(m);
            m
        };

        let a1 = match_row(api_key_entry_clone.upcast_ref(), "api key gemini");
        let a2 = match_row(ollama_base_url_entry_clone.upcast_ref(), "base url ollama");
        
        group_gemini_api.set_visible(a1);
        group_ollama_api.set_visible(a2);
        group_gemini_api.is_visible() || group_ollama_api.is_visible()
    })
}