# Docker Sandboxes template for manga-theka.
# Reference: https://docs.docker.com/ai/sandboxes/customize/templates/
#
# `-docker` variant: the project's compose stack runs inside the sandbox, so it
# needs its own Docker Engine. It is also the `sbx run claude` default.
FROM docker/sandbox-templates:claude-code-docker

# --- system packages --------------------------------------------------------
# root for apt-get.
USER root

# Editor plus the native build deps the cargo tools below compile against.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        libssl-dev \
        pkg-config \
        vim \
    && rm -rf /var/lib/apt/lists/*

# Sourced before every shell (BASH_ENV, /etc/profile.d), so user and agent both
# get the tooling on PATH. Replaces `. $HOME/.cargo/env`, which only prepends
# that path and does not exist for root.
RUN echo 'export PATH="$HOME/.cargo/bin:$HOME/go/bin:$PATH"' >> /etc/sandbox-persistent.sh

# --- user-level tooling -----------------------------------------------------
# rustup, go install and cargo install write to $HOME: as root they land in
# /root, invisible in the sandbox.
USER agent

# Task runner for taskfile.yml.
RUN go install github.com/go-task/task/v3/cmd/task@latest \
    && go clean -modcache

# --no-modify-path: PATH is handled above, not in shell profiles.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path

# Cargo-installed CLIs the taskfile drives. --locked pins each to its published
# lockfile. RUN uses dash, which ignores BASH_ENV, so cargo needs sourcing.
RUN . "$HOME/.cargo/env" \
    && cargo install --locked \
        sqlx-cli \
        cargo-audit \
        cargo-machete \
        cargo-tarpaulin \
    && rm -rf "$HOME/.cargo/registry" "$HOME/.cargo/git"
