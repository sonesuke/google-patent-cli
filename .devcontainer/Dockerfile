FROM rust:1-slim-bookworm

# Set UTF-8 locale
ENV LANG=C.UTF-8 \
    LANGUAGE=C.UTF-8 \
    LC_ALL=C.UTF-8

# Install Node.js and other dependencies as root
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    build-essential \
    curl \
    git \
    pkg-config \
    libssl-dev \
    ripgrep \
    tmux \
    vim \
    jq \
    sudo \
    chromium \
    lcov \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/* \
    && ln -s /usr/bin/chromium /usr/bin/google-chrome

# Install Node.js LTS manually
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y nodejs \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*

# Install yq manually
RUN curl -L https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 -o /usr/local/bin/yq \
    && chmod +x /usr/local/bin/yq

# Ensure pre-installed tools are always in PATH even if CARGO_HOME is overridden at runtime
ENV PATH=/usr/local/cargo/bin:$PATH

# Install cargo-binstall and other rust tools as root (installs to /usr/local/cargo/bin)
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && rustup component add rustfmt clippy llvm-tools-preview \
    && cargo binstall -y cargo-audit cargo-llvm-cov
