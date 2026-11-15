//! Issue #129: the active key mode is named for as long as it lasts, and no longer.
use super::pane_verb_tests::{framed_terminal, rect};
use super::*;
use crate::app::actions;
use rex_rune::keymap::{Overrides, Resolved};
use rexrs::api::client::{Binding, KeymapResponse, Mode};
use rune::{PhysicalInput, PhysicalKeyCode as Code};
use serde_json::json;
use std::sync::Arc;

const LONG: &str = "a-mode-name-too-long-for-a-narrow-toolbar";

fn settle(rt: &mut Runtime<Workspace>) {
    for _ in 0..90 {
        rt.frame(1. / 60.);
    }
}

/// The host's keymap with modes to nest, a once mode, an ordinary key and a long name.
fn with_modes(ws: &mut Workspace, exclusive: bool) {
    let reply = KeymapResponse {
        bindings: vec![
            Binding::new(["ctrl+r"], "client.mode.enter").args(json!({"name":"resize"})),
            Binding::new(["ctrl+l"], "client.mode.enter").args(json!({"name":LONG})),
            Binding::new(["left"], "pane.resize")
                .args(json!({"direction":"left","amount":1}))
                .mode("resize"),
            Binding::new(["q"], "client.mode.exit_all").mode("resize"),
            Binding::new(["n"], "client.mode.enter")
                .args(json!({"name":"nested"}))
                .mode("resize"),
            Binding::new(["a"], "client.mode.enter")
                .args(json!({"name":"once","once":true}))
                .mode("resize"),
            Binding::new(["x"], "client.mode.exit").mode("nested"),
            Binding::new(["right"], "pane.resize")
                .args(json!({"direction":"right","amount":1}))
                .mode("once"),
            Binding::new(["ctrl+b", "c"], "pane.balance"),
            Binding::new(["z"], "pane.close"),
        ],
        modes: vec![
            Mode {
                name: "resize".into(),
                exclusive,
                blocked: vec![Some(vec!["z".into()])],
            },
            Mode {
                name: "nested".into(),
                ..Default::default()
            },
            Mode {
                name: "once".into(),
                ..Default::default()
            },
            Mode {
                name: LONG.into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    ws.hosts[0].model.keymap = Some(Arc::new(rex_rune::connection::keymap::Snapshot {
        generation: ws.keymap_generation,
        resolved: Resolved::from_server(&reply, &Overrides::default()),
        reply,
    }));
    ws.hosts[0].model.connected = true;
}

/// A Ctrl chord as the window delivers it: a press on the physical key.
fn ctrl(rt: &mut Runtime<Workspace>, code: Code, character: char) {
    let ev = KeyEvent {
        key: Key::Command(character),
        mods: Mods {
            ctrl: true,
            ..Mods::default()
        },
    };
    rt.physical_key_input(
        PhysicalInput::Key {
            key: code,
            mods: ev.mods,
            pressed: true,
            repeat: false,
        },
        ev,
        None,
    );
}

/// The mode name as the chip draws it: none when no mode is active.
fn chip_text(rt: &Runtime<Workspace>) -> Option<String> {
    has(rt, "mode-chip").then(|| element_text(rt, "mode-name").join(""))
}

fn chip(rt: &Runtime<Workspace>) -> rune::ElementInfo {
    rt.elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some("mode-chip"))
        .expect("the mode chip")
}

#[test]
fn no_mode_draws_no_chip_and_reserves_no_space() {
    let (mut ws, _rx) = tabbed_workspace("split");
    with_modes(&mut ws, false);
    let mut rt = runtime(ws);
    settle(&mut rt);
    assert!(!has(&rt, "mode-chip"));
    assert!(
        rect(&rt, "title-drag")[2] >= 48.,
        "the drag space is the same without a chip"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_mode_names_itself_for_its_whole_life_and_leave_ends_it() {
    let (mut ws, rx) = tabbed_workspace("split");
    with_modes(&mut ws, false);
    let (sender, sent) = flume::unbounded();
    live_terminal(&mut ws, "block:b", &sender);
    let mut rt = runtime(ws);
    settle(&mut rt);
    assert_eq!(chip_text(&rt), None);
    let focus = focused(&rt);

    ctrl(&mut rt, Code::KeyR, 'r');
    assert_eq!(
        chip_text(&rt),
        None,
        "nothing is drawn before the frame that sees the mode"
    );
    rt.frame(1. / 60.);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));
    assert_eq!(focused(&rt), focus, "the chip takes no focus");
    assert_eq!(chip(&rt).label.as_deref(), Some("Key mode: resize"));
    assert_eq!(
        chip(&rt).help.as_deref(),
        Some("Key mode resize · Escape to leave")
    );

    // A notice would have gone by now; the mode has not.
    for _ in 0..6 * 60 {
        rt.frame(1. / 60.);
    }
    assert_eq!(
        chip_text(&rt).as_deref(),
        Some("resize"),
        "the name lasts as long as the mode, not as long as a notice"
    );

    press(&mut rt, Key::Escape);
    assert_eq!(chip_text(&rt), None, "Escape left, and the chip with it");

    // Escape to leave again, this time from the host's own exit-all binding.
    ctrl(&mut rt, Code::KeyR, 'r');
    rt.frame(1. / 60.);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));
    press(&mut rt, Key::Text("q".into()));
    assert_eq!(chip_text(&rt), None);
    assert!(
        requests(&rx).is_empty(),
        "leaving a mode asks the server nothing"
    );
    assert!(sent_keys(&sent).is_empty(), "and types nothing");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn nested_and_once_modes_name_the_innermost_and_fall_back_to_the_outer() {
    let (mut ws, rx) = tabbed_workspace("split");
    with_modes(&mut ws, false);
    let mut rt = runtime(ws);
    settle(&mut rt);

    ctrl(&mut rt, Code::KeyR, 'r');
    rt.frame(1. / 60.);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));
    press(&mut rt, Key::Text("n".into()));
    assert_eq!(chip_text(&rt).as_deref(), Some("nested"));
    press(&mut rt, Key::Text("x".into()));
    assert_eq!(
        chip_text(&rt).as_deref(),
        Some("resize"),
        "the outer mode is named again after the inner one leaves"
    );

    // A once mode is named like any other, and gone the moment its one key has run.
    press(&mut rt, Key::Text("a".into()));
    assert_eq!(chip_text(&rt).as_deref(), Some("once"));
    press(&mut rt, Key::Right);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));
    assert_eq!(requests(&rx)[0].0, "session.resize_pane");

    // Its one key may also be Escape.
    press(&mut rt, Key::Text("a".into()));
    press(&mut rt, Key::Escape);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));

    press(&mut rt, Key::Text("q".into()));
    assert_eq!(chip_text(&rt), None);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_pending_sequence_and_a_mode_share_the_bar_in_that_order() {
    let (mut ws, _rx) = tabbed_workspace("split");
    with_modes(&mut ws, false);
    let mut rt = runtime(ws);
    settle(&mut rt);
    ctrl(&mut rt, Code::KeyR, 'r');
    rt.frame(1. / 60.);
    ctrl(&mut rt, Code::KeyB, 'b');
    rt.frame(1. / 60.);
    assert_eq!(rt.state.bindings.sequence.depth, 1);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));
    assert!(has(&rt, "pending-sequence"), "the sequence chip too");
    assert!(
        rect(&rt, "mode-chip")[0] < rect(&rt, "pending-sequence")[0],
        "the mode comes before the sequence"
    );
    press(&mut rt, Key::Escape);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));
    assert!(
        !has(&rt, "pending-sequence"),
        "Escape cancelled only the prefix"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn an_exclusive_mode_keeps_its_name_while_it_swallows_keys() {
    let (mut ws, _rx) = tabbed_workspace("split");
    with_modes(&mut ws, true);
    let (sender, sent) = flume::unbounded();
    live_terminal(&mut ws, "block:b", &sender);
    let mut rt = runtime(ws);
    settle(&mut rt);
    ctrl(&mut rt, Code::KeyR, 'r');
    rt.frame(1. / 60.);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));
    press(&mut rt, Key::Text("w".into()));
    assert!(sent_keys(&sent).is_empty(), "the mode takes the plain key");
    assert_eq!(
        chip_text(&rt).as_deref(),
        Some("resize"),
        "a key that does nothing leaves the name that explains why"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn hint_off_keeps_the_mode_name_and_a_plain_key_still_reaches_the_program() {
    let (mut ws, _rx) = tabbed_workspace("split");
    ws.settings.terminal.key_sequence_hints = false;
    with_modes(&mut ws, false);
    let (sender, sent) = flume::unbounded();
    live_terminal(&mut ws, "block:b", &sender);
    let mut rt = runtime(ws);
    settle(&mut rt);
    ctrl(&mut rt, Code::KeyR, 'r');
    rt.frame(1. / 60.);
    assert_eq!(
        chip_text(&rt).as_deref(),
        Some("resize"),
        "the mode is not a key sequence hint"
    );
    // A non-exclusive mode falls through, so ordinary typing is unchanged.
    press(&mut rt, Key::Text("w".into()));
    assert_eq!(sent_keys(&sent).len(), 1);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn a_toolbar_with_no_row_floats_the_mode_and_never_moves_the_terminal() {
    use rex_rune::settings::interface::ToolbarBehavior;
    for toolbar in [ToolbarBehavior::Hidden, ToolbarBehavior::OnHover] {
        let (mut ws, _rx) = tabbed_workspace("single");
        ws.settings.interface.toolbar = toolbar;
        with_modes(&mut ws, false);
        let (sender, _sent) = flume::unbounded();
        framed_terminal(&mut ws, "block:a", &sender, b"$ make\r\nok");
        let mut rt = runtime(ws);
        settle(&mut rt);
        let grid = rect(&rt, "grid:block:a");
        assert!(
            !has(&rt, "toolbar") && !has(&rt, "mode-chip"),
            "{toolbar:?}"
        );
        ctrl(&mut rt, Code::KeyR, 'r');
        rt.frame(1. / 60.);
        assert!(!has(&rt, "toolbar"), "{toolbar:?}: no row comes back");
        assert_eq!(chip_text(&rt).as_deref(), Some("resize"), "{toolbar:?}");
        let float = rect(&rt, "pending-sequence-float");
        let chip = rect(&rt, "mode-chip");
        assert_eq!((float[0], float[1]), (10., 8.), "{toolbar:?}");
        assert_eq!(
            (chip[0], chip[1]),
            (float[0], float[1]),
            "{toolbar:?}: the chip is the float's first"
        );
        assert_eq!(
            rect(&rt, "grid:block:a"),
            grid,
            "{toolbar:?}: floating status reserves no terminal space"
        );
        press(&mut rt, Key::Escape);
        assert!(!has(&rt, "pending-sequence-float"), "{toolbar:?}");
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn a_long_name_truncates_at_both_sizes_and_leaves_the_controls_whole() {
    for size in [(1024., 700.), (640., 480.)] {
        let (mut ws, _rx) = tabbed_workspace("split");
        with_modes(&mut ws, false);
        let (sender, _sent) = flume::unbounded();
        for block in ["block:b", "block:c"] {
            framed_terminal(&mut ws, block, &sender, b"$ cargo test\r\nok");
        }
        let mut rt = runtime_at(ws, size);
        settle(&mut rt);
        let grid = rect(&rt, "grid:block:b");
        ctrl(&mut rt, Code::KeyR, 'r');
        rt.frame(1. / 60.);
        // A non-exclusive mode falls through to the ordinary map, which enters the long one.
        ctrl(&mut rt, Code::KeyL, 'l');
        rt.frame(1. / 60.);
        assert_eq!(chip_text(&rt).as_deref(), Some(LONG), "{size:?}");
        let name = chip(&rt).label.clone().unwrap_or_default();
        assert_eq!(name, format!("Key mode: {LONG}"), "{size:?}");
        assert_eq!(
            chip(&rt).help.as_deref(),
            Some(format!("Key mode {LONG} · Escape to leave").as_str()),
            "{size:?}: the tooltip keeps the name whole"
        );
        let chip_rect = rect(&rt, "mode-chip");
        assert!(
            chip_rect[2] <= 150.5,
            "{size:?}: the chip is bounded: {chip_rect:?}"
        );
        assert_eq!(
            rect(&rt, "grid:block:b"),
            grid,
            "{size:?}: the terminal keeps its rows"
        );
        let settings = rect(&rt, "settings");
        assert!(
            chip_rect[0] + chip_rect[2] <= settings[0] + 0.5,
            "{size:?}: the chip clears the bar's controls: {chip_rect:?} {settings:?}"
        );
        let close = rect(&rt, "close-app");
        assert!(
            close[0] + close[2] <= size.0 + 0.5,
            "{size:?}: Close app keeps its place"
        );
        for key in ["commands", "new-tab", "settings", "close-app"] {
            let r = rect(&rt, key);
            assert!(r[2] > 0. && r[0] >= 0., "{size:?}: {key} is drawn: {r:?}");
        }
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn switching_host_session_keymap_or_connection_ends_the_name_without_a_keypress() {
    let (mut ws, _rx) = tabbed_workspace("split");
    with_modes(&mut ws, false);
    let mut rt = runtime(ws);
    settle(&mut rt);
    ctrl(&mut rt, Code::KeyR, 'r');
    rt.frame(1. / 60.);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));

    // A pane may change under a mode; its host, session and keymap may not.
    rt.update(|s, _| {
        let view = s.hosts[0].model.views.get_mut("s").unwrap();
        let window = view
            .windows
            .iter_mut()
            .find(|w| w.window_id.as_deref() == Some("split"))
            .unwrap();
        window.focused_block_id = Some("block:c".into());
    });
    settle(&mut rt);
    assert_eq!(
        chip_text(&rt).as_deref(),
        Some("resize"),
        "another pane keeps the mode"
    );

    rt.update(|s, _| s.hosts[0].model.selected = Some("elsewhere".into()));
    settle(&mut rt);
    assert_eq!(chip_text(&rt), None, "another session ends it at once");

    rt.update(|s, _| s.hosts[0].model.selected = Some("s".into()));
    settle(&mut rt);
    ctrl(&mut rt, Code::KeyR, 'r');
    rt.frame(1. / 60.);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));
    rt.update(|s, _| s.hosts[0].model.connected = false);
    settle(&mut rt);
    assert_eq!(chip_text(&rt), None, "a lost connection ends it at once");

    rt.update(|s, _| s.hosts[0].model.connected = true);
    settle(&mut rt);
    ctrl(&mut rt, Code::KeyR, 'r');
    rt.frame(1. / 60.);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));
    rt.update(|s, _| s.keymap_generation += 1);
    settle(&mut rt);
    assert_eq!(chip_text(&rt), None, "another keymap ends it at once");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn another_host_does_not_keep_the_name_and_reload_clears_it() {
    let mut ws = workspace_hosts(2);
    with_modes(&mut ws, false);
    let mut rt = runtime(ws);
    settle(&mut rt);
    ctrl(&mut rt, Code::KeyR, 'r');
    rt.frame(1. / 60.);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));
    rt.update(|s, tx| s.activate_host(1, tx));
    settle(&mut rt);
    assert_eq!(
        chip_text(&rt),
        None,
        "the first host's mode is not the second host's"
    );

    rt.update(|s, tx| s.activate_host(0, tx));
    settle(&mut rt);
    ctrl(&mut rt, Code::KeyR, 'r');
    rt.frame(1. / 60.);
    assert_eq!(chip_text(&rt).as_deref(), Some("resize"));
    rt.update(|s, tx| {
        assert!(actions::run(
            s,
            "client.config.reload",
            &serde_json::Value::Null,
            tx
        ));
    });
    settle(&mut rt);
    assert_eq!(chip_text(&rt), None, "Reload config ends every mode");
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn the_title_row_and_its_strip_name_the_mode_too() {
    use rex_rune::settings::interface::ToolbarBehavior;
    use rune::ChromeInsets;
    for toolbar in [ToolbarBehavior::Always, ToolbarBehavior::Hidden] {
        let (mut ws, _rx) = tabbed_workspace("split");
        ws.unified_title = true;
        ws.settings.interface.toolbar = toolbar;
        with_modes(&mut ws, false);
        let (sender, _sent) = flume::unbounded();
        framed_terminal(&mut ws, "block:b", &sender, b"$ make");
        let mut rt = runtime_at(ws, (1024., 700.));
        let mut insets = ChromeInsets::default();
        insets.leading = 76.;
        insets.height = crate::app::platform::chrome::TITLE_ROW;
        rt.set_chrome_insets(insets);
        settle(&mut rt);
        ctrl(&mut rt, Code::KeyR, 'r');
        settle(&mut rt);
        let row = if toolbar == ToolbarBehavior::Always {
            "toolbar"
        } else {
            "title-strip"
        };
        assert!(has(&rt, row), "{toolbar:?}: {row}");
        assert_eq!(chip_text(&rt).as_deref(), Some("resize"), "{toolbar:?}");
        let (chip, bar) = (rect(&rt, "mode-chip"), rect(&rt, row));
        assert!(
            chip[0] >= bar[0]
                && chip[0] + chip[2] <= bar[0] + bar[2]
                && chip[1] >= bar[1]
                && chip[1] + chip[3] <= bar[1] + bar[3],
            "{toolbar:?}: the chip is in the row: {chip:?} {bar:?}"
        );
        press(&mut rt, Key::Escape);
        assert_eq!(chip_text(&rt), None, "{toolbar:?}");
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

/// Real fonts are process-global: run each capture test on its own with `REX_TEST_SNAP_DIR`.
#[test]
fn mode_chip_draws() {
    draw("mode-chip", (1024., 700.));
}

/// Real fonts are process-global: run each capture test on its own with `REX_TEST_SNAP_DIR`.
#[test]
fn mode_chip_draws_at_the_minimum_size() {
    draw("mode-chip-minimum", (640., 480.));
}

fn draw(prefix: &str, size: (f32, f32)) {
    let Some(mut camera) = super::pane_verb_tests::Camera::new() else {
        return;
    };
    let _families = rune::text::with_families("Inter", "IBM Plex Mono");
    let mut rt = painted_runtime(size, false);
    camera.snap(&mut rt, &format!("{prefix}-none"));
    ctrl(&mut rt, Code::KeyR, 'r');
    settle(&mut rt);
    camera.snap(&mut rt, &format!("{prefix}-resize"));
    ctrl(&mut rt, Code::KeyL, 'l');
    settle(&mut rt);
    camera.snap(&mut rt, &format!("{prefix}-long-name"));
    std::fs::remove_file(&rt.state.settings_path).ok();

    // The title row macOS draws instead of a separate toolbar row.
    let mut title = painted_runtime(size, true);
    ctrl(&mut title, Code::KeyR, 'r');
    settle(&mut title);
    camera.snap(&mut title, &format!("{prefix}-title-row"));
    std::fs::remove_file(&title.state.settings_path).ok();
}

fn painted_runtime(size: (f32, f32), unified_title: bool) -> Runtime<Workspace> {
    let fonts = crate::app::fonts::load("IBM Plex Mono", &[]);
    let (mut ws, _rx) = tabbed_workspace("split");
    ws.unified_title = unified_title;
    ws.title_inset = if unified_title { 76. } else { 0. };
    with_modes(&mut ws, false);
    let (sender, _sent) = flume::unbounded();
    for block in ["block:b", "block:c"] {
        super::pane_verb_tests::framed_terminal(
            &mut ws,
            block,
            &sender,
            b"$ cargo test\r\nrunning 12 tests",
        );
    }
    let mut rt = Runtime::with_text(
        ws,
        view,
        size,
        Env::new(true, 1.),
        rune::text::TextSystem::new(fonts.files, fonts.families),
    );
    rt.on_key = Some(crate::app::input::on_key);
    settle(&mut rt);
    rt
}
