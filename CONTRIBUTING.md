# Contributing

Thanks for helping. A few things keep the project easy to work on:

- **Core logic lives in `crates/core`** and must not depend on the browser.
  If something can be tested without a canvas (geometry, connectivity, rules,
  file format, exports), put it there and add a test.
- **Run the checks** before opening a pull request:
  ```sh
  cargo fmt --all
  cargo clippy --workspace --all-targets -- -D warnings
  cargo clippy -p easypro-app --target wasm32-unknown-unknown -- -D warnings
  cargo test --workspace
  ```
- **Browser smoke test**: build the app (`cd app && trunk build`) and run
  `cd tests/e2e && npm install && node smoke.mjs`.
- **New commands** go into the table in `app/src/commands.rs` so they appear
  in menus, the palette and the shortcut editor automatically.
- **File format changes** bump `FORMAT_VERSION` in `crates/core/src/project.rs`
  and add a migration step in `migrate`, so old files keep opening.
- Keep the UI close to EasyEDA Pro's layout; users moving from it should find
  things where they expect them.
