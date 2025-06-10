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

## Build

```bash
devcontainer --docker-path podman --workspace-folder . up
devcontainer exec --docker-path podman --workspace-folder . bash
devcontainer exec --docker-path podman --workspace-folder . npm run tauri build
sudo bash repack-appimage.sh
```

## Autostart (Systemd unit)

move the built appimage to `/home/deck/AppImage/`:

```bash
mkdir -p /home/deck/AppImage
cp src-tauri/target/release/bundle/appimage/steamdeck-keyboard_0.1.0_amd64_patched.AppImage /home/deck/AppImage/steamdeck-keyboard_0.1.0_amd64.AppImage
```

run `systemctl edit --user --force --full steamdeck-keyboard` and insert:

```systemd
[Unit]
Description=Steamdeck keyboard
After=plasma-workspace.target

[Service]
Type=simple
ExecStart=/home/deck/AppImage/steamdeck-keyboard_0.1.0_amd64.AppImage
Restart=on-failure
Environment=DISPLAY=:0
Environment=XDG_CURRENT_DESKTOP=KDE

[Install]
WantedBy=default.target
```

then

- `systemctl enable --user steamdeck-keyboard`
- `systemctl start  --user steamdeck-keyboard`

## Common build issues

- On strip issues build with `NO_STRIP=true npm run tauri build`

## Find steam pid

```bash
ls -la */exe | grep -i steam$
```

## TODO

- permission build
- run html only
- improve deadzone (with timeout/slowly moving to goal/goal = average with deadzone)
- steamdeck specific split in main.ts
- split main.ts into different files
- gtk issue/black webview - still relevant with steamos 3.7 and wayland/xwayland?
