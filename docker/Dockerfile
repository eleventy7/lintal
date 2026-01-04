# Claude Code Development Environment for Lintal
# Usage:
#   docker build -t lintal-claude-dev ./docker
#   docker run -d --name lintal-dev -p 2222:22 -v lintal-patches:/patches lintal-claude-dev
#   ssh -p 2222 developer@localhost

FROM debian:bookworm-slim

# Avoid prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install essential packages
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    git \
    openssh-server \
    sudo \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    gnupg \
    vim \
    less \
    rsync \
    jq \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Configure SSH server
RUN mkdir /var/run/sshd && \
    sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin no/' /etc/ssh/sshd_config && \
    sed -i 's/#PasswordAuthentication yes/PasswordAuthentication yes/' /etc/ssh/sshd_config && \
    sed -i 's/#PubkeyAuthentication yes/PubkeyAuthentication yes/' /etc/ssh/sshd_config

# Create developer user
RUN useradd -m -s /bin/bash developer && \
    echo "developer:developer" | chpasswd && \
    usermod -aG sudo developer && \
    echo "developer ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Switch to developer user for mise and tool installation
USER developer
WORKDIR /home/developer

# Install mise
RUN curl https://mise.run | sh && \
    echo 'eval "$(/home/developer/.local/bin/mise activate bash)"' >> ~/.bashrc

# Set up mise path for this build
ENV PATH="/home/developer/.local/bin:${PATH}"

# Install Node.js (required for Claude Code) and project tools via mise
RUN /home/developer/.local/bin/mise use -g node@22 && \
    /home/developer/.local/bin/mise use -g rust@1.92 && \
    /home/developer/.local/bin/mise use -g java@25 && \
    /home/developer/.local/bin/mise use -g python@3.12 && \
    /home/developer/.local/bin/mise use -g uv@latest

# Install Claude Code globally via npm
RUN /home/developer/.local/bin/mise exec -- npm install -g @anthropic-ai/claude-code

# Create workspace and patches directories
RUN mkdir -p /home/developer/workspace /home/developer/patches

# Create helper scripts
RUN mkdir -p /home/developer/bin

# Script to export changes as patches
COPY --chown=developer:developer scripts/export-changes.sh /home/developer/bin/export-changes
RUN chmod +x /home/developer/bin/export-changes

# Script to start Claude Code in YOLO mode
COPY --chown=developer:developer scripts/claude-yolo.sh /home/developer/bin/claude-yolo
RUN chmod +x /home/developer/bin/claude-yolo

# Add bin to PATH
RUN echo 'export PATH="/home/developer/bin:$PATH"' >> ~/.bashrc

# Configure git (minimal config for pull-only operations)
RUN git config --global user.name "Claude Code (Container)" && \
    git config --global user.email "claude-container@localhost" && \
    git config --global init.defaultBranch main

# Note: ~/.claude will be mounted from host for auth/settings
# Create directory as fallback if not mounted
RUN mkdir -p /home/developer/.claude

# Switch back to root for SSH startup
USER root

# Create volume mount points
VOLUME ["/home/developer/workspace", "/home/developer/patches", "/home/developer/.claude"]

# Copy entrypoint script
COPY --chown=root:root scripts/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Expose SSH port
EXPOSE 22

# Use entrypoint to handle permissions before starting SSH
ENTRYPOINT ["/entrypoint.sh"]
CMD ["/usr/sbin/sshd", "-D"]
