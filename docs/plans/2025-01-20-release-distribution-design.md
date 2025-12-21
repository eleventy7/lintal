# Release & Distribution Design

## Overview

Enable lintal distribution via GitHub Releases, Homebrew tap, and mise ubi.

## Platforms

| Target | Runner | Artifact |
|--------|--------|----------|
| `x86_64-apple-darwin` | `macos-13` | `lintal-x86_64-apple-darwin.tar.gz` |
| `aarch64-apple-darwin` | `macos-14` | `lintal-aarch64-apple-darwin.tar.gz` |
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` | `lintal-x86_64-unknown-linux-gnu.tar.gz` |

## Release Workflow

**Trigger:** Push of tags matching `v*`

**Process:**
1. Build release binary for each target in parallel
2. Create tarball with binary
3. Generate SHA256 checksums
4. Create GitHub Release with all artifacts attached

## Installation Methods

**mise (works immediately after release):**
```bash
mise use ubi:eleventy7/lintal
```

**Homebrew (requires tap setup):**
```bash
brew tap eleventy7/lintal
brew install lintal
```

## Release Process

```bash
mise run release 0.1.1
```

This validates the build, commits pending changes, tags, and pushes.

## Homebrew Tap Setup

1. Create repo `eleventy7/homebrew-lintal` on GitHub
2. Add `Formula/lintal.rb` from the template in this repo
3. Update SHA256 checksums after each release
