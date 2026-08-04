use chrono::Local;
use globset::{Glob, GlobMatcher};
use ignore::WalkBuilder;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::ErrorKind,
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
    include_capture_groups: bool,
    glob: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SweepResult {
    output_path: String,
    matches_written: usize,
    files_scanned: usize,
    files_skipped: usize,
    affected_files: usize,
    errors_count: usize,
    export_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchRecord {
    path: String,
    pattern: String,
    #[serde(rename = "match")]
    matched_text: String,
    line: usize,
    start_column: usize,
    end_column: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    capture_groups: Vec<CaptureGroup>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureGroup {
    index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    value: String,
    start_column: usize,
    end_column: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileError {
    path: String,
    error: String,
}

struct SweepData {
    folder_scanned: String,
    scan_date_time: String,
    patterns: Vec<String>,
    matches: Vec<MatchRecord>,
    files_scanned: usize,
    errors: Vec<FileError>,
    include_capture_groups: bool,
}

struct CompiledPattern {
    source: String,
    regex: Regex,
    capture_names: Vec<Option<String>>,
}

struct FileGlob {
    matcher: GlobMatcher,
    exclude: bool,
}

#[tauri::command]
fn sweep_to_json(request: SweepRequest) -> Result<SweepResult, String> {
    let output_path = validate_output_path(&request.output_path)?;
    let data = run_sweep(&request)?;

    let json = serde_json::to_string_pretty(&data.matches)
        .map_err(|error| format!("Could not serialize matches: {error}"))?;
    fs::write(&output_path, format!("{json}\n"))
        .map_err(|error| format!("Could not write JSON file: {error}"))?;

    Ok(sweep_result(output_path, &data, "JSON"))
}

#[tauri::command]
fn sweep_to_report(request: SweepRequest) -> Result<SweepResult, String> {
    let output_path = validate_output_path(&request.output_path)?;
    let data = run_sweep(&request)?;
    let html = build_html_report(&data)?;

    fs::write(&output_path, html).map_err(|error| format!("Could not write report: {error}"))?;

    Ok(sweep_result(output_path, &data, "HTML report"))
}

#[tauri::command]
fn import_text_file(path: String) -> Result<String, String> {
    let path = PathBuf::from(path.trim());
    if !path.is_file() {
        return Err("Choose a text file that exists.".into());
    }

    if is_unsupported_binary_file(&path) {
        return Err("Choose a text file to import.".into());
    }

    fs::read_to_string(&path).map_err(|error| {
        if error.kind() == ErrorKind::InvalidData {
            "Choose a UTF-8 text file to import.".to_string()
        } else {
            format!("Could not read text file: {error}")
        }
    })
}

fn validate_output_path(output_path: &str) -> Result<PathBuf, String> {
    let output_path = PathBuf::from(output_path.trim());
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err("Choose an output location inside an existing folder.".into());
        }
    }

    Ok(output_path)
}

fn run_sweep(request: &SweepRequest) -> Result<SweepData, String> {
    let root = PathBuf::from(request.folder.trim());
    if !root.is_dir() {
        return Err("Choose a folder that exists before exporting.".into());
    }

    let output_path = comparable_path(Path::new(request.output_path.trim()));
    let patterns = compile_patterns(&request.patterns, request.ignore_case)?;
    if patterns.is_empty() {
        return Err("Add at least one regex pattern before exporting.".into());
    }

    let file_glob = compile_glob(request.glob.as_deref())?;
    let mut files_scanned = 0;
    let mut records = Vec::new();
    let mut errors = Vec::new();

    let mut walker = WalkBuilder::new(&root);
    walker
        .hidden(!request.include_hidden)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true);

    for entry in walker.build() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(FileError {
                    path: "Unknown path".to_string(),
                    error: error.to_string(),
                });
                continue;
            }
        };

        let path = entry.path();
        if comparable_path(path) == output_path {
            continue;
        }

        if !path.is_file() || !matches_glob(path, &root, file_glob.as_ref()) {
            continue;
        }

        if is_unsupported_binary_file(path) {
            continue;
        }

        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::InvalidData => {
                continue;
            }
            Err(error) => {
                errors.push(FileError {
                    path: display_path(path, &root),
                    error: error.to_string(),
                });
                continue;
            }
        };

        files_scanned += 1;
        collect_matches(
            path,
            &root,
            &content,
            &patterns,
            request.include_capture_groups,
            &mut records,
        );
    }

    Ok(SweepData {
        folder_scanned: root.display().to_string(),
        scan_date_time: Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string(),
        patterns: patterns
            .iter()
            .map(|pattern| pattern.source.clone())
            .collect(),
        matches: records,
        files_scanned,
        errors,
        include_capture_groups: request.include_capture_groups,
    })
}

