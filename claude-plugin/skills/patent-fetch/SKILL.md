---
name: patent-fetch
description: "Get complete patent details including title, abstract, claims, description, assignee, filing dates, and legal status. Use when the user provides a patent ID or patent number and needs full patent information from Google Patents."
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
# IMPORTANT: Always use execute_cypher to get the patent data
# Do NOT read the output_file directly with Read tool
```

## Response Guidelines

**Default behavior (when user doesn't specify what they need):**
Return ONLY title and abstract_text:
```cypher
MATCH (p:Patent) RETURN p.title, p.abstract_text
```

**When user asks for specific information, include relevant fields:**

| User Request | Fields to Return |
|--------------|------------------|
| Assignee, owner, applicant | `p.assignee` |
| Legal status, status, expired/active | `p.legal_status` |
| Claims, what is claimed | `p.claims` |
| Description, details, specification | `p.description` |
| Filing date, application date | `p.filing_date` |
| Publication date | `p.publication_date` |
| Priority date | `p.priority_date` |
| Everything, full details | `p.title, p.abstract_text, p.description, p.assignee, p.filing_date, p.publication_date, p.priority_date, p.legal_status, p.claims` |

**Example queries based on user request:**

User: "Fetch patent US9152718B2"
→ Use default: `MATCH (p:Patent) RETURN p.title, p.abstract_text`

User: "Who owns patent US9152718B2?"
→ Include assignee: `MATCH (p:Patent) RETURN p.title, p.assignee`

User: "What's the legal status of US9152718B2?"
→ Include legal_status: `MATCH (p:Patent) RETURN p.title, p.legal_status`

User: "Get full details for US9152718B2"
→ Include everything: `MATCH (p:Patent) RETURN p.title, p.abstract_text, p.description, p.assignee, p.filing_date, p.publication_date, p.priority_date, p.legal_status, p.claims`

## Important Notes

- The `output_file` returned by fetch_patent is for debugging purposes only
- **Always use `execute_cypher` to access patent data** - do not use the Read tool on the output_file
- The dataset is automatically loaded into memory for efficient querying

## Parameters

- `patent_id` (string, required): Patent ID (e.g., "US9152718B2", "JP2023-123456-A")
- `language` (string, optional): Language/locale for patent pages (ja, en, zh)
