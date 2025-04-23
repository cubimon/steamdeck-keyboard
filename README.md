# Steamdeck keyboard

An on screen keyboard tailored to the steamdeck with additional features.

Here are some goals:

- similar to steam's keyboard, e.g. touchpads for typing
- better defaults, e.g. transparent by default
- customizability of the keyboard layout similar to QMK and using CSS for visuals customization
- more "power user" oriented, e.g. better way to open/hide the keyboard

![image](./docs/screenshot.png "Screenshot showing the keyboard opened up in kate")

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- build devcontainer with `podman build . -t tauri-devcontainer`
- run with `RUST_LOG=debug ./src-tauri/target/release/bundle/appimage/virtual-keyboard-pad_0.1.0_amd64.AppImage`

### Common issues

- On strip issues build with `NO_STRIP=true npm run tauri build`

#### Find steam pid

```bash
ls -la */exe | grep -i steam$
```

### TODO

- build/install/autostart/bash into devcontainer/build from outside devcontainer
- improve deadzone (with timeout/slowly moving to goal/goal = average with deadzone)
- steamdeck specific split in main.ts
- split main.ts into different files
- gtk issue/black webview

