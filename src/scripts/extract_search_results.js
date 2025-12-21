(() => {
    // Extract total results count
    let totalResults = "Unknown";
    const countSpan = document.querySelector('search-results #count span.flex');
    if (countSpan) {
        totalResults = countSpan.innerText.trim();
    }

    const items = document.querySelectorAll("search-result-item");

    const patents = Array.from(items)
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
            let assignee = null;
            const abstractDiv = item.querySelector("div.abstract");
            if (abstractDiv) {
                const rawHtmls = abstractDiv.querySelectorAll("raw-html span");
                const texts = Array.from(rawHtmls).map(el => el.innerText.trim()).filter(t => t.length > 0);

                // Find the snippet (usually the longest text, or contains "...")
                let snippetIndex = -1;
                let maxLength = 0;

                for (let i = 0; i < texts.length; i++) {
                    const text = texts[i];
                    if (text.length > maxLength) {
                        maxLength = text.length;
                        snippetIndex = i;
                    }
                }

                if (snippetIndex !== -1) {
                    snippet = texts[snippetIndex];

                    // Everything before the snippet is likely Inventor/Assignee
                    const metadata = texts.slice(0, snippetIndex);
                    if (metadata.length > 0) {
                        assignee = metadata.join(", ");
                    }
                } else if (texts.length > 0) {
                    // Fallback: if no clear snippet, take the last one as snippet, others as assignee
                    snippet = texts[texts.length - 1];
                    if (texts.length > 1) {
                        assignee = texts.slice(0, texts.length - 1).join(", ");
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
                assignee: assignee,
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

    return {
        total_results: totalResults,
        patents: patents
    };
})()
