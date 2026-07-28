# Regex Sweep

Regex Sweep is a local-first React application for building a safe Bash command that searches files using one or more regular expressions and returns newline-delimited JSON.

## How it works

1. Enter one or more regular expressions. Use **Add another pattern** to search for several patterns in one sweep.
2. Paste optional sample text into **Test string**. The app evaluates the patterns in the browser and previews the matches; it does not read local files.
3. Enter the folder that Bash should search.
4. Optionally choose advanced search flags.
5. Select **Generate command**, copy the result, and run it in Bash.

The generated pipeline has two stages:

```text
ripgrep searches the files and emits structured events
                         |
                         v
jq selects match events and produces compact JSON objects
```

Each output line resembles:

```json
{"path":"src/config.ts","regex":"password = secret"}
```

The output is NDJSON: one valid JSON object per line. This format is convenient for streaming, logging, and processing large result sets.

## What “safely shell-escaped” means

Folder paths and patterns can contain spaces, quotes, dollar signs, or other characters that Bash normally interprets. Regex Sweep wraps and escapes those values so Bash passes them to `rg` as data instead of treating them as additional shell syntax.

Always review a generated command before running it. The app generates commands locally but does not execute them.

## Requirements

- Node.js 20 or newer for the web application
- [ripgrep](https://github.com/BurntSushi/ripgrep) (`rg`) for searching files
- [jq](https://jqlang.org/) for transforming matches into JSON
- Bash or a Bash-compatible shell for running the generated command

Example installations:

```bash
# macOS
brew install ripgrep jq

# Ubuntu/Debian
sudo apt install ripgrep jq

# Windows with Chocolatey (run the generated command in Git Bash or WSL)
choco install ripgrep jq
```

## Advanced options

- **Include hidden files** adds `--hidden`. Files such as `.env` may then be searched, while ripgrep's ignore rules still apply.
- **Ignore letter case** adds `--ignore-case`.
- **File glob** adds `--glob`, allowing filters such as `*.ts`, `*.json`, or `!node_modules/**`.

## Run locally

```bash
npm install
npm run dev
```

Open the local URL printed by Vite. To create a production bundle:

```bash
npm run build
```

## Technology

- React
- TypeScript
- Vite
- Tailwind CSS
- daisyUI
