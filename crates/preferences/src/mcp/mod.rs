use crate::config::Settings;
use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

mod mcp_dialog;

pub fn setup_mcp_page(
    builder: &gtk::Builder,
    settings_rc: Rc<RefCell<Settings>>,
    on_change: Rc<dyn Fn(Settings) + 'static>,
) -> Box<dyn Fn(&str) -> bool> {
    let mcp_servers_group: adw::PreferencesGroup = builder.object("mcp_servers_group").unwrap();
    let mcp_add_server_row: adw::ActionRow = builder.object("mcp_add_server_row").unwrap();

    let render_server_list = {
        let group = mcp_servers_group.clone();
        let s_rc = settings_rc.clone();
        let cb = on_change.clone();
        move || {
            // Collect all rows to remove first to avoid modifying the list while iterating
            let mut to_remove = Vec::new();
            let mut current = group.first_child();
            while let Some(child) = current {
                let next = child.next_sibling();
                if child.downcast_ref::<adw::ActionRow>().is_some()
                    && child.widget_name() != "mcp_add_server_row"
                {
                    to_remove.push(child.clone());
                }
                current = next;
            }

            for child in to_remove {
                group.remove(&child);
            }

            let server_list = {
                let s = s_rc.borrow();
                s.mcp_servers.clone()
            };

            for (idx, server) in server_list.into_iter().enumerate() {
                let row = adw::ActionRow::builder()
                    .title(&server.name)
                    .subtitle(match &server.transport {
                        boxxy_mcp::config::McpTransport::Stdio { command, .. } => {
                            format!("Stdio: {}", command)
                        }
                        boxxy_mcp::config::McpTransport::Http { url, .. } => {
                            format!("HTTP: {}", url)
                        }
                    })
                    .build();

                let switch = gtk::Switch::builder()
                    .active(server.enabled)
                    .valign(gtk::Align::Center)
                    .build();

                let s_rc_inner = s_rc.clone();
                let cb_inner = cb.clone();
                let server_name = server.name.clone();
                switch.connect_active_notify(move |sw| {
                    let mut s = s_rc_inner.borrow_mut();
                    if let Some(srv) = s.mcp_servers.iter_mut().find(|srv| srv.name == server_name)
                    {
                        if srv.enabled != sw.is_active() {
                            srv.enabled = sw.is_active();
                            s.save();
                            cb_inner(s.clone());
                        }
                    }
                });

                row.add_suffix(&switch);

                // Add Edit button
                let edit_btn = gtk::Button::builder()
                    .icon_name("document-edit-symbolic")
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();

                let s_rc_inner = s_rc.clone();
                let cb_inner = cb.clone();
                let row_inner = row.clone();
                let server_clone = server.clone();
                let old_name = server.name.clone();
                edit_btn.connect_clicked(move |btn| {
                    if let Some(window) = btn.root().and_then(|r| r.downcast::<gtk::Window>().ok())
                    {
                        let s_rc_inner2 = s_rc_inner.clone();
                        let cb_inner2 = cb_inner.clone();
                        let row_inner2 = row_inner.clone();
                        let old_name_clone = old_name.clone();
                        let dialog = mcp_dialog::build_add_mcp_dialog(
                            &window,
                            Some(&server_clone),
                            move |new_config| {
                                let s_clone = {
                                    let mut s = s_rc_inner2.borrow_mut();
                                    if let Some(srv) = s
                                        .mcp_servers
                                        .iter_mut()
                                        .find(|srv| srv.name == old_name_clone)
                                    {
                                        *srv = new_config.clone();
                                    }
                                    s.clone()
                                };

                                s_clone.save();
                                cb_inner2(s_clone);

                                // Update row UI manually
                                row_inner2.set_title(&new_config.name);
                                let sub = match &new_config.transport {
                                    boxxy_mcp::config::McpTransport::Stdio { command, .. } => {
                                        format!("Stdio: {}", command)
                                    }
                                    boxxy_mcp::config::McpTransport::Http { url, .. } => {
                                        format!("HTTP: {}", url)
                                    }
                                };
                                row_inner2.set_subtitle(&sub);
                            },
                        );
                        dialog.present();
                    }
                });

                row.add_suffix(&edit_btn);

                // Add Delete button
                let del_btn = gtk::Button::builder()
                    .icon_name("user-trash-symbolic")
                    .valign(gtk::Align::Center)
                    .css_classes(["flat", "destructive-action"])
                    .build();

                let s_rc_inner = s_rc.clone();
                let cb_inner = cb.clone();
                let group_inner = group.clone();
                let row_inner = row.clone();
                del_btn.connect_clicked(move |_| {
                    let mut s = s_rc_inner.borrow_mut();
                    let server_name = server.name.clone();
                    s.mcp_servers.retain(|srv| srv.name != server_name);
                    s.save();
                    cb_inner(s.clone());
                    group_inner.remove(&row_inner);
                });

                row.add_suffix(&del_btn);
                group.add(&row);
            }
        }
    };

    render_server_list();

    let s_rc = settings_rc.clone();
    let cb = on_change.clone();
    let render_clone = Rc::new(render_server_list);
    mcp_add_server_row.connect_activated(move |row| {
        if let Some(window) = row.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
            let s_rc_inner = s_rc.clone();
            let cb_inner = cb.clone();
            let render_inner = render_clone.clone();
            let dialog = mcp_dialog::build_add_mcp_dialog(&window, None, move |new_config| {
                let s_clone = {
                    let mut s = s_rc_inner.borrow_mut();
                    s.mcp_servers.push(new_config);
                    s.clone()
                };
                s_clone.save();
                cb_inner(s_clone);
                render_inner();
            });
            dialog.present();
        }
    });

    let mcp_add_server_row_clone = mcp_add_server_row.clone();
    let mcp_servers_group_clone = mcp_servers_group.clone();

    Box::new(move |query: &str| {
        let match_row = |r: &gtk::Widget, text: &str| {
            let m = query.is_empty() || text.to_lowercase().contains(query);
            r.set_visible(m);
            m
        };

        let mcp_visible = match_row(
            mcp_add_server_row_clone.upcast_ref(),
            "mcp servers model context protocol external tools add",
        );

        mcp_servers_group_clone.set_visible(mcp_visible);

        mcp_servers_group_clone.is_visible()
    })
}
