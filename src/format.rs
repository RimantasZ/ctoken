use std::path::Path;

use crate::tokenize_files::Outcome;

pub fn fmt_thousands(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

pub fn fmt_verbose_line(rel: &Path, outcome: &Outcome) -> String {
    match outcome {
        Outcome::Tokens(n) => {
            let formatted = fmt_thousands(*n);
            format!("{:>8} - {}", formatted, rel.display())
        }
        Outcome::Binary => {
            format!("{:>8} - {} (binary)", "", rel.display())
        }
        Outcome::Error(_) => {
            // errors are reported to stderr; verbose line shows them skipped
            format!("{:>8} - {} (error)", "", rel.display())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn thousands_zero() {
        assert_eq!(fmt_thousands(0), "0");
    }

    #[test]
    fn thousands_small() {
        assert_eq!(fmt_thousands(999), "999");
    }

    #[test]
    fn thousands_one_comma() {
        assert_eq!(fmt_thousands(1234), "1,234");
    }

    #[test]
    fn thousands_two_commas() {
        assert_eq!(fmt_thousands(1234567), "1,234,567");
    }

    #[test]
    fn thousands_exact_boundary() {
        assert_eq!(fmt_thousands(1000), "1,000");
        assert_eq!(fmt_thousands(1000000), "1,000,000");
    }

    #[test]
    fn verbose_token_padding_short() {
        let line = fmt_verbose_line(Path::new("src/main.rs"), &Outcome::Tokens(412));
        assert_eq!(line, "     412 - src/main.rs");
    }

    #[test]
    fn verbose_token_padding_with_comma() {
        let line = fmt_verbose_line(Path::new("src/lib.rs"), &Outcome::Tokens(1205));
        assert_eq!(line, "   1,205 - src/lib.rs");
    }

    #[test]
    fn verbose_binary() {
        let line = fmt_verbose_line(Path::new("assets/logo.png"), &Outcome::Binary);
        assert_eq!(line, "         - assets/logo.png (binary)");
    }

    #[test]
    fn verbose_large_token_count_exceeds_width() {
        // > 8 chars formatted: pushes dash right, accepted behavior
        let line = fmt_verbose_line(Path::new("big.rs"), &Outcome::Tokens(123456789));
        assert!(line.starts_with("123,456,789 - "));
    }
}
