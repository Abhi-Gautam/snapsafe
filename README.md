# SnapSafe

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)

SnapSafe creates local directory snapshots. It uses BLAKE3 hashes to identify file contents and hard-links verified unchanged files to reduce storage use.

## Installation

With Cargo:

```bash
cargo install snapsafe
```

Rust 1.85 or newer is required.

Prebuilt binaries and checksums are available on the [GitHub releases page](https://github.com/Abhi-Gautam/snapsafe/releases).

To build from source:

```bash
git clone https://github.com/Abhi-Gautam/snapsafe.git
cd snapsafe
cargo build --release --locked
```

## Quick start

```bash
cd path/to/directory
snapsafe init
snapsafe snapshot --message "Initial state"

# Make changes, then capture another state.
snapsafe snapshot --message "Updated assets"

snapsafe list
snapsafe diff v1.0.0.0 v1.0.0.1
snapsafe verify
```

Restore a snapshot:

```bash
snapsafe restore v1.0.0.0
```

Restore creates a backup snapshot first, verifies the source, shows the change counts, and asks for confirmation. It restores managed files, directories, supported symlinks, permissions, and modification times. Managed paths absent from the selected snapshot are removed.

Preview a restore without changing files:

```bash
snapsafe restore v1.0.0.0 --dry-run
```

Use `--yes` for confirmed non-interactive operations:

```bash
snapsafe --yes restore v1.0.0.0
snapsafe --yes prune --keep-last 5
```

Without `--yes`, destructive commands reject non-interactive input.

## Commands

| Command | Description |
| --- | --- |
| `snapsafe init` | Initialize `.snapsafe` in the current directory |
| `snapsafe snapshot` | Create a content-hashed snapshot |
| `snapsafe list` | List snapshots |
| `snapsafe diff SNAPSHOT [SNAPSHOT]` | Compare snapshots |
| `snapsafe restore [SNAPSHOT]` | Restore a snapshot; defaults to the latest |
| `snapsafe verify [SNAPSHOT]` | Verify manifests and stored file contents |
| `snapsafe info [SNAPSHOT]` | Show snapshot statistics |
| `snapsafe prune` | Remove snapshots by count or age |
| `snapsafe tag [SNAPSHOT]` | Add, remove, or list tags |
| `snapsafe meta [SNAPSHOT]` | Set, remove, or list metadata |

Run `snapsafe COMMAND --help` for command options.

## Snapshot options

```bash
snapsafe snapshot \
  --version 2.0.0.0 \
  --message "Release candidate" \
  --tags release candidate \
  --meta build_id 12345
```

Versions contain one to four numeric components and are normalized to `vMAJOR.MINOR.PATCH.BUILD`.

## Pruning

```bash
snapsafe prune --keep-last 5 --dry-run
snapsafe prune --older-than 30d --dry-run

snapsafe --yes prune --keep-last 5
```

Count and age criteria are mutually exclusive. Durations support `d`, `h`, `m`, and `s`.

## Ignored names

`snapsafe init` creates `.snapsafeignore`. Each non-empty, non-comment line is a literal file or directory name ignored at every directory level. Glob and negation syntax are not supported.

`.snapsafe` and `.snapsafeignore` are always excluded from snapshot payloads and preserved during restore.

## Storage and integrity

Repository data is stored under `.snapsafe`:

```text
.snapsafe/
├── head_manifest.json
├── snapshots/
│   └── v1.0.0.0/
│       ├── manifest.json
│       └── data/
└── tmp/
```

Each regular file receives a streaming BLAKE3 digest. SnapSafe verifies the previous stored file before hard-linking it into a new snapshot. `verify` recalculates stored hashes and validates each snapshot manifest against the head manifest.

Snapshot creation, pruning, restore, and metadata updates use a repository lock. Snapshot and manifest writes are staged before commit, and interrupted snapshot or prune transactions are recovered on the next command.

## Repository migration

The first command run against a v1 repository migrates the complete repository to format v2 before continuing. Migration:

- hashes each unique hard-linked physical file once per run;
- writes resumable prepared manifests;
- keeps the v1 head until every snapshot is ready;
- writes the v2 head last;
- resumes after interruption.

Migration establishes hashes for the bytes currently stored in v1 snapshots. Run `snapsafe verify` after migration.

## Contributing

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo package --locked
```

## License

[MIT](LICENSE)