fn sweep_result(output_path: PathBuf, data: &SweepData, export_type: &str) -> SweepResult {
    SweepResult {
        output_path: output_path.display().to_string(),
        matches_written: data.matches.len(),
        files_scanned: data.files_scanned,
        files_skipped: data.errors.len(),
        affected_files: affected_files(data).len(),
        errors_count: data.errors.len(),
        export_type: export_type.to_string(),
    }
}

fn compile_patterns(
    patterns: &[String],
    ignore_case: bool,
) -> Result<Vec<CompiledPattern>, String> {
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
                capture_names: regex
                    .capture_names()
                    .map(|name| name.map(str::to_string))
                    .collect(),
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
    include_capture_groups: bool,
    records: &mut Vec<MatchRecord>,
) {
    let display_path = display_path(path, root);

    for (line_index, line) in content.lines().enumerate() {
        for pattern in patterns {
            if include_capture_groups {
                for captures in pattern.regex.captures_iter(line) {
                    let Some(matched) = captures.get(0) else {
                        continue;
                    };

                    let capture_groups = captures
                        .iter()
                        .enumerate()
                        .skip(1)
                        .filter_map(|(group_index, capture)| {
                            capture.map(|capture| CaptureGroup {
                                index: group_index,
                                name: pattern
                                    .capture_names
                                    .get(group_index)
                                    .and_then(|name| name.clone()),
                                value: capture.as_str().to_string(),
                                start_column: capture.start() + 1,
                                end_column: capture.end() + 1,
                            })
                        })
                        .collect();

                    records.push(MatchRecord {
                        path: display_path.clone(),
                        pattern: pattern.source.clone(),
                        matched_text: matched.as_str().to_string(),
                        line: line_index + 1,
                        start_column: matched.start() + 1,
                        end_column: matched.end() + 1,
                        capture_groups,
                    });
                }

                continue;
            }

            for matched in pattern.regex.find_iter(line) {
                records.push(MatchRecord {
                    path: display_path.clone(),
                    pattern: pattern.source.clone(),
                    matched_text: matched.as_str().to_string(),
                    line: line_index + 1,
                    start_column: matched.start() + 1,
                    end_column: matched.end() + 1,
                    capture_groups: Vec::new(),
                });
            }
        }
    }
}

fn display_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn comparable_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn is_unsupported_binary_file(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "pdf"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "bmp"
            | "ico"
            | "tif"
            | "tiff"
            | "heic"
            | "heif"
            | "avif"
            | "raw"
            | "svgz"
    )
}

fn affected_files(data: &SweepData) -> BTreeSet<String> {
    data.matches
        .iter()
        .map(|record| record.path.clone())
        .collect::<BTreeSet<_>>()
}

fn file_type_counts(data: &SweepData) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for path in affected_files(data) {
        let extension = Path::new(&path)
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| format!(".{}", value.to_lowercase()))
            .unwrap_or_else(|| "No extension".to_string());
        *counts.entry(extension).or_insert(0) += 1;
    }
    counts
}

