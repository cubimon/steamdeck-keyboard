FROM rust:bookworm

# Source: https://github.com/ivangabriele/docker-tauri/blob/main/dockerfiles/debian-bookworm-22.Dockerfile

# Install base utils
RUN apt-get update
RUN apt-get install -y \
  curl \
  psmisc

# Install Node.js
RUN curl -fsSL "https://deb.nodesource.com/setup_22.x" | bash -
RUN apt-get install -y nodejs

# Install Yarn
RUN corepack enable

# Install Tarpaulin
RUN cargo install cargo-tarpaulin

# Install Tauri v1 dependencies
# https://tauri.app/v1/guides/getting-started/prerequisites#setting-up-linux
RUN apt-get install -y \
  libwebkit2gtk-4.0-dev \
  build-essential \
  curl \
  wget \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

# Install Tauri v2 dependencies
# https://v2.tauri.app/start/prerequisites/#linux
RUN apt-get install -y \
  file \
  libwebkit2gtk-4.1-dev \
  libxdo-dev

# Install tauri-driver dependencies
RUN apt-get install -y \
  dbus-x11 \
  webkit2gtk-4.0-dev \
  webkit2gtk-driver \
  xvfb

# Install tauri-driver
# https://tauri.app/v1/guides/testing/webdriver/introduction#system-dependencies
RUN cargo install tauri-driver

# libudev-dev to access steamdeck's cirque usb touchpad device from hidraw
RUN apt-get install -y \
  libudev-dev

CMD ["/bin/bash"]
