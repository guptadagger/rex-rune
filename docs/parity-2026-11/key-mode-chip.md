# Active key-mode chip

Issue [#129](https://github.com/shuv1337/rex-rune/issues/129): Rex key modes were routed
correctly but only a six-second notice named one, so while a mode lasted the client did not
say which one was active. An exclusive mode swallows the keys it does not bind, so the
terminal could look unresponsive with nothing on screen to explain it.

## As built

- **One chip, from the existing stack.** The toolbar's live chips now begin with the key
  mode: a quiet 28 pt chip, `mode resize`, naming the innermost entry of
  `bindings.modes` (`Workspace::key_mode`). Nothing is cached or duplicated; the chip is
  derived from the same state dispatch uses, so nested modes show the inner one and pop back
  to the outer one, a once mode is named until its key runs it, and `Escape`, Exit key mode,
  Exit all key modes, reload and automatic clearing remove it on the next frame.
- **The same lifetime rule as dispatch.** `input.rs` ends modes when the host, session,
  keymap generation, server revision or connection changes. That predicate moved into
  `bindings::Context::owns_modes` / `keeps_modes`, and `key_mode` asks it of the state on
  screen, so a host switch, a session switch, a reconnect or a new keymap removes the name
  straight away instead of leaving the old host's mode on the chip until the next keypress.
  A pane, a panel or focus changing does not end a mode, as before.
- **Placement.** In the toolbar row and in the macOS title row the chip sits after the tab
  track, before the pending sequence; with no toolbar row it is the first chip on the
  existing floating status surface at the top left, and the title strip draws it where the
  toolbar has no row. It is drawn whether or not **Show key sequence hints** is on, because a
  mode is not a key sequence hint.
- **Bounded and accessible.** At most 150 pt wide and 28 pt high, the name truncates with an
  ellipsis while the element's accessible label (`Key mode: <name>`) and tooltip
  (`Key mode <name> · Escape to leave`) keep it whole. It takes no focus, has no press
  handler, and reserves no space when no mode is active.
- **The entry notice is gone.** The chip replaces "Key mode: … · Escape to leave", which only
  ever described the start of a mode. An unknown mode name still gets its notice, since
  nothing was entered.

Per-mode palettes, outline colours or a settings switch are deliberately absent: the issue
listed them as optional, and the chip is chrome like the pending sequence.

## Checks

`src/app/key_mode_tests.rs`, run against the real headless UI runtime:

- no mode: no chip, and no room taken from the drag space;
- a mode appears on the frame that follows the key (not before), stays past the old notice's
  six seconds, and leaves with `Escape` or an inherited `client.mode.exit_all` binding;
- nested modes name the inner one and fall back to the outer one; a once mode is named until
  its one key runs, including the key that runs a pane action;
- an exclusive mode keeps its name while it swallows a plain key, and a non-exclusive one
  still lets that key reach the program;
- key sequence hints off keeps the chip;
- Toolbar Hidden and Show on hover: no row returns, the chip is the floating status
  surface's first chip, and the terminal grid does not move;
- a long name truncates at 1024×700 and 640×480 with the whole name in the tooltip, the
  chip clears Settings and Close app, and the terminal grid keeps its rect;
- the unified macOS title row and its title strip name the mode in the row;
- a pane change keeps the mode; another session, a lost connection, another keymap, another
  host and Reload config all end it without a keypress;
- the chip takes no focus, and existing dispatch/release behaviour is covered unchanged by
  `compatibility_tests.rs`.

Real-font captures (`REX_TEST_SNAP_DIR`, one test at a time as fonts are process-global):
`mode_chip_draws` (1024×700) and `mode_chip_draws_at_the_minimum_size` (640×480) write
no mode, `resize`, a long truncated name, and the title row.

## Limits

- Verified locally on Linux only (check, Clippy, fmt, the headless suite and the captures
  above). The macOS title row is exercised headlessly through `unified_title`, as other
  title-row tests do; native macOS and Windows were not run here — the pull request's CI
  covers their builds and the shared suite.
- At 640 pt the bar's live chips take their room from the tab track, which scrolls to its
  arrows: at that width this chip leaves one pill's width unreadable, as a receiving-download
  or pending-sequence chip already does. Tabs remain reachable by their arrows and Commands,
  and the chip disappears with the mode.
- The chip reflects only this client's mode stack. A mode another client entered, and a mode
  a host's keymap defines but nobody entered, are not named.
