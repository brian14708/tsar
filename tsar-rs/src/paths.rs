pub const BUNDLE_META_PATH: &str = ".tsar/bundle";

pub fn chunk_path(f: impl AsRef<str>) -> String {
    format!(".tsar/chunks/{}", f.as_ref())
}
