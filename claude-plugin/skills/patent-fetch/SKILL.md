---
name: patent-fetch
description: "Fetch detailed patent information by patent ID from Google Patents."
metadata:
  author: sonesuke
  version: 1.0.0
context: fork
agent: general-purpose
---

# Patent Fetch

Fetch detailed patent information by patent ID from Google Patents using the google-patent-cli MCP server.

## Purpose

Retrieve complete patent details including title, abstract, description, claims, assignee, filing dates, and legal status.

## MCP Tool

Uses `fetch_patent` MCP tool provided by google-patent-cli.

## Parameters

- `patent_id` (string, required): Patent ID in the format recognized by Google Patents
  - Examples: "US9152718B2", "JP2023-123456-A", "EP1234567B1"
- `raw` (boolean, optional): If true, returns raw HTML instead of structured JSON
- `language` (string, optional): Language/locale for patent pages (ja, en, zh)

## Usage

### Fetch patent details (JSON)

```
patent_fetch({
  patent_id: "US9152718B2"
})
```

### Fetch patent with language

```
patent_fetch({
  patent_id: "JP2023123456A",
  language: "en"
})
```

### Fetch raw HTML (for debugging)

```
patent_fetch({
  patent_id: "US9152718B2",
  raw: true
})
```

## Response Format

### JSON mode (raw: false or omitted)

Returns a JSON object containing:

- `output_file`: Path to the JSON file with patent details
- `schema`: JSON schema of the patent data
- `dataset`: Dataset name for Cypher queries (optional)
- `graph_schema`: Graph schema for Cypher queries (optional)

The patent data includes:
- `id`: Patent ID
- `title`: Patent title
- `abstract_text`: Abstract text
- `description_paragraphs`: Full description (array of paragraphs)
- `claims`: Claims text
- `assignee`: Assignee/Applicant name
- `filing_date`: Filing date
- `priority_date`: Priority date
- `legal_status`: Legal status (if available)
- `citations`: Cited patents
- `images`: Patent images/figures

### HTML mode (raw: true)

Returns a JSON object containing:

- `output_file`: Path to the HTML file
- `schema`: Schema (string type for HTML)

## Notes

- Patent details are automatically loaded into Cypher store for further querying
- Use standard patent ID formats recognized by Google Patents
- Raw HTML mode is useful for debugging or when you need the full page source
