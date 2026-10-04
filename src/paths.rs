use std::io;
use std::path::{Component, Path, PathBuf};

pub fn validate_snapshot_id(id: &str) -> io::Result<()> {
    if id.is_empty()
        || id == "."
        || id == ".."
        || id.contains('/')
        || id.contains('\\')
        || id.contains(':')
        || id.chars().any(char::is_control)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid snapshot ID: {}", id),
        ));
    }
    Ok(())
}

pub fn relative_path(base: &Path, path: &Path) -> io::Result<String> {
    let relative = path
        .strip_prefix(base)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "path escaped repository"))?;
    let mut parts = Vec::new();

    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 path"))?,
            ),
            _ => return Err(invalid_relative_path(relative)),
        }
    }

    if parts.is_empty() {
        return Err(invalid_relative_path(relative));
    }

    let relative = parts.join("/");
    validate_relative_path(&relative)?;
    Ok(relative)
}

pub fn join_relative(root: &Path, relative: &str) -> io::Result<PathBuf> {
    validate_relative_path(relative)?;
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    Ok(path)
}

pub fn validate_relative_path(relative: &str) -> io::Result<()> {
    if relative.is_empty()
        || relative.contains('\\')
        || relative.contains(':')
        || relative.chars().any(char::is_control)
    {
        return Err(invalid_relative_path(Path::new(relative)));
    }

    for part in relative.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(invalid_relative_path(Path::new(relative)));
        }
    }

    if Path::new(relative).is_absolute() {
        return Err(invalid_relative_path(Path::new(relative)));
    }

    Ok(())
}

fn invalid_relative_path(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid relative path: {}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::{join_relative, validate_relative_path, validate_snapshot_id};
    use std::path::Path;

    #[test]
    fn accepts_safe_relative_paths() {
        assert_eq!(
            join_relative(Path::new("root"), "dir/file.bin").unwrap(),
            Path::new("root").join("dir").join("file.bin")
        );
    }

    #[test]
    fn rejects_unsafe_relative_paths() {
        for path in [
            "",
            "/tmp/file",
            "../file",
            "dir/../file",
            "dir\\file",
            "C:outside",
            "dir/a:b",
        ] {
            assert!(validate_relative_path(path).is_err(), "accepted {path}");
        }
    }

    #[test]
    fn rejects_snapshot_path_components() {
        for id in [
            "",
            ".",
            "..",
            "../snapshot",
            "dir/snapshot",
            "dir\\snapshot",
            "C:snapshot",
        ] {
            assert!(validate_snapshot_id(id).is_err(), "accepted {id}");
        }
    }
}
