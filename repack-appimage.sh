#!/bin/bash

wget -nc https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
chmod u+x appimagetool-x86_64.AppImage
cd src-tauri/target/release/bundle/appimage
./steamdeck-keyboard_0.1.0_amd64.AppImage --appimage-extract
cp /usr/lib/libwayland-*.so* squashfs-root/usr/lib
../../../../../appimagetool-x86_64.AppImage squashfs-root steamdeck-keyboard_0.1.0_amd64_patched.AppImage
rm -rf squashfs-root
