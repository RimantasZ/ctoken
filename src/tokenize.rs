use tiktoken_rs::{cl100k_base, o200k_base, p50k_base, p50k_edit, r50k_base, CoreBPE};

use crate::cli::EncodingArg;
use crate::error::Result;

pub struct Tokenizer {
    bpe: CoreBPE,
}

impl Tokenizer {
    pub fn new(encoding: &EncodingArg) -> Result<Self> {
        let bpe = match encoding {
            EncodingArg::Cl100kBase => cl100k_base(),
            EncodingArg::O200kBase => o200k_base(),
            EncodingArg::P50kBase => p50k_base(),
            EncodingArg::P50kEdit => p50k_edit(),
            EncodingArg::R50kBase => r50k_base(),
        }
        .map_err(|e| anyhow::anyhow!("failed to load encoding {}: {}", encoding, e))?;
        Ok(Self { bpe })
    }

    pub fn count(&self, text: &str) -> usize {
        self.bpe.encode_with_special_tokens(text).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::EncodingArg;

    fn tok(enc: EncodingArg) -> Tokenizer {
        Tokenizer::new(&enc).unwrap()
    }

    #[test]
    fn cl100k_known_counts() {
        let t = tok(EncodingArg::Cl100kBase);
        assert_eq!(t.count("hello world"), 2);
        assert_eq!(t.count(""), 0);
        assert_eq!(t.count("Hello, World!"), 4);
    }

    #[test]
    fn o200k_known_counts() {
        let t = tok(EncodingArg::O200kBase);
        assert_eq!(t.count("hello world"), 2);
        assert_eq!(t.count(""), 0);
    }

    #[test]
    fn p50k_known_counts() {
        let t = tok(EncodingArg::P50kBase);
        // "hello world" tokenizes differently in p50k
        let n = t.count("hello world");
        assert!(n >= 2);
    }
}
