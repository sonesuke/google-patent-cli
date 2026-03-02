---
name: patent-fetch
description: "Fetch complete patent details including title, abstract, claims, description, assignee, filing dates, and legal status. Use when the user provides a patent ID or patent number and needs full patent information from Google Patents."
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

## Usage

Fetch a patent, then use the returned `dataset` name to query with Cypher:

```
patent_fetch({
  patent_id: "US9152718B2"
})

# Returns dataset name like "fetch-abc123"
# Then query with execute_cypher:
execute_cypher({
  dataset: "fetch-abc123",
  query: "MATCH (p:Patent) RETURN p.title, p.abstract_text, p.assignee"
})
```

## Parameters

- `patent_id` (string, required): Patent ID (e.g., "US9152718B2", "JP2023-123456-A")
- `language` (string, optional): Language/locale for patent pages (ja, en, zh)
