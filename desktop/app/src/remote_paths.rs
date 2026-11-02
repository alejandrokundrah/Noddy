// SPDX-License-Identifier: GPL-3.0-or-later
//! The plain parts of browsing another computer's folders: paths inside what it
//! shares (relative, `/`-separated, `""` is the top), how a folder's entries are
//! ordered and shown, and file names fit to save here.

use noddy_core::StorageEntry;

/// `name` inside the folder `base`.
pub fn join(base: &str, name: &str) -> String {
    if base.is_empty() { name.to_owned() } else { format!("{base}/{name}") }
}

/// The folder holding `path`, or `None` at the top.
pub fn parent(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    Some(path.rsplit_once('/').map_or_else(String::new, |(up, _)| up.to_owned()))
}

/// The last part of `path`.
pub fn leaf(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Folders first, then files, each by name ignoring case.
pub fn sort_entries(entries: &mut [StorageEntry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// The entries for QML: `[{name, path, isDir, size, modified}]`.
pub fn entries_json(base: &str, entries: &[StorageEntry]) -> String {
    let rows: Vec<_> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "name": e.name,
                "path": join(base, &e.name),
                "isDir": e.is_dir,
                "size": e.size,
                "modified": e.modified,
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Where `path` is, as clickable steps from the top: `[{name, path}]`, the
/// first being the top (`top_name`).
pub fn crumbs_json(top_name: &str, path: &str) -> String {
    let mut steps = vec![serde_json::json!({ "name": top_name, "path": "" })];
    let mut so_far = String::new();
    for part in path.split('/').filter(|p| !p.is_empty()) {
        so_far = join(&so_far, part);
        steps.push(serde_json::json!({ "name": part, "path": so_far }));
    }
    serde_json::Value::Array(steps).to_string()
}

/// A name that is safe to create here: no folders, no characters a file
/// system refuses, never empty or all dots.
pub fn safe_file_name(name: &str) -> String {
    let cleaned: String =
        name.chars().map(|c| if c.is_control() || "\\/:*?\"<>|".contains(c) { '_' } else { c }).collect();
    let cleaned = cleaned.trim().trim_end_matches('.').trim_start_matches('.').trim().to_owned();
    if cleaned.is_empty() { "file".to_owned() } else { cleaned }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool) -> StorageEntry {
        StorageEntry { name: name.into(), size: 3, modified: 0, is_dir }
    }

    #[test]
    fn paths_join_and_go_up() {
        assert_eq!(join("", "Docs"), "Docs");
        assert_eq!(join("Docs/2026", "a.txt"), "Docs/2026/a.txt");
        assert_eq!(parent(""), None);
        assert_eq!(parent("Docs").as_deref(), Some(""));
        assert_eq!(parent("Docs/2026").as_deref(), Some("Docs"));
        assert_eq!(leaf("Docs/2026/a.txt"), "a.txt");
        assert_eq!(leaf("a.txt"), "a.txt");
    }

    #[test]
    fn folders_come_first_then_files_by_name_ignoring_case() {
        let mut list =
            vec![entry("b.txt", false), entry("Zed", true), entry("A.txt", false), entry("alpha", true)];
        sort_entries(&mut list);
        let names: Vec<_> = list.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["alpha", "Zed", "A.txt", "b.txt"]);
    }

    #[test]
    fn entries_carry_their_full_path() {
        let json: serde_json::Value =
            serde_json::from_str(&entries_json("Docs", &[entry("a.txt", false)])).unwrap();
        assert_eq!(json[0]["path"], "Docs/a.txt");
        assert_eq!(json[0]["isDir"], false);
        assert_eq!(json[0]["size"], 3);
    }

    #[test]
    fn breadcrumbs_lead_from_the_top() {
        let json: serde_json::Value = serde_json::from_str(&crumbs_json("Laptop", "Docs/2026")).unwrap();
        let steps: Vec<_> = json
            .as_array()
            .unwrap()
            .iter()
            .map(|s| (s["name"].as_str().unwrap(), s["path"].as_str().unwrap()))
            .collect();
        assert_eq!(steps, [("Laptop", ""), ("Docs", "Docs"), ("2026", "Docs/2026")]);
        let top: serde_json::Value = serde_json::from_str(&crumbs_json("Laptop", "")).unwrap();
        assert_eq!(top.as_array().unwrap().len(), 1);
    }

    #[test]
    fn saved_names_cannot_escape_or_be_empty() {
        assert_eq!(safe_file_name("report.pdf"), "report.pdf");
        assert_eq!(safe_file_name("..\\..\\evil.exe"), "_.._evil.exe");
        assert_eq!(safe_file_name("a/b:c?.txt"), "a_b_c_.txt");
        assert_eq!(safe_file_name("..."), "file");
        assert_eq!(safe_file_name("   "), "file");
        assert_eq!(safe_file_name(".hidden"), "hidden");
    }
}
