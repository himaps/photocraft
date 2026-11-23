//! The activation tool: with no arguments, print this machine's machine code (the words its
//! user sends to the author); with machine-code words, print the activation code answering
//! them (the author's side, see `photocraft-activation`).

use crate::args::ActivationArgs;
use crate::error::{CliError, Result};
use std::io::Write;

pub(crate) fn activation(a: &ActivationArgs, out: &mut dyn Write) -> Result<()> {
    if a.words.is_empty() {
        let id = photocraft_activation::machine_id().map_err(|e| CliError::Other(e.to_string()))?;
        let code = photocraft_activation::machine_code(&id);
        let _ = writeln!(out, "{}", code.render());
        return Ok(());
    }
    // The words may arrive space-separated on a command line; rejoin with the separator before
    // parsing (parse trims whitespace around each `-` token).
    let machine = photocraft_activation::parse(&a.words.join(&photocraft_activation::SEPARATOR.to_string()))
        .map_err(|e| CliError::Other(e.to_string()))?;
    let response = photocraft_activation::activation_response(&machine);
    let _ = writeln!(out, "{}", response.render());
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::test_support::invoke;

    /// The author's activation tool round-trips: the code printed for a machine code verifies
    /// against it in the activation crate, and a bad word is an error, not a panic (rule 9).
    #[test]
    fn activation_answers_the_machine_code_words() {
        let machine = photocraft_activation::machine_code("cli-test-machine");
        let words = machine.render();
        let out = invoke(&["activation", &words]);
        assert_eq!(out.code, 0);
        let answer = out.stdout.trim().to_owned();
        assert_eq!(
            photocraft_activation::parse(&answer).unwrap(),
            photocraft_activation::activation_response(&machine)
        );
        // Space-separated words are accepted too (how a command line would pass them).
        let spaced = words.split(photocraft_activation::SEPARATOR).collect::<Vec<_>>();
        let out = invoke(&["activation"].iter().copied().chain(spaced.iter().copied()).collect::<Vec<_>>());
        assert_eq!(out.code, 0);
        assert_eq!(out.stdout.trim(), answer);
        // A word outside the table is a usage error.
        let out = invoke(&["activation", "hello"]);
        assert_ne!(out.code, 0);
        assert!(!out.stderr.is_empty());
    }
}
