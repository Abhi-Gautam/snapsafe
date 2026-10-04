use std::io::{self, IsTerminal, Write};

pub fn confirm(prompt: &str, assume_yes: bool) -> io::Result<bool> {
    if assume_yes {
        return Ok(true);
    }

    if !io::stdin().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "confirmation requires an interactive terminal; pass --yes to continue",
        ));
    }

    print!("{} [y/N] ", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    let bytes_read = io::stdin().read_line(&mut input)?;
    if bytes_read == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "confirmation input closed",
        ));
    }

    Ok(input.trim().eq_ignore_ascii_case("y"))
}
