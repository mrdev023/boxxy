# Claw UI Crate (`boxxy-claw-ui`)

## Role

Every GTK widget that renders into the "Claw" page of the right-hand sidebar lives here. The crate is the glue between `boxxy-claw-protocol`'s DTOs and GTK4/libadwaita rendering.

Two independent consumers:
- `boxxy-window` embeds `ClawSidebarComponent` as one page of its right-hand `ViewStack`.
- `boxxy-terminal`'s pane module creates one message list per pane via `create_claw_message_list()` and pushes rows into it from the `ClawEngineEvent` stream.

## Structure

- **`sidebar::ClawSidebarComponent`** — outer window-scoped shell. One per `AppWindow`. Retargets itself at whichever pane is active via `set_history_widget(list, agent_name, pinned, web_search_enabled)`. Owns the status page (title / description / icon), the scrolled window that hosts the per-pane ListView, the token-usage label, the "Clear Screen" button (fires the caller-supplied soft-clear callback + drains the visible list without touching the DB), and the pending-tasks expander with per-task cancel buttons.
- **`create_claw_message_list()`** — builds the per-pane virtual history list. Returns `(gtk::ListView, gio::ListStore)`. Uses a `SignalListItemFactory` that constructs one icon/title/pane-label/`StructuredViewer`/cmd-label tree per visible slot and stashes it on the row via `boxxy_core_widgets::ObjectExtSafe`; `connect_bind` rebinds the same widgets per row rather than rebuilding, so long histories stay cheap.
- **`row_object::ClawRowObject`** — glib subclass wrapping a `PersistentClawRow`. Exposes a `content` property so GTK selection/sort bindings could attach in the future; `get_row()` returns a cloned DTO for rendering.
- **`ProcessListRenderer`** — custom `BlockRenderer` registered on the shared `ViewerRegistry`, used when a tool emits a `Custom { schema: "list_processes", .. }` block. Produces a columnar `AdwActionRow` list with PID / ellipsized name / (optional) disk-IO / CPU / memory, and falls back to the legacy tuple JSON schema so restored sessions keep rendering.
- **`get_claw_viewer_registry()`** — `ViewerRegistry::new_with_defaults()` plus `ProcessListRenderer`, wrapped in `Rc` (what `StructuredViewer::new` needs).

## Row-Append Helpers

Used by `boxxy-terminal`'s pane module to push rows into the per-pane store. The sidebar is strictly read-only; callbacks on the approval helpers are intentionally ignored.

- `add_system_message_row(store, pane_id, content)`
- `add_user_row(store, pane_id, content)`
- `add_diagnosis_row(store, pane_id, agent_name, content)`
- `add_suggested_row(store, pane_id, agent_name, diagnosis, command)`
- `add_tool_call_row(store, pane_id, agent_name, tool_name, result)`
- `add_process_list_row(store, pane_id, agent_name, result_json, on_kill_request)`
- `add_approval_row`, `add_file_write_approval_row`, `add_file_delete_approval_row`, `add_kill_process_approval_row`, `add_clipboard_get_approval_row`, `add_clipboard_set_approval_row` — format the proposal as a Diagnosis row and swallow their callback args. The real approval UI is the in-terminal popover.

## Visual Language

Each row shares the same layout (icon + heading + dim pane label + viewer + optional monospace command line) with per-variant styling:

| Row variant | Icon | Heading | Extra styling |
|---|---|---|---|
| SystemMessage | — (hidden) | "Models" | `accent` title, `system-message` row class |
| User | `boxxy-comic-bubble-symbolic` | "User Message" | — |
| Diagnosis | `boxxyclaw` | "Diagnosis" | `accent` icon |
| Suggested | `boxxy-dialog-warning-symbolic` | "Suggested Action" | `warning` icon; command shown in monospace |
| ProcessList | `boxxyclaw` | "Process List" | `accent` icon; custom renderer fills the viewer |
| ToolCall | `boxxy-build-circle-symbolic` | `Used tool: {name}` | `accent` icon; viewer hidden (compact row) |
| Command | `utilities-terminal-symbolic` | "Command Execution" or "Command Failed (Exit N)" | `error` icon when exit != 0 |

## Dependencies

`boxxy-claw-protocol` (DTOs), `boxxy-viewer` (`StructuredViewer` + renderer trait), `boxxy-core-widgets` (`ObjectExtSafe`), `gtk4`, `libadwaita`, `uuid`, `chrono`. Explicitly **not** `boxxy-claw` — the engine knows nothing about GTK and this crate knows nothing about reasoning.
