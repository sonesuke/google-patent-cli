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

## Parameters

- `company_name` (string, required): Company name to check
- `country` (string, optional): Filter by country code to narrow results
- `limit` (number, optional): Maximum results for analysis (default: 100)

## Usage

### Check assignee variations

```
patent_assignee_check({
  company_name: "Google"
})
```

### Check with country filter

```
patent_assignee_check({
  company_name: "Toyota",
  country: "JP",
  limit: 200
})
```

## Process

1. **Search**: Execute `search_patents` with the assignee parameter
2. **Analyze**: Extract and group assignee names from results
3. **Frequency**: Calculate frequency of each assignee name variation
4. **Report**: Return top variations with counts

## Response Format

The underlying `search_patents` MCP tool returns:

- `dataset`: **Dataset name** for Cypher queries (results are auto-loaded into Cypher store)
- `graph_schema`: **Graph schema** for constructing Cypher queries
- `count`: Number of patents found

The skill analyzes the results and reports:

- `company_name`: The input company name
- `assignee_variations`: Array of assignee name variations found
  - `assignee`: Assignee name variation
  - `count`: Number of patents with this assignee name
  - `percentage`: Percentage of total results
- `total_results`: Total number of patents analyzed
- `top_assignee`: Most common assignee name (likely the official name)

## Example Output

```
Company: Google
Total results: 1655

Top assignee variations:
1. Google LLC (75.5%) - 1250 patents
2. Google Inc. (21.1%) - 350 patents
3. Alphabet Inc. (3.4%) - 55 patents

Most common: Google LLC (likely the official name)

Dataset: search-abc123
Graph schema: Patent {id, title, assignee, ...}
```

## Notes

- Search results are **automatically loaded into Cypher store** for querying
- Use the returned `dataset` name with `execute_cypher` for further analysis
- The top assignee by count is likely the official/canonical name
- All variations should be considered when conducting comprehensive patent searches

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
