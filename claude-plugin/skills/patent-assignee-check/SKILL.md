---
name: patent-assignee-check
description: "Identify and verify assignee name spelling variations in patent databases. Check how a company name appears across patents (e.g., Google Inc vs Google LLC)."
metadata:
  author: sonesuke
  version: 1.0.0
context: fork
agent: general-purpose
---

# Patent Assignee Check

Check assignee name variations and verify the correct assignee name used in patent databases.

## Purpose

Identify spelling variations and official assignee names for a given company in patent databases. This helps ensure comprehensive patent searches by capturing all name variations.

## MCP Tool

Uses `search_patents` MCP tool provided by google-patent-cli.

## Usage

Search for patents by assignee to find name variations, then use Cypher to analyze:

```
patent_assignee_check({
  company_name: "Toyota",
  country: "JP"
})

# Returns dataset name like "search-abc123"
# Then query with execute_cypher to find variations:
execute_cypher({
  dataset: "search-abc123",
  query: "MATCH (p:Patent) RETURN p.assignee, count(*) ORDER BY count(*) DESC"
})
```

## Parameters

- `company_name` (string, required): Company name to check for variations
- `country` (string, optional): Filter by country code (JP, US, CN)
- `limit` (number, optional): Maximum results (default: 100)

## Common Variations

Typical assignee name variations include:

- **Legal form changes**: Inc. → LLC, Ltd. → Co., Ltd.
- **Mergers/Acquisitions**: Old company names vs new parent company names
- **Spelling differences**: Brackets, commas, abbreviations
- **Subsidiaries**: Parent company vs subsidiary names

## Integration with Patent Search

After identifying assignee variations, use them in `patent-search`:

```
# Search with multiple assignee variations
patent_search({
  assignee: "Google LLC",
  country: "US"
})
```
