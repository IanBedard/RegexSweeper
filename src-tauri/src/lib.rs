use globset::{Glob, GlobMatcher};
use ignore::WalkBuilder;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SweepRequest {
    patterns: Vec<String>,
    folder: String,
    output_path: String,
    include_hidden: bool,
    ignore_case: bool,
    glob: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SweepResult {
    output_path: String,
    matches_written: usize,
    files_scanned: usize,
    files_skipped: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchRecord {
    path: String,
    pattern: String,
    #[serde(rename = "match")]
    matched_text: String,
    line: usize,
    start_column: usize,
    end_column: usize,
}

struct CompiledPattern {
    source: String,
    regex: Regex,
}

struct FileGlob {
    matcher: GlobMatcher,
    exclude: bool,
}

#[tauri::command]
fn sweep_to_json(request: SweepRequest) -> Result<SweepResult, String> {
    let root = PathBuf::from(request.folder.trim());
    if !root.is_dir() {
        return Err("Choose a folder that exists before exporting JSON.".into());
    }

    let output_path = PathBuf::from(request.output_path.trim());
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err("Choose an output location inside an existing folder.".into());
        }
    }

    let patterns = compile_patterns(&request.patterns, request.ignore_case)?;
    if patterns.is_empty() {
        return Err("Add at least one regex pattern before exporting JSON.".into());
    }

    let file_glob = compile_glob(request.glob.as_deref())?;
    let mut files_scanned = 0;
    let mut files_skipped = 0;
    let mut records = Vec::new();

    let mut walker = WalkBuilder::new(&root);
    walker
        .hidden(!request.include_hidden)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true);

    for entry in walker.build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                files_skipped += 1;
                continue;
            }
        };

        let path = entry.path();
        if !path.is_file() || !matches_glob(path, &root, file_glob.as_ref()) {
            continue;
        }

        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => {
                files_skipped += 1;
                continue;
            }
        };

        files_scanned += 1;
        collect_matches(path, &root, &content, &patterns, &mut records);
    }

    let json = serde_json::to_string_pretty(&records)
        .map_err(|error| format!("Could not serialize matches: {error}"))?;
    fs::write(&output_path, format!("{json}\n"))
        .map_err(|error| format!("Could not write JSON file: {error}"))?;

    Ok(SweepResult {
        output_path: output_path.display().to_string(),
        matches_written: records.len(),
        files_scanned,
        files_skipped,
    })
}

fn compile_patterns(patterns: &[String], ignore_case: bool) -> Result<Vec<CompiledPattern>, String> {
    patterns
        .iter()
        .map(|pattern| pattern.trim())
        .filter(|pattern| !pattern.is_empty())
        .map(|pattern| {
            let regex = RegexBuilder::new(pattern)
                .case_insensitive(ignore_case)
                .build()
                .map_err(|error| format!("Invalid regex \"{pattern}\": {error}"))?;

            Ok(CompiledPattern {
                source: pattern.to_string(),
                regex,
            })
        })
        .collect()
}

fn compile_glob(glob: Option<&str>) -> Result<Option<FileGlob>, String> {
    let glob = match glob.map(str::trim).filter(|glob| !glob.is_empty()) {
        Some(glob) => glob,
        None => return Ok(None),
    };
    let exclude = glob.starts_with('!');
    let pattern = glob.trim_start_matches('!');
    let matcher = Glob::new(pattern)
        .map_err(|error| format!("Invalid file glob \"{glob}\": {error}"))?
        .compile_matcher();

    Ok(Some(FileGlob { matcher, exclude }))
}

fn matches_glob(path: &Path, root: &Path, glob: Option<&FileGlob>) -> bool {
    let Some(glob) = glob else {
        return true;
    };

    let relative_path = path.strip_prefix(root).unwrap_or(path);
    let is_match = glob.matcher.is_match(relative_path)
        || relative_path
            .file_name()
            .map(|name| glob.matcher.is_match(Path::new(name)))
            .unwrap_or(false);

    if glob.exclude {
        !is_match
    } else {
        is_match
    }
}

fn collect_matches(
    path: &Path,
    root: &Path,
    content: &str,
    patterns: &[CompiledPattern],
    records: &mut Vec<MatchRecord>,
) {
    let display_path = path
        .strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string();

    for (line_index, line) in content.lines().enumerate() {
        for pattern in patterns {
            for matched in pattern.regex.find_iter(line) {
                records.push(MatchRecord {
                    path: display_path.clone(),
                    pattern: pattern.source.clone(),
                    matched_text: matched.as_str().to_string(),
                    line: line_index + 1,
                    start_column: matched.start() + 1,
                    end_column: matched.end() + 1,
                });
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![sweep_to_json])
        .run(tauri::generate_context!())
        .expect("error while running Regex Sweep");
}
