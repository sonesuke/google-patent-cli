# Google Patent CLI - AI-ready
 
An AI-ready search and fetch tool for Google Patents, designed for both humans and AI agents. It extracts structured data including title, abstract, filing date, assignee, description paragraphs, claims, and images.
 
## Features
- **Search patents** by free-text query, assignee, country, and date.
- **Fetch patent details** by patent number (e.g., "US10000000").
- **Formatted JSON output** including `description_paragraphs` and `claims`.
- **Pagination support** via `--limit` option.
- **Date filtering** with `--before` and `--after`.
- **Country filtering** with `--country` (e.g., JP, US, CN).
- **Language/locale support** with `--language` (e.g., ja, en).
- **Raw HTML output** with `--raw` flag for debugging.
- **Headless mode** by default; use `--head` to show the browser.
- **Model Context Protocol (MCP)** support to integrate with AI agents.
- **Robust formatting**: Uses structured JSON for easy machine consumption.
 
## Installation
 
### Easy Install (Recommended)
 
**Linux & macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/sonesuke/google-patent-cli/main/install.sh | bash
```
> Note: On Linux, this installs to `~/.local/bin` without requiring `sudo`. Make sure `~/.local/bin` is in your `PATH`.
 
**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/sonesuke/google-patent-cli/main/install.ps1 | iex
```
 
### From Source (Cargo)
If you have Rust installed, you can build from source:
```bash
cargo install --path .
```
 
## Model Context Protocol (MCP)
 
`google-patent-cli` supports the [Model Context Protocol](https://modelcontextprotocol.io/), allowing AI agents (like Claude Desktop) to search and fetch patents directly.
 
### Available Tools
 
| Tool Name | Description | Parameters |
|---|---|---|
| `search_patents` | Search Google Patents matching a query, assignee, and date filters. | `query`, `assignee`, `limit`, `before`, `after`, `country`, `language` |
| `fetch_patent` | Fetch details (metadata, description, claims) of a specific patent. | `patent_id` (required, e.g., "US9152718B2"), `language`, `raw` |
 
### Usage
To start the MCP server over `stdio`:
```bash
google-patent-cli mcp
```
 
### Configuration for Claude Desktop
 
Add this to your `claude_desktop_config.json`:
 
```json
{
  "mcpServers": {
    "google-patent-cli": {
      "command": "/path/to/google-patent-cli",
      "args": ["mcp"]
    }
  }
}
```
 
## CLI Usage
 
### CLI Commands
 
| Command | Description | Example |
|---|---|---|
| `search` | Search for patents matching a query/assignee. | `google-patent-cli search --query "machine learning" --limit 10` |
| `fetch` | Fetch a single patent's metadata and data. | `google-patent-cli fetch US9152718B2` |
| `config` | Manage configuration settings. | `google-patent-cli config --set-browser "/path/to/chrome"` |
| `mcp` | Start the MCP server over stdio. | `google-patent-cli mcp` |
 
### Search by query
Search for patents matching a query.
```bash
google-patent-cli search --query "machine learning" --limit 10
```
 
### Filter by assignee
```bash
google-patent-cli search --query "AI" --assignee "Google"
```
 
### Filter by date and country
```bash
# Patents filed after 2024-01-01 in Japan
google-patent-cli search --query "camera" --after "2024-01-01" --country JP
 
# Patents filed between 2023-01-01 and 2023-12-31
google-patent-cli search --query "blockchain" --after "2023-01-01" --before "2023-12-31"
```
 
### Fetch patent details
Fetch a single patent's metadata, description, and claims.
```bash
google-patent-cli fetch US9152718B2
```
 
### Language/locale support
Fetch or search using a specific language locale.
```bash
google-patent-cli fetch US9152718B2 --language ja
```
 
### Output raw HTML (debug)
Prints the full HTML source instead of structured JSON.
```bash
google-patent-cli fetch US9152718B2 --raw > patent.html
```
 
### Show the browser window
Useful for debugging.
```bash
google-patent-cli search --query "AI" --head
```
 
## Configuration
This tool relies on a compatible Chrome/Chromium installation for scraping.
 
### Manage Configuration
You can manage the configuration via CLI:
 
```bash
# Show current configuration and config file path
google-patent-cli config
 
# Set custom browser path
google-patent-cli config --set-browser "/path/to/chrome"
```
 
## License
MIT
