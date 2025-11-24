(() => {
    const items = document.querySelectorAll("search-result-item");

    return Array.from(items)
        .map(item => {
            // Title
            const titleEl = item.querySelector(".result-title h3 raw-html span");
            const title = titleEl ? titleEl.innerText.trim() : "No Title";

            // ID - try multiple methods
            // Method 1: .pdfLink span (most common)
            let id = null;
            const idEl = item.querySelector(".pdfLink span");
            if (idEl) {
                id = idEl.innerText.trim();
            }

            // Method 2: Extract from result-title link href
            if (!id || id === "") {
                const titleLink = item.querySelector(".result-title a[href*='/patent/']");
                if (titleLink) {
                    const href = titleLink.getAttribute("href");
                    const match = href.match(/\/patent\/([A-Z0-9]+)/);
                    if (match) {
                        id = match[1];
                    }
                }
            }

            // Method 3: Look for patent number pattern in the entire item text
            if (!id || id === "") {
                const itemText = item.innerText;
                const patentMatch = itemText.match(/\b([A-Z]{2}\d{7,}[A-Z]\d?)\b/);
                if (patentMatch) {
                    id = patentMatch[1];
                }
            }

            // Fallback
            if (!id || id === "") {
                id = "Unknown";
            }

            // Dates
            const datesEl = item.querySelector("h4.dates");
            const datesText = datesEl ? datesEl.innerText.trim() : "";

            // Snippet
            // The snippet is usually in a raw-html span following the dates
            // It might be inside the abstract div
            let snippet = "";
            const abstractDiv = item.querySelector("div.abstract");
            if (abstractDiv) {
                const rawHtmls = abstractDiv.querySelectorAll("raw-html span");
                // The last one is usually the snippet, or we can join them
                // The first one might be assignee/inventor
                for (const span of rawHtmls) {
                    // Skip if it looks like a name (short) or if it's the title (already handled)
                    if (span.innerText.length > 50) {
                        snippet = span.innerText.trim();
                        break;
                    }
                }
            }

            let date = "Unknown";
            const priorityMatch = datesText.match(/Priority\s+(\d{4}-\d{2}-\d{2})/);
            if (priorityMatch) {
                date = priorityMatch[1];
            } else {
                const filedMatch = datesText.match(/Filed\s+(\d{4}-\d{2}-\d{2})/);
                if (filedMatch) {
                    date = filedMatch[1];
                }
            }

            return {
                id: id,
                title: title,
                snippet: snippet,
                filing_date: date,
                grant_date: null,
                publication_date: null,
                url: "https://patents.google.com/patent/" + id,
                abstract_text: null,
                description: null,
                description_paragraphs: null,
                claims: null,
                images: null
            };
        });
})()
