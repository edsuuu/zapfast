# Build environment for ZapFast's Debian package: Debian 12, as the upstream
# release builds use, so the binary needs glibc 2.36 and installs on Debian 12,
# Ubuntu 22.10 and newer.
FROM debian:12

RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      ca-certificates curl git build-essential pkg-config \
      libxkbcommon-dev libwayland-dev libgl1-mesa-dev libasound2-dev \
      libpipewire-0.3-dev clang libclang-dev cmake perl nasm \
      dpkg-dev fakeroot file \
 && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:$PATH

ARG RUST_VERSION=1.98.0
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path \
      --profile minimal --default-toolchain "$RUST_VERSION" \
      --component rustfmt --component clippy \
 && chmod -R a+rwX /opt/rustup /opt/cargo
# The right-to-left layout tests need a Hebrew/Arabic-capable sans.
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends fonts-dejavu-core && rm -rf /var/lib/apt/lists/*
