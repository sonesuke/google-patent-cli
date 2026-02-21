# Google Patent CLI

A Rust‑based command‑line tool for extracting structured data from Google Patents. It retrieves the patent title, abstract, filing date, description paragraphs (with numbers), claims (with numbers), and image URLs while preserving their internal structure.

## Features
- Search patents by free‑text query
- Retrieve a single patent by number
- Filter results by priority date (`--before` / `--after`)
- Filter by country (`--country JP`)
- **Language/locale support** (`--language ja`) for translated patent pages
- Output JSON with full structured fields (`description_paragraphs`, `claims`, `images`)
- Optional `--raw` flag to output the raw HTML for debugging
- **Headless mode is the default**; use `--head` to show the Chrome window
- Custom CDP (Chrome DevTools Protocol) implementation for robust browser control
- **MCP (Model Context Protocol) server** for AI agent integration
- Simple configuration for custom Chrome/Chromium executable path

## Installation

### macOS / Linux (one-liner)
```bash
curl -fsSL https://raw.githubusercontent.com/sonesuke/google-patent-cli/main/install.sh | bash
```

### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/sonesuke/google-patent-cli/main/install.ps1 | iex
```

### Build from Source
```bash
cargo install --path .
```

## Usage
### Search by query
```bash
google-patent-cli search --query "machine learning" --before "2020-01-01"
```

### Show the browser window
```bash
google-patent-cli search --query "machine learning" --head
```

### Filter by assignee
```bash
# Patents assigned to a specific company
google-patent-cli search --query "machine learning" --assignee "Google"
```

### Filter by filing date
```bash
# Patents filed after 2020‑01‑01
google-patent-cli search --query "AI" --after "2020-01-01"

# Patents filed between 2018‑01‑01 and 2020‑12‑31
google-patent-cli search --query "blockchain" --after "2018-01-01" --before "2020-12-31"
```

### Filter by country
```bash
# Patents valid in Japan (JP), United States (US), or China (CN)
google-patent-cli search --query "camera" --country JP
```

### Lookup by patent number
```bash
google-patent-cli fetch "US10000000"
```

### Output raw HTML (debug)
The `--raw` flag disables the structured JSON extraction and prints the full HTML source of the requested patent page. This is useful for debugging or inspecting the page manually.
```bash
google-patent-cli fetch "US20220319181A1" --raw
```

### Language/locale
```bash
# Fetch patent page in Japanese
google-patent-cli fetch US9152718B2 --language ja

# Search with Japanese locale
google-patent-cli search --query "machine learning" --language ja
```

### MCP server
Start the MCP server for AI agent integration:
```bash
google-patent-cli mcp
```

The MCP server exposes two tools:
- `search_patents` — Search Google Patents with query, assignee, country, date, and language filters
- `fetch_patent` — Fetch a specific patent by ID (JSON or raw HTML)

## Configuration
The CLI stores a simple TOML config file at `~/.config/google-patent-cli/config.toml`. The tool automatically detects the default Chrome installation path for your OS. To set a custom Chrome/Chromium executable path (useful on non‑standard installations):

**macOS/Linux:**
```bash
google-patent-cli config --set-browser "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
```

**Windows:**
```bash
google-patent-cli config --set-browser "C:\Program Files\Google\Chrome\Application\chrome.exe"
```

If the config file does not exist, the first run creates it with default values.

## Requirements
- Chrome or Chromium installed on the system
  - Windows: `C:\Program Files\Google\Chrome\Application\chrome.exe`
  - macOS: `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`
  - Linux: `/usr/bin/google-chrome`
- Rust 1.81 or newer

## Implementation Details
The tool communicates with Chrome via a lightweight CDP client built on `tokio‑tungstenite`. It waits for dynamic page content (description, claims, images) using `page.wait_for_element` instead of fixed `sleep` calls, ensuring reliable extraction across patent pages.

## License
MIT
