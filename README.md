# Regex Sweep

Regex Sweep is a local-first desktop app for scanning a folder with one or more regular expressions and creating a report. The main output is a self-contained, filterable HTML report, and JSON is also available as an export option.

It is built with Tauri, React, and Rust. The app does the file search and JSON writing itself, so people using the packaged app do not need `ripgrep`, `jq`, Bash, Node.js, Rust, or Cargo installed.

## How it works

1. Add one or more regex patterns.
2. Use the optional test string panel to preview matches before scanning files.
3. Choose a folder to search.
4. Adjust advanced options if needed.
5. Click **Web report** to create the main HTML report, or **Export JSON** if you need raw structured data.

Regex Sweep sends the selected folder, patterns, and options to the bundled Rust backend. The backend walks the folder, applies the regex patterns to readable text files, collects match metadata, and writes the selected export at the location you choose.

Need help writing a pattern? [RegExr](https://regexr.com/) is a useful regex builder and tester.

## JSON output

The exported file is a JSON array. Each object includes the matched file, the regex pattern that matched, the matched text, and its location in the file.

```json
[
  {
    "path": "src/config.ts",
    "pattern": "password\\s*[=:]\\s*[^\\s]+",
    "match": "password = secret",
    "line": 12,
    "startColumn": 1,
    "endColumn": 18
  }
]
```

## Web report output

The **Web report** option creates a standalone `.html` file that can be opened in a browser and shared without any server. It includes:

- Folder scanned
- Scan date and time
- Regex or search term
- Number of files scanned
- Number of affected files
- Total matches
- File types affected
- Errors or inaccessible files
- A filterable results table

The table can be filtered by free text, pattern, file type, and affected file path.

## Advanced options

- **Include hidden files** searches dotfiles and hidden folders while still respecting ignore rules.
- **Ignore letter case** applies case-insensitive matching to both preview and export.
- **File glob** limits the files searched, such as `*.ts` or `*.json`.
- Prefix a glob with `!` to exclude files, such as `!node_modules/**`.

## Using the app

Download or build the desktop bundle for your operating system, then open Regex Sweep like a normal app.

On macOS, a production build creates:

```text
src-tauri/target/release/bundle/macos/Regex Sweep.app
src-tauri/target/release/bundle/dmg/Regex Sweep_0.1.0_aarch64.dmg
```

On Windows, a production build can create a Microsoft Store-ready MSIX package from a Windows build machine or GitHub Actions runner:

```text
dist-msix/RegexSweepDesktop_0.1.0.0_x64.msix
```

The packaged app is self-contained. Runtime terminal tools are not required.

## Windows deployment

Yes, Regex Sweep can be deployed on Windows. The release workflow builds a Windows MSIX with Microsoft's WinApp CLI on `windows-latest`, then uploads it as a GitHub Actions artifact.

For Microsoft Store submission, use the MSIX artifact instead of the MSI/EXE installer URL path. Microsoft signs MSIX packages during Store processing, so no paid code-signing certificate is needed for that Store route.

To build an MSIX locally, use Windows 11 with Node.js, Rust, and the WinApp CLI installed:

```powershell
winget install Microsoft.winappcli --source winget
npm run pack:msix
```

For local sideload testing only, you can ask the packaging script to generate a development certificate:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/pack-msix.ps1 -GenerateDevCert -InstallDevCert
```

## Development

Install JavaScript dependencies:

```bash
npm install
```

Run the Tauri desktop app in development mode:

```bash
npm run dev
```

Build the production desktop app:

```bash
npm run build
```

Build only the web frontend:

```bash
npm run build:web
```

## Developer requirements

These are only required to develop or build Regex Sweep from source:

- Node.js 20.19 or newer
- Rust and Cargo
- Tauri platform prerequisites for your operating system

## Technology

- Tauri
- Rust
- React
- TypeScript
- Vite
- Tailwind CSS
- daisyUI
