# Snap Safe

**Snap Safe** is a command-line tool written in Rust for creating efficient snapshots of directories.  
It is designed to help manage build artifacts, large binaries, and backups by leveraging filesystem level operations. Using hard links to avoid duplicating unchanged files.

**TODOS**
- Figure out how folder copy works with hard links.

## Features

- **Efficient Incremental Snapshots:**  
  Snap Safe uses hard links to create snapshots that only duplicate changed files, saving disk space and speeding up backup operations.

- **Simple, Lightweight Workflow:**  
  With just a few commands (`init`, `snapshot`, `list`, `diff`, `restore`), Snap Safe offers a straightforward alternative to more complex systems like Git for file-based backups.

- **Custom Metadata (Planned):**  
  Future iterations will support storing custom metadata (e.g., build version, configuration, environment details) with each snapshot, making it easier to audit and manage snapshots.

## Differentiating Snap Safe from Git

While Git is a powerful distributed version control system optimized for source code, Snap Safe is tailored for scenarios such as:

- **Target Use Case & File Types:**  
  - **Git:** Optimized for text-based source code and small-to-medium repositories.  
  - **Snap Safe:** Designed for build artifacts, large binary files, and environments where most files remain unchanged between snapshots.

- **Snapshot Methodology:**  
  - **Git:** Uses commits, branching, and delta compression, which can add overhead when dealing with large binary files.  
  - **Snap Safe:** Creates complete snapshots using hard links to avoid duplicating unchanged files, making it extremely efficient for frequent backups.

- **User Interface and Workflow:**  
  - **Git:** Offers a complex set of commands for commits, branches, merges, etc.  
  - **Snap Safe:** Provides a minimal and straightforward command set for initialization and snapshots, focused purely on state backup and restoration.

- **Performance for Large or Frequent Changes:**  
  - **Git:** Can be less optimal when handling very large files or frequent snapshots in CI/CD pipelines.  
  - **Snap Safe:** Optimized for speed and storage efficiency by linking unchanged files.

## Potential Use Cases

1. **Build Artifact Management:**  
   - **Scenario:** A large microservices project where most build outputs remain unchanged between builds.
   - **Advantage:** Snap Safe efficiently creates snapshots with minimal disk space, enabling quick rollback if a new build introduces issues.

2. **Rapid Debugging and Hotfixes:**  
   - **Scenario:** A production system requires an immediate rollback to a stable build state for debugging.
   - **Advantage:** Snap Safe can quickly restore a previous snapshot without the overhead of complex Git operations.

3. **Large Binary and Non-Code Repositories:**  
   - **Scenario:** Managing large media files, firmware, or container images that are cumbersome for Git.
   - **Advantage:** Snap Safe handles large files efficiently, providing versioned backups without relying on delta compression.

4. **Simplified Backup System:**  
   - **Scenario:** An automated backup system for a build directory that requires fast, incremental snapshots.
   - **Advantage:** Snap Safe offers an out-of-the-box solution with minimal configuration and high performance for frequent snapshots.