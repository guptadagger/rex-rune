# Test fixtures: who owns a test's files

A test that writes files owns them, and nothing else's.

## The rule

- A fixture creates a folder of its **own**, under the temporary directory the run was given
  (`TMPDIR`, through `std::env::temp_dir()`), with a name nothing else holds. Creation is
  exclusive (`create_dir`, which fails on an existing path), so a folder already on disk is
  never adopted, emptied or removed — a name that looks like a process id or a counter is not
  proof that this run made it.
- The folder is removed when its owner drops: on success, on a failed assertion and while the
  test unwinds. No test removes its files by hand for cleanup.
- A window relaunched on the same settings path **borrows** the first window's folder and owns
  none, so it cannot remove the file the first window is still using.
- Never change `TMPDIR` (or any environment variable) inside a test: tests run in parallel in
  one process. Set it in the environment of the run, which every process and child inherits.
- Nothing falls back to a shared `/tmp`: if the run's temporary directory is unusable, the
  fixture says so instead of writing somewhere else.

## Where it lives

- `src/app/scratch.rs`: `Folder` for the app's headless tests, and `claim`, the exclusive
  creation it uses. `Workspace::test_folder` holds the folder a test window's settings file is
  in (`#[cfg(test)]`, `None` for a relaunch and for production).
- `src/download/tests.rs`: `Temp` for the download core's tests, made the same way.
- `src/app/downloads_tests.rs`: each test's downloads folder is a `Folder`, not a bare path.

## Running the suite privately

The suite writes only where the run sends it. A private run keeps every file out of a shared
temporary directory and out of the real home:

```sh
RUN="$HOME/.cache/agent-ws/rex-tests"
mkdir -p "$RUN/tmp" "$RUN/home"
TMPDIR="$RUN/tmp" HOME="$RUN/home" cargo test --locked --all-features
```

`TMPDIR` and `HOME` are set for the whole run, not per test: the test process and every child
it starts inherit them, and the app's `~` display keeps its CI meaning (`TMPDIR` outside
`HOME`). After a clean run, `"$RUN/tmp"` is empty again.

## What the tests prove

- A folder is under the run's `TMPDIR`, two fixtures get different folders, and dropping one
  removes its files while a sibling folder stays.
- A name that is already on disk is refused, and what it holds stays.
- A test that fails after making its folder still removes it.
- A relaunched window leaves the first window's file alone, and the window that made the folder
  removes it when it drops.

## Remaining gaps

These still write PID-named files directly under `std::env::temp_dir()` and clean up by hand,
so they honour `TMPDIR` (a private run keeps their files private) but a failing test can leave
one behind: the settings, fonts, shortcuts and recent-path fixtures in `src/settings*`,
`src/app/font_settings_tests.rs`, `src/app/fonts.rs`, `src/connection/tests.rs`,
`src/ssh.rs`, `src/ssh/setup.rs`, `src/update.rs`, `src/upload.rs`, `src/terminal_theme.rs`
and `src/bin/update_probe.rs`, plus the screenshot fixtures in the capture tests. Moving one to
a `Folder`-style guard is a small change; it was not part of this one. `Cleanup::keep` in the
production installer is not a test fixture and is unchanged.
