---
name: patent-search
description: "Search and analyze patents from Google Patents with flexible filters including query, assignee, country, and date range. Use when the user wants to find, discover, or research patents by topic, company, technology, or assignee name."
metadata:
  author: sonesuke
  version: 1.0.0
context: fork
agent: general-purpose
---

# Patent Search

Search for patents from Google Patents using the google-patent-cli MCP server.

## Purpose

Execute patent searches with various filters including query, assignee, country, and date range.

## MCP Tool

Uses `search_patents` MCP tool provided by google-patent-cli.

## Usage

Search for patents, then use the returned `dataset` name to query with Cypher:

```
patent_search({
  query: "machine learning",
  limit: 20
})

# Returns dataset name like "search-abc123"
# Then query with execute_cypher:
execute_cypher({
  dataset: "search-abc123",
  query: "MATCH (p:Patent) RETURN p.title, p.assignee LIMIT 5"
})
```

## Parameters

- `query` (string, optional): Free-text search query
- `assignee` (array of strings, optional): Filter by assignee/applicant names
- `country` (string, optional): Filter by country code (JP, US, CN, EP)

### Date Filters

Three independent date filters are available:

- `priority_after` (string, optional): Filter by priority date after (YYYY-MM-DD)
- `priority_before` (string, optional): Filter by priority date before (YYYY-MM-DD)
- `publication_after` (string, optional): Filter by publication date after (YYYY-MM-DD)
- `publication_before` (string, optional): Filter by publication date before (YYYY-MM-DD)
- `filing_after` (string, optional): Filter by filing date after (YYYY-MM-DD)
- `filing_before` (string, optional): Filter by filing date before (YYYY-MM-DD)

### Other Parameters

- `limit` (number, optional): Maximum number of results (default: 10)
- `language` (string, optional): Language/locale (ja, en, zh)
