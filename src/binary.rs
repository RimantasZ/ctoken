use std::path::Path;

const BINARY_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "bmp", "tiff", "svg", "pdf", "zip", "tar", "gz",
    "bz2", "xz", "7z", "rar", "exe", "dll", "so", "dylib", "o", "a", "class", "jar", "wasm", "mp3",
    "mp4", "mov", "avi", "mkv", "flv", "wmv", "ogg", "wav", "flac", "ttf", "otf", "woff", "woff2",
    "eot", "pyc", "pyd", "pyo", "db", "sqlite", "sqlite3",
];

pub fn is_binary_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| BINARY_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn is_binary_content(bytes: &[u8]) -> bool {
    let sample = if bytes.len() > 8192 {
        &bytes[..8192]
    } else {
        bytes
    };
    sample.contains(&0u8)
}

#[allow(dead_code)]
pub fn is_binary(path: &Path, sample: &[u8]) -> bool {
    is_binary_ext(path) || is_binary_content(sample)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn binary_ext_known() {
        assert!(is_binary_ext(Path::new("image.png")));
        assert!(is_binary_ext(Path::new("lib.so")));
        assert!(is_binary_ext(Path::new("Archive.ZIP"))); // case-insensitive
        assert!(!is_binary_ext(Path::new("main.rs")));
        assert!(!is_binary_ext(Path::new("noext")));
    }

    #[test]
    fn binary_content_nul() {
        assert!(is_binary_content(b"hello\x00world"));
        assert!(!is_binary_content(b"hello world"));
    }

    #[test]
    fn binary_content_nul_beyond_8k() {
        // NUL after 8 KB should not trigger binary detection
        let mut data = vec![b'a'; 8193];
        data[8192] = 0u8;
        assert!(!is_binary_content(&data));
    }

    #[test]
    fn png_header_bytes_no_nul() {
        // PNG magic bytes don't contain NUL — detected as binary by extension only
        let png_header: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        assert!(!is_binary_content(png_header));
        assert!(is_binary_ext(Path::new("image.png")));
    }
}
