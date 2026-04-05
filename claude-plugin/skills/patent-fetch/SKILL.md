---
name: patent-fetch
description: "Fetch complete patent details from Google Patents including title, abstract, claims, description, images, assignee, filing dates, and legal status. Always use this skill when the user asks to fetch, get, or look up a specific patent by ID or number."
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
| Claims, what is claimed | See claims section below |
| Description, details, specification | `p.description` |
| Filing date, application date | `p.filing_date` |
| Publication date, `p.publication_date` |
| Priority date | `p.priority_date` |

**Claims retrieval:**

`p.claims` is always null. You MUST query `:claims` child nodes directly:

```cypher
MATCH (c:claims) RETURN c.number, c.text
```

**CRITICAL — Cypher Parser Limitations:**

The Cypher parser has known bugs. Follow these rules strictly:

1. **Do NOT use `ORDER BY`** — causes `c.text` to return `expression: null`
2. **Do NOT use `WHERE` clauses** — `WHERE d.text CONTAINS '...'` and `WHERE c.number = '1'` cause parse errors
3. **Do NOT use relationship patterns for property access** — `(p:Patent)-[:claims]->(c:claims) RETURN c.text` returns null
4. **Do NOT use `p.claims`** — always null, claims are stored as child nodes
5. **Do NOT use wrong labels** — `[:HAS_CHILD]->(c:claim)`, `[:claim]->(c:claim)`, `[:claims]->(c:claim)` return empty

**Safe query pattern**: `MATCH (c:claims) RETURN c.number, c.text` (direct node match, no ORDER BY, no WHERE)

**Example queries based on user request:**

User: "Fetch patent US9152718B2"
→ Use default: `MATCH (p:Patent) RETURN p.title, p.abstract_text`

User: "Who owns patent US9152718B2?"
→ Include assignee: `MATCH (p:Patent) RETURN p.title, p.assignee`

User: "What's the legal status of US9152718B2?"
→ Include legal_status: `MATCH (p:Patent) RETURN p.title, p.legal_status`

User: "Get full details for US9152718B2"
→ Include everything:
```cypher
MATCH (p:Patent) RETURN p.title, p.abstract_text, p.description, p.assignee, p.filing_date, p.publication_date, p.legal_status
```
Then get claims separately:
```cypher
MATCH (c:claims) RETURN c.number, c.text
```

## Important Notes

- The `output_file` returned by fetch_patent is for debugging purposes only
- **Always use `execute_cypher` to access patent data** - do not use the Read tool on the output_file
- The dataset is automatically loaded into memory for efficient querying

## Graph Structure

Patent data is loaded as a graph with the following structure:

- **Patent node** (`:Patent`) - Main patent with id, title, abstract_text, etc.
- **Array fields become relationships**:
  - `(:Patent)-[:claims]->(:claims)` - Claim nodes with number, text
  - `(:Patent)-[:description_paragraphs]->(:description_paragraphs)` - Description nodes
  - `(:Patent)-[:images]->(:images)` - Image nodes

**Accessing claims (direct node match — do NOT use relationship patterns or ORDER BY/WHERE):**
```cypher
MATCH (c:claims) RETURN c.number, c.text
```

## Parameters

- `patent_id` (string, required): Patent ID (e.g., "US9152718B2", "JP2023-123456-A")
- `language` (string, optional): Language/locale for patent pages (ja, en, zh)
