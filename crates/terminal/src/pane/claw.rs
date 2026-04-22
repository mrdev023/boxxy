use super::{PaneInner, PendingDiagnosis};
use crate::PaneOutput;
use crate::claw_indicator::ClawIndicator;
use crate::overlay::{OverlayMode, TerminalOverlay};
use boxxy_claw_protocol::{AgentStatus, ClawEngineEvent, ClawMessage, UsageWrapper};
use gtk4 as gtk;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Computes a session-total delta from a per-turn usage wrapper. The protocol
/// dropped the materialized `total_tokens` field, so we sum it at the call site.
#[inline]
fn usage_total(u: &UsageWrapper) -> u64 {
    u.input_tokens + u.output_tokens
}

pub(super) fn setup_claw(
    widget: &gtk::Overlay,
    inner: &Rc<RefCell<PaneInner>>,
    id: String,
    claw_sender: async_channel::Sender<ClawMessage>,
    claw_rx: async_channel::Receiver<ClawEngineEvent>,
    claw_list_store: gtk::gio::ListStore,
    callback: std::sync::Arc<dyn Fn(PaneOutput) + Send + Sync + 'static>,
    spawn_intent: Option<String>,
    total_tokens: Rc<Cell<u64>>,
    is_pinned: Rc<Cell<bool>>,
    is_web_search: Rc<Cell<bool>>,
    session_status: Rc<RefCell<AgentStatus>>,
    agent_name: Rc<RefCell<String>>,
    claw_indicator: &ClawIndicator,
) -> (TerminalOverlay, PendingDiagnosis) {
    let pending_proactive_diagnosis =
        Rc::new(RefCell::new(None::<(String, crate::TerminalProposal)>));
    let pending_diag_clone = pending_proactive_diagnosis.clone();

    // Provide the initial intent if one was passed in
    if let Some(intent) = spawn_intent {
        let tx = claw_sender.clone();
        let inner_clone = inner.clone();

        // Wait for PID to ensure PTY is ready
        gtk::glib::spawn_future_local(async move {
            let mut check_count = 0;
            loop {
                let has_pid = inner_clone.borrow().pid.is_some();
                if has_pid {
                    break;
                }
                check_count += 1;
                if check_count > 50 {
                    // Timeout after 5 seconds
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            let pane = inner_clone.borrow().terminal.clone();
            let cwd = inner_clone.borrow().working_dir.clone().unwrap_or_default();
            if let Some(snapshot) = pane.get_text_snapshot(100, 0).await {
                let _ = tx
                    .send(ClawMessage::UserMessage {
                        message: intent,
                        snapshot,
                        cwd,
                        image_attachments: vec![],
                    })
                    .await;
            }
        });
    }

    let inner_clone_for_cmd = Rc::downgrade(inner);
    let inner_clone_for_reply = Rc::downgrade(inner);
    let inner_clone_for_file_reply = Rc::downgrade(inner);
    let inner_clone_for_cancel = Rc::downgrade(inner);
    let tx_user_reply = claw_sender.clone();
    let tx_file_reply = claw_sender.clone();
    let tx_lazy_reply = claw_sender.clone();

    let claw_popover_self_ref: Rc<RefCell<Option<TerminalOverlay>>> = Rc::new(RefCell::new(None));
    let cb_focus = callback.clone();

    let id_for_focus = id.clone();
    let claw_popover_for_file_reply = claw_popover_self_ref.clone();
    let claw_popover = TerminalOverlay::new(
        move |cmd: String| {
            if let Some(inner) = inner_clone_for_cmd.upgrade() {
                let mut bytes = cmd.as_bytes().to_vec();
                bytes.push(b'\r');
                inner.borrow().terminal.write_all(bytes);
                inner.borrow().terminal.grab_focus();
            }
        },
        move |(reply, image_attachments)| {
            let tx = tx_user_reply.clone();
            let inner_opt = inner_clone_for_reply.upgrade();
            if let Some(inner) = inner_opt {
                let pane = inner.borrow().terminal.clone();
                let cwd = inner.borrow().working_dir.clone().unwrap_or_default();
                inner.borrow().terminal.grab_focus();
                gtk::glib::spawn_future_local(async move {
                    if let Some(snapshot) = pane.get_text_snapshot(100, 0).await {
                        let _ = tx
                            .send(ClawMessage::UserMessage {
                                message: reply,
                                snapshot,
                                cwd,
                                image_attachments,
                            })
                            .await;
                    }
                });
            }
        },
        move |approved| {
            if let Some(inner) = inner_clone_for_file_reply.upgrade() {
                inner.borrow().terminal.grab_focus();
            }

            let proposal = if let Some(popover) = claw_popover_for_file_reply.borrow().as_ref() {
                popover.current_proposal()
            } else {
                crate::TerminalProposal::None
            };

            let tx = tx_file_reply.clone();
            gtk::glib::spawn_future_local(async move {
                let msg = match proposal {
                    crate::TerminalProposal::FileWrite(_, _) => {
                        ClawMessage::FileWriteReply { approved }
                    }
                    crate::TerminalProposal::FileDelete(_) => {
                        ClawMessage::FileDeleteReply { approved }
                    }
                    crate::TerminalProposal::KillProcess(_, _) => {
                        ClawMessage::KillProcessReply { approved }
                    }
                    crate::TerminalProposal::GetClipboard => {
                        ClawMessage::GetClipboardReply { approved }
                    }
                    crate::TerminalProposal::SetClipboard(_) => {
                        ClawMessage::SetClipboardReply { approved }
                    }
                    _ => ClawMessage::FileWriteReply { approved },
                };
                let _ = tx.send(msg).await;
            });
        },
        move |_proposal| {
            cb_focus(PaneOutput::FocusClawSidebar(id_for_focus.clone()));
        },
        {
            let tx_cancel = claw_sender.clone();
            move |mode| {
                if let Some(inner) = inner_clone_for_cancel.upgrade() {
                    let term = inner.borrow().terminal.clone();
                    // Small delay to ensure focus sticks after the overlay is hidden
                    gtk4::glib::timeout_add_local(
                        std::time::Duration::from_millis(50),
                        move || {
                            term.grab_focus();
                            gtk4::glib::ControlFlow::Break
                        },
                    );
                }
                if mode == OverlayMode::Claw {
                    let tx = tx_cancel.clone();
                    gtk::glib::spawn_future_local(async move {
                        let _ = tx
                            .send(ClawMessage::CancelPending)
                            .await;
                    });
                }
            }
        },
        {
            let inner_clone_for_vis = Rc::downgrade(inner);
            move |visible| {
                if let Some(inner) = inner_clone_for_vis.upgrade() {
                    inner.borrow().terminal.set_focusable(!visible);
                }
            }
        },
    );
    *claw_popover_self_ref.borrow_mut() = Some(claw_popover.clone());
    widget.add_overlay(claw_popover.widget());

    let popover_clone = claw_popover.clone();
    claw_indicator.set_callbacks(
        || {},
        move || {
            let tx = tx_lazy_reply.clone();
            gtk::glib::spawn_future_local(async move {
                let _ = tx
                    .send(ClawMessage::RequestLazyDiagnosis {})
                    .await;
            });
        },
        move || {
            if let Some((diag, proposal)) = pending_diag_clone.borrow_mut().take() {
                popover_clone.show(OverlayMode::Claw, "Boxxy-Claw", None, &diag, proposal);
            }
        },
    );

    let cb_clone_events = callback.clone();
    let popover_event_clone = claw_popover.clone();
    let indicator_event_clone = claw_indicator.clone();
    let claw_store_events = claw_list_store.clone();
    let inner_clone = inner.clone();
    let total_tokens_for_events = total_tokens.clone();
    let is_pinned_for_events = is_pinned.clone();
    let is_web_search_for_events = is_web_search.clone();
    let agent_name_for_events = agent_name.clone();
    // Used to reply to correlation-ID requests from the engine (e.g. scrollback).
    let tx_engine_reply = claw_sender.clone();

    gtk::glib::spawn_future_local(async move {
        while let Ok(event) = claw_rx.recv().await {
            match &event {
                ClawEngineEvent::SessionStateChanged { status, .. } => {
                    *session_status.borrow_mut() = status.clone();
                    inner_clone.borrow().msg_bar.set_status(status.clone());

                    if let Some(indicator) = &inner_clone.borrow().claw_indicator {
                        indicator.set_mode(status.clone());
                    }
                }
                ClawEngineEvent::UserMessage { content } => {
                    boxxy_claw_ui::add_user_row(&claw_store_events, id.clone(), content);
                }
                ClawEngineEvent::AgentThinking { is_thinking, .. } => {
                    if *is_thinking {
                        indicator_event_clone.show_thinking();
                    } else {
                        indicator_event_clone.hide();
                    }
                }
                ClawEngineEvent::LazyErrorIndicator { .. } => {
                    indicator_event_clone.show_lazy_error();
                }
                ClawEngineEvent::DiagnosisComplete {
                    diagnosis,
                    agent_name,
                    usage,
                } => {
                    if let Some(usage) = usage {
                        total_tokens_for_events
                            .set(total_tokens_for_events.get() + usage_total(usage));
                    }
                    boxxy_claw_ui::add_diagnosis_row(
                        &claw_store_events,
                        id.clone(),
                        Some(agent_name.clone()),
                        diagnosis,
                    );
                    indicator_event_clone.hide();
                    popover_event_clone.show(
                        OverlayMode::Claw,
                        &agent_name,
                        Some("Diagnosis"),
                        diagnosis,
                        crate::TerminalProposal::None,
                    );
                }
                // Proposal events: the in-terminal popover owns user
                // approval. The sidebar gets a read-only log entry so
                // the history view reflects that the agent proposed
                // something — the approval-row helpers format it as a
                // Diagnosis row and ignore their callback args.
                ClawEngineEvent::InjectCommand {
                    command,
                    diagnosis,
                    agent_name,
                    usage,
                } => {
                    if let Some(usage) = usage {
                        total_tokens_for_events
                            .set(total_tokens_for_events.get() + usage_total(usage));
                    }
                    if !diagnosis.is_empty() {
                        boxxy_claw_ui::add_diagnosis_row(
                            &claw_store_events,
                            id.clone(),
                            Some(agent_name.clone()),
                            diagnosis,
                        );
                    }
                    boxxy_claw_ui::add_approval_row(
                        &claw_store_events,
                        id.clone(),
                        Some(agent_name.clone()),
                        command,
                        |_| {},
                    );
                    indicator_event_clone.hide();
                    popover_event_clone.show(
                        OverlayMode::Claw,
                        agent_name,
                        Some("Propose Command"),
                        diagnosis,
                        crate::TerminalProposal::Command(command.clone()),
                    );
                }
                ClawEngineEvent::ProposeFileWrite {
                    path,
                    content,
                    agent_name,
                    usage,
                } => {
                    if let Some(usage) = usage {
                        total_tokens_for_events
                            .set(total_tokens_for_events.get() + usage_total(usage));
                    }
                    boxxy_claw_ui::add_file_write_approval_row(
                        &claw_store_events,
                        id.clone(),
                        Some(agent_name.clone()),
                        path,
                        content,
                        |_| {},
                        |_| {},
                    );
                    popover_event_clone.show(
                        OverlayMode::Claw,
                        agent_name,
                        Some("Propose File Edit"),
                        &format!("Path: `{path}`\n\nI need to write or edit this file to complete the task."),
                        crate::TerminalProposal::FileWrite(path.clone(), content.clone())
                    );
                }
                ClawEngineEvent::ProposeFileDelete {
                    path,
                    agent_name,
                    usage,
                } => {
                    if let Some(usage) = usage {
                        total_tokens_for_events
                            .set(total_tokens_for_events.get() + usage_total(usage));
                    }
                    boxxy_claw_ui::add_file_delete_approval_row(
                        &claw_store_events,
                        id.clone(),
                        Some(agent_name.clone()),
                        path,
                        |_| {},
                        |_| {},
                    );
                    popover_event_clone.show(
                        OverlayMode::Claw,
                        agent_name,
                        Some("Delete File"),
                        &format!("I want to DELETE this file:\n\n`{path}`"),
                        crate::TerminalProposal::FileDelete(path.clone()),
                    );
                }
                ClawEngineEvent::ProposeKillProcess {
                    pid,
                    process_name,
                    agent_name,
                    usage,
                } => {
                    if let Some(usage) = usage {
                        total_tokens_for_events
                            .set(total_tokens_for_events.get() + usage_total(usage));
                    }
                    boxxy_claw_ui::add_kill_process_approval_row(
                        &claw_store_events,
                        id.clone(),
                        Some(agent_name.clone()),
                        *pid,
                        process_name,
                        |_| {},
                        |_| {},
                    );
                    popover_event_clone.show(
                        OverlayMode::Claw,
                        agent_name,
                        Some("Kill Process"),
                        &format!("I want to KILL this process:\n\nPID: {pid} ({process_name})"),
                        crate::TerminalProposal::KillProcess(*pid, process_name.clone()),
                    );
                }
                ClawEngineEvent::ProposeGetClipboard { agent_name, usage } => {
                    if let Some(usage) = usage {
                        total_tokens_for_events
                            .set(total_tokens_for_events.get() + usage_total(usage));
                    }
                    boxxy_claw_ui::add_clipboard_get_approval_row(
                        &claw_store_events,
                        id.clone(),
                        Some(agent_name.clone()),
                        |_| {},
                        |_| {},
                    );
                    popover_event_clone.show(
                        OverlayMode::Claw,
                        agent_name,
                        Some("Read Clipboard"),
                        "I want to read your clipboard.",
                        crate::TerminalProposal::GetClipboard,
                    );
                }
                ClawEngineEvent::ProposeSetClipboard {
                    agent_name,
                    text,
                    usage,
                } => {
                    if let Some(usage) = usage {
                        total_tokens_for_events
                            .set(total_tokens_for_events.get() + usage_total(usage));
                    }
                    boxxy_claw_ui::add_clipboard_set_approval_row(
                        &claw_store_events,
                        id.clone(),
                        Some(agent_name.clone()),
                        text,
                        |_| {},
                        |_| {},
                    );
                    popover_event_clone.show(
                        OverlayMode::Claw,
                        agent_name,
                        Some("Write Clipboard"),
                        &format!("I want to write this to your clipboard:\n\n{text}"),
                        crate::TerminalProposal::SetClipboard(text.clone()),
                    );
                }
                ClawEngineEvent::ProposeTerminalCommand {
                    command,
                    explanation,
                    agent_name,
                    usage,
                } => {
                    if let Some(usage) = usage {
                        total_tokens_for_events
                            .set(total_tokens_for_events.get() + usage_total(usage));
                    }
                    if !explanation.is_empty() {
                        boxxy_claw_ui::add_diagnosis_row(
                            &claw_store_events,
                            id.clone(),
                            Some(agent_name.clone()),
                            explanation,
                        );
                    }
                    boxxy_claw_ui::add_approval_row(
                        &claw_store_events,
                        id.clone(),
                        Some(agent_name.clone()),
                        command,
                        |_| {},
                    );
                    popover_event_clone.show(
                        OverlayMode::Claw,
                        agent_name,
                        Some("Terminal Command"),
                        explanation,
                        crate::TerminalProposal::Command(command.clone()),
                    );
                }
                ClawEngineEvent::Identity {
                    agent_name,
                    pinned,
                    web_search_enabled,
                    total_tokens,
                } => {
                    if let Some(ind) = &inner_clone.borrow().claw_indicator {
                        ind.set_identity(agent_name);
                    }
                    is_pinned_for_events.set(*pinned);
                    is_web_search_for_events.set(*web_search_enabled);
                    *agent_name_for_events.borrow_mut() = agent_name.clone();

                    let status = session_status.borrow().clone();
                    inner_clone
                        .borrow()
                        .msg_bar
                        .update_ui(status, *pinned, *web_search_enabled);
                    total_tokens_for_events.set(*total_tokens);
                }
                ClawEngineEvent::PinStatusChanged(pinned) => {
                    is_pinned_for_events.set(*pinned);
                    let status = session_status.borrow().clone();
                    inner_clone.borrow().msg_bar.update_ui(
                        status,
                        *pinned,
                        inner_clone.borrow().msg_bar.web_search_state.get(),
                    );
                }
                ClawEngineEvent::WebSearchStatusChanged(enabled) => {
                    is_web_search_for_events.set(*enabled);
                    let status = session_status.borrow().clone();
                    inner_clone.borrow().msg_bar.update_ui(
                        status,
                        inner_clone.borrow().msg_bar.pin_state.get(),
                        *enabled,
                    );
                }
                ClawEngineEvent::Evicted => {
                    if let Some(ind) = &inner_clone.borrow().claw_indicator {
                        ind.set_evicted(true);
                    }
                    indicator_event_clone.hide();
                    popover_event_clone.hide();
                    boxxy_claw_ui::add_diagnosis_row(
                        &claw_store_events,
                        id.clone(),
                        None,
                        "Agent was EVICTED because the session was resumed in another pane.",
                    );
                }
                ClawEngineEvent::RequestCwdSwitch { path } => {
                    let pane_inner = inner_clone.borrow();
                    let terminal = pane_inner.terminal.clone();
                    let path = path.clone();
                    let id_clone_notify = id.clone();
                    let cb_clone_notify = cb_clone_events.clone();

                    gtk::glib::spawn_future_local(async move {
                        // Check if in alt screen (terminal busy with TUI like vim)
                        if terminal.is_alt_screen() {
                            cb_clone_notify(PaneOutput::Notification(
                                id_clone_notify,
                                "Session resumed, but folder switch skipped (Terminal Busy)."
                                    .to_string(),
                            ));
                            return;
                        }

                        // Validate path exists on host
                        if std::path::Path::new(&path).exists() {
                            terminal.write_all(format!("cd \"{}\"\n", path).into_bytes());
                        } else {
                            cb_clone_notify(PaneOutput::Notification(
                                id_clone_notify,
                                format!(
                                    "Directory '{}' no longer exists. Staying in current folder.",
                                    path
                                ),
                            ));
                        }
                    });
                }
                ClawEngineEvent::SystemMessage { text } => {
                    boxxy_claw_ui::add_system_message_row(&claw_store_events, id.clone(), text);
                }
                ClawEngineEvent::RequestScrollback {
                    max_lines,
                    offset_lines,
                    request_id,
                    ..
                } => {
                    let pane_inner = inner_clone.borrow();
                    let pane = pane_inner.terminal.clone();

                    // Hide the Claw badge while a TUI (alt screen) owns the terminal:
                    // scrollback requests during vim/less would otherwise be
                    // visually noisy without being useful.
                    if let Some(ind) = &pane_inner.claw_indicator {
                        ind.set_visible(!pane.is_alt_screen());
                    }

                    let max_lines = *max_lines;
                    let offset_lines = *offset_lines;
                    let request_id = *request_id;
                    let tx_reply = tx_engine_reply.clone();
                    gtk::glib::spawn_future_local(async move {
                        let content = pane
                            .get_text_snapshot(max_lines, offset_lines)
                            .await
                            .unwrap_or_else(|| {
                                "Error: Failed to fetch scrollback.".to_string()
                            });
                        let _ = tx_reply
                            .send(ClawMessage::ScrollbackReply { request_id, content })
                            .await;
                    });
                }
                ClawEngineEvent::ProposalResolved { .. } => {
                    popover_event_clone.hide();
                }
                ClawEngineEvent::RequestSpawnAgent { .. }
                | ClawEngineEvent::RequestCloseAgent { .. }
                | ClawEngineEvent::InjectKeystrokes { .. } => {
                    // Handled upstream by TerminalComponent / Window
                }
                ClawEngineEvent::ToolResult {
                    agent_name,
                    tool_name,
                    result,
                    usage,
                } => {
                    if let Some(usage) = usage {
                        total_tokens_for_events
                            .set(total_tokens_for_events.get() + usage_total(usage));
                    }
                    if tool_name == "list_processes" {
                        boxxy_claw_ui::add_process_list_row(
                            &claw_store_events,
                            id.clone(),
                            Some(agent_name.clone()),
                            result,
                            |_, _| {},
                        );
                    } else {
                        boxxy_claw_ui::add_tool_call_row(
                            &claw_store_events,
                            id.clone(),
                            Some(agent_name.clone()),
                            tool_name,
                            result,
                        );
                    }
                }
                ClawEngineEvent::TaskStatusChanged { tasks, .. } => {
                    let has_pending = tasks
                        .iter()
                        .any(|t| t.status == boxxy_claw_protocol::TaskStatus::Pending);
                    if let Some(ind) = &inner_clone.borrow().claw_indicator {
                        ind.set_has_tasks(has_pending);
                    }
                }
                ClawEngineEvent::RestoreHistory(rows) => {
                    // Bulk append history items to minimize UI layout passes
                    let mut items = Vec::with_capacity(rows.len());
                    for row in rows {
                        items.push(boxxy_claw_ui::row_object::ClawRowObject::new(row.clone()));
                    }
                    claw_store_events.remove_all();
                    claw_store_events.extend_from_slice(&items);
                }
                ClawEngineEvent::TaskCompleted { .. }
                | ClawEngineEvent::PushGlobalNotification { .. }
                | ClawEngineEvent::DelegatedTaskReply { .. } => {
                    // Routed by the window orchestrator / swarm layer, not the pane.
                }
            }

            cb_clone_events(PaneOutput::ClawEvent(id.clone(), event));
        }
    });

    (claw_popover, pending_proactive_diagnosis)
}
