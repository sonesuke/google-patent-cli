---
name: patent-search
description: "Search for patents from Google Patents by query, assignee, country, and/or date range."
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

## Parameters

- `query` (string, optional): Free-text search query
- `assignee` (string, optional): Filter by assignee/applicant name
- `country` (string, optional): Filter by country code (JP, US, CN, EP)
- `after` (string, optional): Filter by priority date after (YYYY-MM-DD)
- `before` (string, optional): Filter by priority date before (YYYY-MM-DD)
- `limit` (number, optional): Maximum number of results (default: 10)
- `language` (string, optional): Language/locale (ja, en, zh)

## Usage

### Search by query

```
patent_search({
  query: "machine learning",
  limit: 20
})
```

### Search by assignee

```
patent_search({
  assignee: "Google LLC",
  country: "US",
  limit: 50
})
```

### Search with date range

```
patent_search({
  query: "transformer architecture",
  after: "2015-01-01",
  before: "2023-12-31",
  country: "US"
})
```

## Response Format

Returns a JSON object containing:

- `dataset`: **Dataset name** for Cypher queries (data is auto-loaded into Cypher store)
- `graph_schema`: **Graph schema** for constructing Cypher queries
- `count`: Number of patents found
- `output_file`: Path to the JSON file (for reference/debugging)
- `schema`: JSON schema of the search results

## Notes

- Search results are **automatically loaded into Cypher store** for querying
- Use the returned `dataset` name with `execute_cypher` to query the results
- The `graph_schema` describes the node types and relationships available for queries
- At least one of `query` or `assignee` should be provided for meaningful results
