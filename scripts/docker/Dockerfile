FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    curl \
    git \
    pkg-config \
    ca-certificates \
    file \
    patchelf \
    dpkg-dev \
    fakeroot \
    wget \
    xz-utils \
    p7zip-full \
    squashfs-tools \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    libjavascriptcoregtk-4.1-dev \
    libsoup-3.0-dev \
    libgtk-3-dev \
    librsvg2-dev \
    libayatana-appindicator3-dev \
    libxdo-dev \
    libmpv-dev \
    libclang-dev \
    clang \
    xdg-utils \
    && rm -rf /var/lib/apt/lists/*

# Node.js 22 + pnpm (corepack respects packageManager pin)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/* \
    && corepack enable

# Rust (stable)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal

ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /repo
