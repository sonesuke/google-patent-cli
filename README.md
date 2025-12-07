# Google Patent CLI

A Rust‑based command‑line tool for extracting structured data from Google Patents. It retrieves the patent title, abstract, filing date, description paragraphs (with numbers), claims (with numbers), and image URLs while preserving their internal structure.

## Features
- Search patents by free‑text query
- Retrieve a single patent by number
- Filter results by priority date (`--before` / `--after`)
- Output JSON with full structured fields (`description_paragraphs`, `claims`, `images`)
- Optional `--raw` flag to output the raw HTML for debugging
- **Headless mode is the default**; use `--head` to show the Chrome window
- Custom CDP (Chrome DevTools Protocol) implementation for robust browser control
- Simple configuration for custom Chrome/Chromium executable path

## Installation

### Option 1: Download Pre-built Binaries (Recommended)

Download the latest release for your platform from the [Releases page](https://github.com/sonesuke/google-patent-cli/releases):

**Linux (x86_64):**
```bash
curl -LO https://github.com/sonesuke/google-patent-cli/releases/latest/download/google-patent-cli-linux-x86_64.tar.gz
tar xzf google-patent-cli-linux-x86_64.tar.gz
sudo mv google-patent-cli /usr/local/bin/
```

**macOS (Intel):**
```bash
curl -LO https://github.com/sonesuke/google-patent-cli/releases/latest/download/google-patent-cli-macos-x86_64.tar.gz
tar xzf google-patent-cli-macos-x86_64.tar.gz
sudo mv google-patent-cli /usr/local/bin/
```

**macOS (Apple Silicon):**
```bash
curl -LO https://github.com/sonesuke/google-patent-cli/releases/latest/download/google-patent-cli-macos-arm64.tar.gz
tar xzf google-patent-cli-macos-arm64.tar.gz
sudo mv google-patent-cli /usr/local/bin/
```

**Windows:**
1. Download `google-patent-cli-windows-x86_64.zip` from the [Releases page](https://github.com/sonesuke/google-patent-cli/releases)
2. Extract the ZIP file
3. Add the extracted `google-patent-cli.exe` to your PATH

### Option 2: Build from Source

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

### Filter by filing date
```bash
# Patents filed after 2020‑01‑01
google-patent-cli search --query "AI" --after "2020-01-01"

# Patents filed between 2018‑01‑01 and 2020‑12‑31
google-patent-cli search --query "blockchain" --after "2018-01-01" --before "2020-12-31"
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
