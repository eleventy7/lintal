# Claude Code Docker Development Environment

A containerized environment for running Claude Code on the lintal project in YOLO mode.

## Quick Start (Isolated Mode - Recommended)

Use a git worktree to protect your main checkout from any damage:

```bash
# Create isolated worktree and start container
./docker/start-isolated.sh

# SSH into the container
ssh -p 2222 developer@localhost
# Password: developer

# Authenticate and start working
cd workspace/lintal
claude --dangerously-skip-permissions
```

This creates a worktree at `../lintal-claude-worktree` on a branch called `claude-work`. Your main checkout is completely protected.

**Review and merge:**
```bash
# See what changed
cd ../lintal-claude-worktree
git diff

# If good, merge to main
cd /path/to/lintal
git merge claude-work

# Or discard everything
git worktree remove ../lintal-claude-worktree
git branch -D claude-work
```

---

## Alternative: Direct Mount (Less Safe)

### Option 1: Docker Compose with Claude Max

```bash
# Build and start the container (mounts your ~/.claude for settings/plugins)
cd /path/to/lintal
docker compose -f docker/docker-compose.yml up -d --build

# SSH into the container
ssh -p 2222 developer@localhost
# Password: developer

# Inside the container, authenticate Claude Code (first time only)
cd workspace/lintal
claude --dangerously-skip-permissions
# This will open a browser URL - copy it to your host browser to authenticate
```

### Option 2: Using API Key

```bash
# Set your API key
export ANTHROPIC_API_KEY=sk-ant-your-key-here

# Build and start
docker compose -f docker/docker-compose.yml up -d --build

# SSH in and use directly
ssh -p 2222 developer@localhost
cd workspace/lintal
claude-yolo
```

### Option 3: Docker CLI

```bash
# Build the image
docker build -t lintal-claude-dev ./docker

# Run the container with ~/.claude mounted
docker run -d \
  --name lintal-dev \
  -p 2222:22 \
  -v $(pwd):/home/developer/workspace/lintal \
  -v ~/.claude:/home/developer/.claude \
  -v lintal-patches:/home/developer/patches \
  lintal-claude-dev

# SSH into the container
ssh -p 2222 developer@localhost
```

## Authentication

Claude Code supports multiple authentication methods:

### Claude Max/Pro (Browser OAuth)

Your `~/.claude` directory is mounted into the container, which preserves your settings and plugins. However, OAuth tokens are stored in the macOS Keychain, not in files.

**First-time setup in container:**
```bash
cd ~/workspace/lintal
claude --dangerously-skip-permissions
```

This will display a URL like:
```
Please visit: https://claude.ai/oauth/...
```

Copy this URL to your host browser, authenticate, and the token will be cached in the container.

### API Key

Set the environment variable before starting:
```bash
export ANTHROPIC_API_KEY=sk-ant-api-...
docker compose -f docker/docker-compose.yml up -d
```

## Credentials

- **SSH User:** developer
- **SSH Password:** developer
- **SSH Port:** 2222

## Working with Changes

### Inside the Container

Claude Code will make changes directly to the mounted workspace. You can:

1. **Review changes with git:**
   ```bash
   cd ~/workspace/lintal
   git status
   git diff
   ```

2. **Export changes as a patch:**
   ```bash
   export-changes
   # Creates patch file in ~/patches/
   ```

### On the Host Machine

Since the workspace is mounted, changes appear immediately on your host:

```bash
# Review changes
cd /path/to/lintal
git status
git diff

# Run tests
cargo test --all

# If happy, commit
git add -A
git commit -m "Changes from Claude Code session"
```

### Using Patches (Alternative Workflow)

If you prefer isolated changes:

```bash
# Copy patches from container volume
docker cp lintal-dev:/home/developer/patches/. ./patches/

# Review and apply
git apply patches/changes-main-20240101-120000.patch
```

## Environment Details

The container includes:

- **mise** - Tool version manager
- **Rust 1.92** - For building lintal
- **Java 25** - For testing against Java files
- **Python 3.12** - For benchmark scripts
- **Node.js 22** - For Claude Code
- **uv** - Python package manager
- **Claude Code** - Installed globally via npm

## Available Commands

Inside the container:

| Command | Description |
|---------|-------------|
| `claude-yolo` | Start Claude Code in YOLO mode |
| `export-changes` | Export git changes as patch file |
| `mise run build` | Build lintal in release mode |
| `mise run test` | Run all tests |
| `mise run check` | Run fmt and clippy |

## YOLO Mode

YOLO mode (`--dangerously-skip-permissions`) auto-approves all tool calls. This is useful for automated workflows but means Claude Code can:

- Read/write any file
- Execute any shell command
- Make git operations

**Only use in a sandboxed environment like this container.**

## Git Configuration

The container is configured for pull-only operations by default. To enable push:

1. Mount your SSH key:
   ```yaml
   volumes:
     - ~/.ssh/id_ed25519:/home/developer/.ssh/id_ed25519:ro
   ```

2. Or configure HTTPS credentials inside the container.

## Troubleshooting

### SSH Connection Refused

```bash
# Check container is running
docker ps | grep lintal-dev

# Check logs
docker logs lintal-dev
```

### Permission Denied on Mounted Files

```bash
# Fix ownership inside container
sudo chown -R developer:developer ~/workspace
```

### Claude Code Not Found

```bash
# Ensure mise environment is loaded
eval "$(mise activate bash)"

# Or use full path
/home/developer/.local/bin/mise exec -- claude
```

## Stopping the Container

```bash
# Stop
docker compose -f docker/docker-compose.yml down

# Or with Docker CLI
docker stop lintal-dev
docker rm lintal-dev
```

## Security Notes

- The container runs SSH with password authentication for convenience
- For production use, configure key-based authentication only
- ANTHROPIC_API_KEY is passed as an environment variable
- Consider using Docker secrets for the API key in production
