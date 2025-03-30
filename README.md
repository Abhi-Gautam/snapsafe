# 🔍 Snap Safe

![GitHub License](https://img.shields.io/badge/license-MIT-blue.svg)
![Language](https://img.shields.io/badge/language-Rust-orange.svg)
![Status](https://img.shields.io/badge/status-Active-brightgreen.svg)

**Snap Safe** is a lightning-fast, lightweight command-line tool for creating efficient directory snapshots. Built in Rust, it leverages hard links to provide space-efficient backups with minimal overhead — perfect for managing build artifacts, large binaries, and environments where most files remain unchanged between versions.

## ✨ Features

- **🚀 Efficient Incremental Snapshots** - Uses hard links to avoid duplicating unchanged files, drastically reducing disk usage
- **⚡ Blazing Fast Performance** - Written in Rust for maximum speed and minimal resource consumption
- **💼 Metadata Management** - Attach custom metadata to snapshots, including tags and key-value properties
- **🔄 Simple Workflow** - Designed for clarity and ease of use with an intuitive command set
- **📊 Smart Analysis** - Built-in tools to compare, verify, and manage snapshots
- **📝 Text-based Diffing** - Specialized handling for configuration files (.json, .yaml, etc.)

## 📋 Table of Contents

- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Commands Reference](#-commands-reference)
- [Use Cases](#-use-cases)
- [How It Works](#-how-it-works)
- [Comparing with Other Tools](#-comparing-with-other-tools)
- [Contributing](#-contributing)
- [License](#-license)

## 📥 Installation

### From Source
```bash
git clone https://github.com/Abhi-Gautam/snapsafe
cd snapsafe
cargo build --release
# The binary will be at target/release/snapsafe
```

### Using Cargo
```bash
cargo install snapsafe
```

## 🚀 Quick Start

```bash
# Initialize a repository in your current directory
snapsafe init

# Create your first snapshot
snapsafe snapshot -m "Initial snapshot"

# Make some changes to files
echo "new content" > example.txt

# Create another snapshot 
snapsafe snapshot -m "Added example.txt"

# List all snapshots
snapsafe list

# Compare differences between snapshots
snapsafe diff v1.0.0.0 v1.0.0.1

# Restore a previous snapshot
snapsafe restore v1.0.0.0
```

## 🧰 Commands Reference

### Core Commands

| Command | Description |
|---------|-------------|
| `init` | Initialize Snap Safe in the current directory |
| `snapshot [-m MSG] [-t TAG]` | Create a new snapshot with optional message and tag |
| `list` | List all available snapshots |
| `diff [SNAPSHOT1] [SNAPSHOT2]` | Show differences between snapshots |
| `restore SNAPSHOT_ID` | Restore the working directory to a snapshot |

### Management Commands

| Command | Description |
|---------|-------------|
| `prune --keep-last N` | Keep only the N most recent snapshots |
| `prune --older-than DURATION` | Remove snapshots older than specified duration (e.g., "7d") |
| `verify [SNAPSHOT_ID]` | Verify the integrity of snapshots |
| `info SNAPSHOT_ID` | Display detailed information about a snapshot |

### Metadata Commands

| Command | Description |
|---------|-------------|
| `tag SNAPSHOT_ID --add TAGS...` | Add tags to a snapshot |
| `tag SNAPSHOT_ID --remove TAGS...` | Remove tags from a snapshot |
| `tag SNAPSHOT_ID --list` | List tags for a snapshot |
| `meta SNAPSHOT_ID --set KEY VALUE` | Set custom metadata for a snapshot |
| `meta SNAPSHOT_ID --remove KEY` | Remove custom metadata from a snapshot |
| `meta SNAPSHOT_ID --list` | List all custom metadata for a snapshot |

### Configuration

| Command | Description |
|---------|-------------|
| `config --set KEY VALUE` | Set a configuration option |
| `config --get KEY` | Get the value of a configuration option |
| `config --list` | List all configuration settings |

## 🎯 Use Cases

### Build Artifact Management

Snap Safe is ideal for CI/CD pipelines where repeated builds produce mostly unchanged artifacts:

```bash
# After building your project
snapsafe snapshot -m "Build #$CI_BUILD_NUMBER" --set build_id "$CI_BUILD_NUMBER"

# To restore a previous build for testing
snapsafe restore v1.2.3.4
```

### Deployment State Management

Track the state of deployed applications with version-tagged snapshots:

```bash
# Before an upgrade
snapsafe snapshot -m "Pre-upgrade state" --add pre-upgrade

# After an upgrade
snapsafe snapshot -m "Post-upgrade state" --add post-upgrade

# If issues arise, compare the differences
snapsafe diff $(snapsafe tag --list | grep pre-upgrade | cut -d' ' -f1) $(snapsafe tag --list | grep post-upgrade | cut -d' ' -f1)
```

### Large Binary Repository Management

For repositories with large binary files that aren't suited for Git:

```bash
# Initialize in your asset directory
cd assets/
snapsafe init

# After adding new assets
snapsafe snapshot -m "Added new character models"

# When you need to revert to a previous state
snapsafe restore v1.0.0.3
```

### Configuration Management

Track changes to configuration across environments:

```bash
# Store a snapshot of configuration
snapsafe snapshot -m "Production config" --add production
snapsafe snapshot -m "Staging config" --add staging

# View what's different between environments
snapsafe text-diff production staging
```

## 🔧 How It Works

Snap Safe creates efficient snapshots through a combination of techniques:

1. **Hard Links for Efficiency**:  
   Instead of duplicating unchanged files, Snap Safe creates hard links pointing to the same data blocks on disk, drastically reducing storage requirements.

2. **Snapshot Manifests**:  
   Each snapshot includes a detailed manifest tracking file metadata (paths, sizes, modification times).

3. **Metadata Tracking**:  
   Custom metadata and tags allow you to organize snapshots by version, environment, or any other criteria.

4. **Specialized Diffing**:  
   Between snapshots, Snap Safe can identify what files were added, removed, or modified.


## 📊 Comparing with Other Tools

### Snap Safe vs. Git

While Git is a powerful distributed version control system, Snap Safe addresses different needs:

| Feature | Snap Safe | Git |
|---------|-----------|-----|
| **Target files** | Build artifacts, large binaries | Source code, text files |
| **Storage efficiency for binaries** | High (hard linking) | Lower (delta compression) |
| **Learning curve** | Simple command set | Complex branching model |
| **Workflow complexity** | Minimal | Feature-rich |
| **Speed for large files** | Very fast | Can be slow |

### Snap Safe vs. Traditional Backup Tools

Compared to backup tools like rsync:

| Feature | Snap Safe | Traditional Backup Tools |
|---------|-----------|--------------------------|
| **Focus** | Version management | Data protection |
| **Metadata** | Rich, custom metadata | Basic file attributes |
| **Diffing capabilities** | Built-in | Limited or separate tools |
| **Designed for** | Dev/build environments | General backup scenarios |
| **Specialized file handling** | Yes (config files) | Typically no |

## 👨‍💻 Contributing

Contributions are welcome! Here's how you can help:

- **Report Bugs**: Open an issue describing the bug and how to reproduce it
- **Suggest Features**: Have an idea for a new feature? Open an issue to discuss it
- **Submit PRs**: Fix bugs or implement new features

Before submitting a PR, please:
1. Ensure your code follows the project's style
2. Add tests for new functionality
3. Make sure all tests pass

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

---

<p align="center">
  <sub>Built with ❤️ in Rust</sub>
</p>
