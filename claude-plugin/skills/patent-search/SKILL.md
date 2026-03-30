---
name: patent-search
description: "Execute patent searches on Google Patents. Supports free-text queries, date range filters (filing, priority, publication), assignee filters, country filters, and result limits. Always use this skill for any patent search, find, or lookup request — even when specific parameters are provided."
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

**CRITICAL**: After searching, always use `execute_cypher` to retrieve results.
Do NOT read the output JSON file directly. The JSON file is an internal
artifact — all data is available through cypher queries.

### Result Retrieval Patterns

Use these cypher patterns to retrieve search results:

**Total count**:

```cypher
MATCH (p:Patent) RETURN COUNT(*) AS count
```

**Top 20 snippets for noise analysis**:

```cypher
MATCH (p:Patent) RETURN p.id, p.title, p.snippet, p.assignee LIMIT 20
```

**Assignee breakdown**:

```cypher
MATCH (p:Patent) RETURN p.assignee, COUNT(*) AS count ORDER BY count DESC
```

**Date range summary**:

```cypher
MATCH (p:Patent) RETURN p.filing_date, p.title LIMIT 10
```

### Available Patent Node Fields

| Field              | Description                     |
| ------------------ | ------------------------------- |
| `id`               | Patent ID (e.g., "US9152718B2") |
| `title`            | Patent title                    |
| `snippet`          | Search result snippet           |
| `abstract_text`    | Full abstract                   |
| `assignee`         | Assignee/applicant name         |
| `filing_date`      | Filing date                     |
| `publication_date` | Publication date                |
| `url`              | Google Patents URL              |
| `legal_status`     | Legal status                    |
| `family_id`        | Patent family ID                |

### Date Filter Examples

Search patents filed in 2023:

```
patent_search({
  query: "artificial intelligence",
  filing_after: "2023-01-01",
  filing_before: "2023-12-31",
  limit: 10
})
```

Search patents with priority date in 2024:

```
patent_search({
  assignee: ["Google LLC"],
  priority_after: "2024-01-01",
  priority_before: "2024-12-31"
})
```

Search patents published in a specific date range:

```
patent_search({
  query: "quantum computing",
  publication_after: "2023-06-01",
  publication_before: "2023-12-31"
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
