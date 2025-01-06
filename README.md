# Tauri + Vanilla TS

This template should help get you started developing with Tauri in vanilla HTML, CSS and Typescript.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- build devcontainer with `podman build . -t tauri-devcontainer`

### Common issues

- On strip issues build with `NO_STRIP=true npm run tauri build`


### TODO

- sometimes opened in background
- pausing steam process breaks touchpad driver? making it act like steam isn't running (like steam isn't accessing the hidraw device and defaulting to "lizard mode")
