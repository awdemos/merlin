//! Local sensitivity detector: deterministic fallback when the laya
//! sidecar is unavailable. Three-state verdict — `Sensitive`, `Clean`,
//! `Undetermined` (only when the scan itself could not run, i.e. the
//! prompt exceeds the scan size cap).

use regex::Regex;

/// Result of a local sensitivity scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanVerdict {
    Sensitive,
    Clean,
    Undetermined,
}

/// Prompts longer than this are not scanned and reported as
/// `Undetermined` (the gate then applies the strictest policy).
const MAX_SCAN_CHARS: usize = 1_000_000;

/// Minimum Shannon entropy (bits per char) for a whitespace-delimited
/// token to count as a probable secret.
const ENTROPY_THRESHOLD: f64 = 4.0;

/// Minimum length of a token considered for entropy screening.
const ENTROPY_MIN_LEN: usize = 20;

/// Clone so server state can share one detector across request handlers.
/// `Regex` is cheap to clone (compiles once, shares the program).
#[derive(Clone)]
pub struct Detector {
    patterns: Vec<Regex>,
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector {
    pub fn new() -> Self {
        let patterns = [
            // US SSN
            r"\b\d{3}-\d{2}-\d{4}\b",
            // Email address
            r"\b[\w.+-]+@[\w-]+\.[\w.-]+\b",
            // US-style phone
            r"\b(?:\+?1[-. ]?)?\(?\d{3}\)?[-. ]?\d{3}[-. ]?\d{4}\b",
            // AWS access key id
            r"\b(?:AKIA|ASIA|ABIA|ACCA)[A-Z0-9]{16}\b",
            // GitHub / GitLab tokens
            r"\b(?:ghp|gho|ghu|ghs|ghr|glpat)_[A-Za-z0-9]{20,}\b",
        ]
        .iter()
        .map(|p| Regex::new(p).expect("detector pattern compiles"))
        .collect();
        Self { patterns }
    }

    pub fn scan(&self, text: &str) -> ScanVerdict {
        if text.chars().count() > MAX_SCAN_CHARS {
            return ScanVerdict::Undetermined;
        }
        if self.patterns.iter().any(|re| re.is_match(text)) {
            return ScanVerdict::Sensitive;
        }
        if has_high_entropy_token(text) {
            return ScanVerdict::Sensitive;
        }
        ScanVerdict::Clean
    }
}

/// Shannon entropy (bits per byte) of a token over its byte alphabet.
fn shannon_entropy(token: &str) -> f64 {
    let mut counts = [0usize; 256];
    let bytes = token.as_bytes();
    for &b in bytes {
        counts[b as usize] += 1;
    }
    let len = bytes.len() as f64;
    counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len;
            -p * p.log2()
        })
        .sum()
}

fn has_high_entropy_token(text: &str) -> bool {
    text.split_whitespace()
        .any(|token| token.len() >= ENTROPY_MIN_LEN && shannon_entropy(token) >= ENTROPY_THRESHOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssn_is_sensitive() {
        assert_eq!(
            Detector::new().scan("my ssn is 123-45-6789, thanks"),
            ScanVerdict::Sensitive
        );
    }

    #[test]
    fn email_is_sensitive() {
        assert_eq!(
            Detector::new().scan("contact me at jane.doe@example.com please"),
            ScanVerdict::Sensitive
        );
    }

    #[test]
    fn phone_is_sensitive() {
        assert_eq!(
            Detector::new().scan("call me at (415) 555-2671"),
            ScanVerdict::Sensitive
        );
    }

    #[test]
    fn aws_key_is_sensitive() {
        assert_eq!(
            Detector::new().scan("key: AKIAIOSFODNN7EXAMPLE and done"),
            ScanVerdict::Sensitive
        );
    }

    #[test]
    fn github_token_is_sensitive() {
        assert_eq!(
            Detector::new().scan("token ghp_16C7e42F292c6912E7710c838347Ae178B4a"),
            ScanVerdict::Sensitive
        );
    }

    #[test]
    fn high_entropy_token_is_sensitive() {
        let secret = "x9Kq2mP8vL4zR7tN3wB6yH5sD1fG0jC8aE3uI";
        assert_eq!(Detector::new().scan(secret), ScanVerdict::Sensitive);
    }

    #[test]
    fn ordinary_prompt_is_clean() {
        assert_eq!(
            Detector::new().scan("What is the capital of France? Explain briefly."),
            ScanVerdict::Clean
        );
    }

    #[test]
    fn empty_prompt_is_clean() {
        assert_eq!(Detector::new().scan(""), ScanVerdict::Clean);
    }

    #[test]
    fn oversized_prompt_is_undetermined() {
        let big = "a".repeat(MAX_SCAN_CHARS + 1);
        assert_eq!(Detector::new().scan(&big), ScanVerdict::Undetermined);
    }
}
