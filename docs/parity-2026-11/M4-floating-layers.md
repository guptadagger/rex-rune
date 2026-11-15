# M4 — floating panes, slice 1: server actions and lifecycle (partial)

Part of **#136**, option B (a usable single-pane MVP). This record covers **slice 1 only**:
the layer verbs, their Commands and pane-menu rows, the cached floating-layer list and the
close confirmation. Pointer drag and edge resize are **slice 2** and are not here; #136 is not
finished by this slice. Verification: 2026-11-15, Pacific Standard Time, on `fcc1974`.

## Shipped

- **Float pane** (`pane.float`) moves the focused pane's block into a new floating layer of its
  own tab with `session.float_block`. The request states a centred rectangle
  (`[0.15, 0.12, 0.7, 0.7]` of the window); the server reports the real bounds, and the client
  never draws its own. The block, its program, its scrollback and its stream are the same one;
  focus does not change and any zoom ends (the server's rule).
- **New floating pane** (`pane.new_floating`) starts **one** terminal in a new floating layer
  of the tab on screen with `session.new_layer`, through the same creation path as Split: the
  host's shell, this client's declared theme, and the focused pane's folder. It names the
  active tab's `window_id` explicitly, so another client switching tabs cannot move it. It
  never creates a session, tab or server, and with no tab open it is unavailable.
- **Minimize floating layer** (`pane.minimize_layer`) sends one `session.set_layer_state` with
  `minimized`. When the layer holds the tab's **only visible panes** the row is disabled and
  says so ("Only visible panes"), because Rex keeps a window visible; nothing is sent and the
  view is not edited locally. The count uses every visible layer, not the zoom projection.
- **Restore and focus floating layer** (`pane.restore_layer`) sends one
  `session.set_layer_state` with `normal` and `focus: true`: the server restores, focuses,
  raises and activates the window in one revision. It works on an already-normal layer too, and
  is never composed from two requests.
- **Bring floating layer to front** (`pane.raise_layer`) sends `session.raise_layer` alone. It
  never focuses, and is named for raising only; on the frontmost layer it is disabled ("Already
  in front").
- **Return pane to tiles** (`pane.return_to_tiles`) sends `session.move_block` with
  `direction: horizontal`, `side: after`, `ratio: 0.5` and the anchor the client chose:
  **the first block of the tab's tiled layer in layout order, never the moving pane**. The
  server accepts an anchor in any layer; this client's policy is a tiled one in the same tab,
  so it will not detach a pane beside another floating one. With no tiled pane it is disabled
  ("No tiled pane beside it") and sends nothing — no placeholder pane, no restart, and #44's
  move-to-new-tab gap is not claimed as solved.
- **Close floating layer…** (`pane.close_layer`) asks first, naming the layer, every pane in it
  and the programs that end (a layer another client made with several panes is named pane by
  pane). **Keep running is the default**; only the confirmation sends one
  `session.close_layer`, and only while the captured layer is still the one the view shows. A
  layer that gained or lost a pane, disappeared, or belongs to a previous connection epoch
  closes nothing and says the layer changed — it is never retargeted. Closing a pane or tab is
  unchanged and still closes a pane, not a layer.
- **Floating layers list** (Commands, Pane group): the tab on screen's floating layers, back to
  front, named by their first pane (`alpha`, `beta +1`), and marked `Minimized, still running`
  when hidden. `Restore <name>` / `Show <name>` runs the atomic restore-and-focus for that
  captured layer. It is read from the cached view at open time — no background prefetch and no
  polling — and it is the way to bring back a minimized layer, whose pane has no menu of its
  own.
- **Pane menu**: one `Floating layer…` row (with the existing chevron) opens a small menu with
  the verbs that apply to that pane's layer. Six more rows in the pane menu itself pushed the
  destructive Close below the fold at 640×480, so they live one level in. On a host before API
  revision 13 the row is absent and the pane menu is exactly as it was.
- Every verb is **local only**: `Remote::Never`, no new default binding, and the remote
  allowed set is still the same eight view actions. Each has a Commands row, a title, a
  Settings → Shortcuts heading and, when bound, its effective hint (the unbound case keeps the
  row's own words in the hint column).

## The API 13 gate

The layer methods and `session.view`'s layers were verified live at **API revision 13 only**
(read-only contract check, `~/.cache/agent-ws/issue136-gate/RESULT.md`). Revisions 10–12 are
unproven, so `src/layers.rs` holds `MIN_API_REVISION = 13` and decides from the negotiated
revision:

- a connected host at revision 10–12 lists every layer row **disabled**, with `Needs Rex API
  13, not <revision>` in its hint column, and every verb reports "This host's Rex is API
  revision N; floating panes need revision 13. Nothing was changed.";
- no layer request is sent to such a host, and no local view is edited on its behalf;
- attachment, tiled rendering and every pre-existing action are unchanged there.

`MIN_API_REVISION` (10) and `API_REVISION` (13) are untouched: this is a capability gate on the
feature, not a new minimum for the client, and API 14 is not claimed (the client would refuse it
at negotiation). No generic capability manager was added; the check is
`layers::unsupported(connected, revision)`, a pure function used by every row and verb.

## What the client relies on from the server

Quoted from the live contract (identical to the pinned SDK):

- `float_block`: the new layer is in the block's own window, in front of its floating layers;
  focus and the active window do not change; a zoom ends; a pane already floating is
  `bad_request`.
- `new_layer`: `layout` always starts new blocks; the whole layout is created or none of it;
  the layer starts in front; `window_id` may be named, and with no window it is `conflict`.
- `set_layer_state`: `state` is `normal` or `minimized`; minimizing a window's last visible
  panes is `bad_request`; `focus: true` also raises, focuses the layer's last focused block and
  makes the window active in one revision; `focus` with `minimized` fails. There is no enum on
  `state`, so the client only ever sends the two documented values.
- `raise_layer`: never changes focus or bounds; the frontmost layer is a no-op.
- `move_block`: the anchor must be placed in a window; an emptied floating layer is removed and
  an emptied window closes; other splits keep their IDs; defaults are horizontal/after/0.5.
- `close_layer`: only a floating layer; it closes **every** block in the layer and returns once
  they have closed; closing the window's last blocks closes the window.
- `session.view`: layers back to front, the first tiled; `bounds` are window fractions; an
  unknown `kind` draws as floating and an unknown `state` as normal.

## Evidence

**Behavior tests** (real dispatch and the captured host commands, `src/app/floating_tests.rs`,
19 tests; pure view rules in `src/layers.rs`, 5 tests):

- every verb is local-only, titled, under the Panes heading, with a Commands row; the remote
  allowed set stays exactly eight;
- revisions 10, 11 and 12: every verb refused with the revision and **zero** host commands,
  rows disabled with the reason, pane menu unchanged;
- float: one `session.float_block` for the same block with exactly `{block_id, bounds,
  session_id}`, the centred rectangle, no new block/tab/session, view revision unchanged; an
  already-floating pane refused before any request;
- new floating: one `session.new_layer` with `{bounds, focus, layout, session_id, window_id}`,
  one terminal shell block, the focused pane's `cwd_from`, no split/tab; no tab → no row and no
  request;
- minimize: one `set_layer_state {layer_id, state}` without `focus`; the last visible layer
  disabled with "Only visible panes", nothing sent, cached state not changed;
- restore: one `set_layer_state {focus, layer_id, state}` with `normal` and `focus: true`, no
  second raise/focus call;
- raise: one `raise_layer {layer_id}`; the frontmost layer refused with "Already in front";
- return: one `move_block` anchored on the tab's first tiled pane with the documented defaults;
  no tiled pane → disabled, nothing sent, no invented anchor;
- close: the dialog names both panes of a two-pane layer, Keep running sends nothing, confirm
  sends one `close_layer`; a layer that gained a pane, disappeared or arrived on a new epoch
  closes nothing (and a disappeared/reconnected one refuses a restore too);
- a layer verb sends its one request and no other command: the pane's terminal worker receives
  nothing, its selection state is unchanged, and focus stays where the server has it;
- the list: this tab's layers only, back to front, `Restore beta`/`Show alpha` with the
  minimized detail, and `Restore beta` sends one atomic restore; the pane menu has one layer
  row, and its menu lists the verbs for the pane it names with the destructive close last.

**Native acceptance** (`scripts/floating-layers-private.py`, 27 checks; own disposable Rex
server on a private unix socket, private HOME/XDG/TMPDIR, private Xvfb, two clients of the built
release binary, real xdotool input): float preserved the block ID, the shell's PID
(`shell-pid=556203`) and its scrollback, typing still reached the PTY, and the second client
rendered the same floating pane (OCR); minimizing handed focus to a visible pane and the pane's
loop printed every second while hidden (7 s); the list named `Restore alpha — Minimized, still
running` while the action row said `Not floating`; restore came back focused and in front; New
floating started exactly one block in front at the requested rectangle; `Show <name>` focused
and raised a layer; focusing a lower layer did not raise it and Bring to front then changed only
the order (bounds and focus unchanged); the close confirmation named `beta`, Keep running left
the layer, and confirming closed only that layer's block; Return docked beside the real tiled
pane with the same block and removed the emptied layer; with no tiled pane the row read `No
tiled pane beside it` and confirming a match sent nothing (server revision unchanged). Receipts
(views, texts, captures, logs) are private under
`~/.cache/agent-ws/issue136-slice1-receipts/`; the run root was removed.

**Renders** (real fonts, `REX_TEST_SNAP_DIR`, one test per run, `floating_pane_menu_draw`,
`floating_layer_menu_draw`, `floating_commands_draw`): 1024×700 Comfortable and 640×480
Comfortable, Compact, and with the toolbar hidden, the pane menu, the layer menu and the
Commands list were read. The pane menu keeps its 16 rows with `Floating layer…` above
`Close pane…`; the layer menu fits at every size with `Close floating layer…` last; the Commands
list shows the layer verbs, `Float pane — Already floating` and the selected
`Restore g — Minimized, still running` without clipping.

## Limits, and what slice 2 still owns

- **No pointer drag or edge resize.** `resize_layer` is in the verified contract but no handle,
  move or resize pointer operation is offered or claimed here.
- **No multi-pane creation or group editing.** New floating layers have one pane; a multi-pane
  layer made by another client is listed, focused, raised and closed with all its panes named.
- Revisions 10–12 are unproven and therefore disabled; proving them needs older server binaries.
  The server's own refusals (last-visible minimize, tiled-layer close, out-of-window bounds) are
  documented rules; the client refuses the ones it can decide first and reports the rest as
  they arrive, without retrying.
- macOS and Windows were not run natively; their builds and the headless suite are covered by
  CI. No native OS windows, snapping, tile policy, #44 move-to-new-tab or #117.
- The client adds no layout cache, no optimistic edit, no replay after a connection failure and
  no new settings page or toolbar state.