fn build_html_report(data: &SweepData) -> Result<String, String> {
    let affected_files = affected_files(data);
    let file_types = file_type_counts(data);
    let records_json = serde_json::to_string(&data.matches)
        .map_err(|error| format!("Could not serialize report rows: {error}"))?;
    let file_types_json = serde_json::to_string(&file_types)
        .map_err(|error| format!("Could not serialize file types: {error}"))?;
    let patterns_json = serde_json::to_string(&data.patterns)
        .map_err(|error| format!("Could not serialize patterns: {error}"))?;
    let file_type_chips = file_types
        .iter()
        .map(|(file_type, count)| format!("{file_type} ({count})"))
        .collect::<Vec<_>>();

    Ok(format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Regex Sweep Report</title>
  <style>
    :root {{ color: #17211c; background: #f3f5f2; font-family: "Segoe UI", Arial, sans-serif; }}
    * {{ box-sizing: border-box; }}
    body {{ margin: 0; min-width: 320px; background: radial-gradient(circle at 50% -20%, #fff 0, #f4f6f3 43%, #edf0ec 100%); }}
    header {{ position: sticky; top: 0; z-index: 3; border-bottom: 1px solid #dfe4df; background: rgba(255,255,255,.88); backdrop-filter: blur(12px); }}
    .bar {{ max-width: 1240px; margin: 0 auto; padding: 18px 24px; display: flex; align-items: center; justify-content: space-between; gap: 16px; }}
    .brand {{ display: flex; align-items: center; gap: 12px; font-weight: 800; font-size: 18px; letter-spacing: -.03em; }}
    .logo {{ width: 36px; height: 36px; border-radius: 12px; background: #173c28; color: #95f5b8; display: grid; place-items: center; font-size: 24px; line-height: 1; box-shadow: 0 1px 3px rgba(0,0,0,.12); }}
    .tag {{ border: 1px solid #dce3dd; background: #f6f8f6; color: #617067; border-radius: 999px; padding: 3px 8px; font-size: 10px; font-weight: 800; text-transform: uppercase; letter-spacing: .12em; }}
    main {{ max-width: 1240px; margin: 0 auto; padding: 42px 24px 64px; }}
    .eyebrow {{ display: inline-flex; align-items: center; gap: 8px; background: #def8e7; color: #147841; border-radius: 999px; padding: 6px 12px; font-size: 12px; font-weight: 800; }}
    h1 {{ margin: 18px 0 10px; color: #142019; font-size: clamp(34px, 5vw, 54px); line-height: 1; letter-spacing: -.045em; }}
    .lede {{ margin: 0 0 28px; max-width: 760px; color: #667269; font-size: 17px; line-height: 1.6; }}
    .panel {{ overflow: hidden; border: 1px solid #dce2dd; border-radius: 16px; background: white; box-shadow: 0 1px 1px rgba(22,34,27,.04), 0 16px 40px rgba(22,34,27,.06); margin-top: 22px; }}
    .summary {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); border-top: 1px solid #e1e6e2; }}
    .metric {{ padding: 20px; border-right: 1px solid #e8ece8; border-bottom: 1px solid #e8ece8; }}
    .metric span {{ display: block; color: #809086; font-size: 11px; font-weight: 800; text-transform: uppercase; letter-spacing: .12em; }}
    .metric strong {{ display: block; margin-top: 8px; color: #173c28; font-size: 30px; line-height: 1; }}
    .details {{ padding: 20px; display: grid; gap: 14px; }}
    .details div {{ display: grid; gap: 4px; }}
    .details span, .controls span {{ color: #809086; font-size: 11px; font-weight: 800; text-transform: uppercase; letter-spacing: .12em; }}
    .details code {{ color: #263229; background: #f0f3f0; border-radius: 6px; padding: 3px 6px; word-break: break-all; }}
    .chips {{ display: flex; flex-wrap: wrap; gap: 8px; }}
    .chip {{ border: 1px solid #dfe4df; border-radius: 999px; background: #fafbfa; color: #263229; padding: 6px 10px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; }}
    .controls {{ display: grid; grid-template-columns: 1.5fr repeat(3, minmax(150px, .5fr)); gap: 12px; padding: 18px; border-bottom: 1px solid #e4e8e4; background: #fbfcfb; }}
    .table-status {{ padding: 10px 18px; border-bottom: 1px solid #e4e8e4; color: #667269; font-size: 13px; }}
    label {{ display: grid; gap: 6px; }}
    input, select {{ width: 100%; min-height: 42px; border: 1px solid #d9dfda; border-radius: 10px; background: white; padding: 0 12px; color: #17211c; font: inherit; }}
    table {{ width: 100%; border-collapse: collapse; font-size: 14px; }}
    th, td {{ padding: 12px 14px; border-bottom: 1px solid #e8ece8; text-align: left; vertical-align: top; }}
    th {{ position: sticky; top: 73px; z-index: 2; background: #f6f8f6; color: #5e6c63; font-size: 11px; text-transform: uppercase; letter-spacing: .12em; }}
    td code {{ font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; color: #263229; }}
    .path {{ max-width: 360px; word-break: break-all; }}
    .match {{ max-width: 380px; word-break: break-word; }}
    .groups {{ min-width: 220px; max-width: 340px; }}
    .group-list {{ display: flex; flex-wrap: wrap; gap: 6px; }}
    .group-chip {{ display: inline-flex; gap: 5px; border: 1px solid #dfe4df; border-radius: 6px; background: #fafbfa; padding: 4px 6px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; }}
    .group-chip b {{ color: #667269; }}
    .empty {{ padding: 28px; color: #7b867e; }}
    @media (max-width: 820px) {{
      .controls {{ grid-template-columns: 1fr; }}
      .bar {{ align-items: flex-start; flex-direction: column; }}
      th {{ position: static; }}
    }}
  </style>
</head>
<body>
  <header>
    <div class="bar">
      <div class="brand"><div class="logo">{{}}</div><span>Regex Sweep</span><span class="tag">Report</span></div>
      <div class="tag">Self-contained HTML</div>
    </div>
  </header>
  <main>
    <span class="eyebrow">Sweep results</span>
    <h1>Sweep files and create a report</h1>
    <p class="lede">A standalone report generated by Regex Sweep with summary metrics and an accessible filterable table of every match.</p>

    <section class="panel">
      <div class="details">
        <div><span>Folder scanned</span><code>{folder}</code></div>
        <div><span>Scan date and time</span><code>{scan_date}</code></div>
        <div><span>Regex or search term</span><div class="chips" id="patterns">{patterns_html}</div></div>
        <div><span>File types affected</span><div class="chips" id="fileTypes">{file_types_html}</div></div>
      </div>
      <div class="summary">
        <div class="metric"><span>Files scanned</span><strong>{files_scanned}</strong></div>
        <div class="metric"><span>Affected files</span><strong>{affected_files}</strong></div>
        <div class="metric"><span>Total matches</span><strong>{total_matches}</strong></div>
      </div>
    </section>

    <section class="panel">
      <div class="controls">
        <label><span>Search table</span><input id="q" placeholder="Filter path, regex, match text, groups..." autocomplete="off" autocapitalize="none" spellcheck="false"></label>
        <label><span>Pattern</span><select id="patternFilter"><option value="">All patterns</option></select></label>
        <label><span>File type</span><select id="typeFilter"><option value="">All file types</option></select></label>
        <label><span>File path</span><select id="pathFilter"><option value="">All affected files</option></select></label>
      </div>
      <div class="table-status" id="tableStatus"></div>
      <div style="overflow:auto">
        <table>
          <thead><tr><th>Path</th><th>Pattern</th><th>Match</th>{groups_header_html}<th>Line</th><th>Columns</th></tr></thead>
          <tbody id="rows">{rows_html}</tbody>
        </table>
      </div>
      <div class="empty" id="empty" hidden>No rows match the current filters.</div>
    </section>

  </main>

  <script type="application/json" id="rows-data">{records_json}</script>
  <script type="application/json" id="file-types-data">{file_types_json}</script>
  <script type="application/json" id="patterns-data">{patterns_json}</script>
  <script>
    const byId = id => document.getElementById(id);
    const rows = JSON.parse(byId('rows-data').textContent);
    const includeCaptureGroups = {include_capture_groups};
    const fileTypes = JSON.parse(byId('file-types-data').textContent);
    const patterns = JSON.parse(byId('patterns-data').textContent);
    const extOf = path => {{
      const normalized = String(path).split(String.fromCharCode(92)).join('/');
      const name = normalized.split('/').pop() || normalized;
      const index = name.lastIndexOf('.');
      return index > 0 ? name.slice(index).toLowerCase() : 'No extension';
    }};
    const escapeHtml = value => String(value).replace(/[&<>"']/g, char => {{
      switch (char) {{
        case '&': return '&amp;';
        case '<': return '&lt;';
        case '>': return '&gt;';
        case '"': return '&quot;';
        case "'": return '&#39;';
        default: return char;
      }}
    }});
    const unique = values => [...new Set(values)].sort((a, b) => a.localeCompare(b));
    const addOptions = (select, values) => values.forEach(value => {{
      const option = document.createElement('option');
      option.value = value;
      option.textContent = value;
      select.appendChild(option);
    }});
    const addChips = (container, values) => {{
      container.innerHTML = values.length ? values.map(value => `<span class=\"chip\">${{escapeHtml(value)}}</span>`).join('') : '<span class=\"chip\">None</span>';
    }};
    const groupLabel = group => group.name || `$${{group.index}}`;
    const groupText = row => (row.captureGroups || []).map(group => `${{groupLabel(group)}}: ${{group.value}}`).join(' ');
    const groupsHtml = row => {{
      const groups = row.captureGroups || [];
      if (!includeCaptureGroups) return '';
      if (!groups.length) return '<td class=\"groups\"><span class=\"empty\">None</span></td>';
      return `<td class=\"groups\"><div class=\"group-list\">${{groups.map(group => `<span class=\"group-chip\"><b>${{escapeHtml(groupLabel(group))}}</b><span>${{escapeHtml(group.value)}}</span></span>`).join('')}}</div></td>`;
    }};
    addChips(byId('patterns'), patterns);
    addChips(byId('fileTypes'), Object.entries(fileTypes).map(([type, count]) => `${{type}} (${{count}})`));
    addOptions(byId('patternFilter'), unique(rows.map(row => row.pattern)));
    addOptions(byId('typeFilter'), unique(rows.map(row => extOf(row.path))));
    addOptions(byId('pathFilter'), unique(rows.map(row => row.path)));

    function render() {{
      const q = byId('q').value.trim().toLowerCase();
      const pattern = byId('patternFilter').value;
      const type = byId('typeFilter').value;
      const path = byId('pathFilter').value;
      const filtered = rows.filter(row => {{
        const haystack = [row.path, row.pattern, row.match, groupText(row), row.line, row.startColumn, row.endColumn].join(' ').toLowerCase();
        return (!q || haystack.includes(q)) &&
          (!pattern || row.pattern === pattern) &&
          (!type || extOf(row.path) === type) &&
          (!path || row.path === path);
      }});
      byId('rows').innerHTML = filtered.map(row => `<tr>
        <td class=\"path\"><code>${{escapeHtml(row.path)}}</code></td>
        <td><code>${{escapeHtml(row.pattern)}}</code></td>
        <td class=\"match\">${{escapeHtml(row.match)}}</td>
        ${{groupsHtml(row)}}
        <td>${{row.line}}</td>
        <td>${{row.startColumn}}-${{row.endColumn}}</td>
      </tr>`).join('');
      byId('tableStatus').textContent = `Showing ${{filtered.length}} of ${{rows.length}} matches`;
      byId('empty').hidden = filtered.length !== 0;
    }}
    ['q', 'patternFilter', 'typeFilter', 'pathFilter'].forEach(id => {{
      byId(id).addEventListener('input', render);
      byId(id).addEventListener('change', render);
    }});
    render();
  </script>
</body>
</html>"#,
        folder = html_escape(&data.folder_scanned),
        scan_date = html_escape(&data.scan_date_time),
        files_scanned = data.files_scanned,
        affected_files = affected_files.len(),
        total_matches = data.matches.len(),
        patterns_html = chips_html(&data.patterns),
        file_types_html = chips_html(&file_type_chips),
        groups_header_html = if data.include_capture_groups {
            "<th>Groups</th>"
        } else {
            ""
        },
        rows_html = rows_html(&data.matches, data.include_capture_groups),
        records_json = json_for_script(&records_json),
        file_types_json = json_for_script(&file_types_json),
        patterns_json = json_for_script(&patterns_json),
        include_capture_groups = data.include_capture_groups,
    ))
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn json_for_script(value: &str) -> String {
    value
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

fn chips_html(values: &[String]) -> String {
    if values.is_empty() {
        return "<span class=\"chip\">None</span>".to_string();
    }

    values
        .iter()
        .map(|value| format!("<span class=\"chip\">{}</span>", html_escape(value)))
        .collect::<Vec<_>>()
        .join("")
}

fn rows_html(matches: &[MatchRecord], include_capture_groups: bool) -> String {
    matches
        .iter()
        .map(|record| {
            format!(
                "<tr><td class=\"path\"><code>{}</code></td><td><code>{}</code></td><td class=\"match\">{}</td>{}<td>{}</td><td>{}-{}</td></tr>",
                html_escape(&record.path),
                html_escape(&record.pattern),
                html_escape(&record.matched_text),
                groups_html(record, include_capture_groups),
                record.line,
                record.start_column,
                record.end_column
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn groups_html(record: &MatchRecord, include_capture_groups: bool) -> String {
    if !include_capture_groups {
        return String::new();
    }

    if record.capture_groups.is_empty() {
        return "<td class=\"groups\"><span class=\"empty\">None</span></td>".to_string();
    }

    let groups = record
        .capture_groups
        .iter()
        .map(|group| {
            let label = group
                .name
                .as_deref()
                .map(str::to_string)
                .unwrap_or_else(|| format!("${}", group.index));
            format!(
                "<span class=\"group-chip\"><b>{}</b><span>{}</span></span>",
                html_escape(&label),
                html_escape(&group.value)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!("<td class=\"groups\"><div class=\"group-list\">{groups}</div></td>")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            sweep_to_json,
            sweep_to_report,
            import_text_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running Regex Sweep");
}

#[cfg(test)]
mod tests {
    use super::{collect_matches, compile_patterns, is_unsupported_binary_file};
    use std::path::Path;

    #[test]
    fn unsupported_binary_files_include_pdfs_and_images() {
        for path in [
            "example.pdf",
            "photo.PNG",
            "scan.jpeg",
            "icon.ico",
            "image.tiff",
            "compressed.svgz",
        ] {
            assert!(is_unsupported_binary_file(Path::new(path)));
        }
    }

    #[test]
    fn text_like_files_are_not_treated_as_unsupported_binary_files() {
        for path in [
            "notes.txt",
            "src/main.rs",
            "README.md",
            "data.json",
            "vector.svg",
        ] {
            assert!(!is_unsupported_binary_file(Path::new(path)));
        }
    }

    #[test]
    fn capture_groups_are_included_when_enabled() {
        let patterns = compile_patterns(
            &[r"First_Name: (?<first>\w+), Last_Name: (?<last>\w+)".to_string()],
            false,
        )
        .expect("pattern should compile");
        let mut records = Vec::new();

        collect_matches(
            Path::new("people.txt"),
            Path::new(""),
            "First_Name: Jane, Last_Name: Smith",
            &patterns,
            true,
            &mut records,
        );

        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].matched_text,
            "First_Name: Jane, Last_Name: Smith"
        );
        assert_eq!(records[0].capture_groups.len(), 2);
        assert_eq!(records[0].capture_groups[0].name.as_deref(), Some("first"));
        assert_eq!(records[0].capture_groups[0].value, "Jane");
        assert_eq!(records[0].capture_groups[1].name.as_deref(), Some("last"));
        assert_eq!(records[0].capture_groups[1].value, "Smith");
    }
}
