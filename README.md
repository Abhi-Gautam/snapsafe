# Snap Safe

**Snap Safe** is a command-line tool written in Rust for creating efficient snapshots of directories.  
It is designed to help manage build artifacts and backups by leveraging hard links to avoid duplicating unchanged files.

## Features

- Incremental snapshots with minimal disk usage
- Support for version history, diffing snapshots, and restoration
- Optimized for build artifacts and backups of microservices

## Usage

After building your project, you can use Snap Safe to:
- Create a snapshot of the build output
- List previous snapshots and view details
- Compare differences between snapshots
- Restore or checkout previous versions for debugging or patching

## Commands

- `init [directory]` — Initialize Snap Safe in a directory
- `snapshot` — Create a new snapshot of the current state
- `list` — List all snapshots
- `diff [snapshot_id1] [snapshot_id2]` — Show differences between snapshots
- `restore [snapshot_id]` — Restore the state from a snapshot
- `config` — Configure Snap Safe settings

## Getting Started

1. Build the project using Cargo:
   ```bash
   cargo build
