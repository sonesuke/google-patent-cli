(async () => {
    // Helper function to wait for elements with retry
    async function waitForElements(selector, maxAttempts = 10, delayMs = 300) {
        for (let i = 0; i < maxAttempts; i++) {
            const elements = document.querySelectorAll(selector);
            if (elements.length > 0) return elements;
            await new Promise(r => setTimeout(r, delayMs));
        }
        return document.querySelectorAll(selector);
    }

    // Helper function to wait for a single element
    async function waitForElement(selector, maxAttempts = 10, delayMs = 300) {
        for (let i = 0; i < maxAttempts; i++) {
            const element = document.querySelector(selector);
            if (element) return element;
            await new Promise(r => setTimeout(r, delayMs));
        }
        return document.querySelector(selector);
    }

    // Helper function to find element by text content
    function findElementByText(text, root = document) {
        const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, null, false);
        let node;
        while (node = walker.nextNode()) {
            if (node.textContent.trim() === text) {
                return node.parentElement;
            }
        }
        const allElements = root.querySelectorAll('*');
        for (const el of allElements) {
            if (el.shadowRoot) {
                const found = findElementByText(text, el.shadowRoot);
                if (found) return found;
            }
        }
        return null;
    }

    // Helper function to extract summary items by data attribute
    function extractSummaryItemsByAttribute(dataAttr) {
        const items = [];
        const stateModifiers = document.querySelectorAll(`state-modifier[${dataAttr}]`);

        for (const modifier of stateModifiers) {
            const name = modifier.getAttribute(dataAttr);
            // Find the percentage value - it's in a sibling .value element
            const nameBlock = modifier.closest('.nameblock');
            let percentage = '';
            if (nameBlock) {
                const valueEl = nameBlock.querySelector('.value');
                if (valueEl) {
                    percentage = valueEl.textContent.trim();
                }
            }
            if (name) {
                items.push({ name, percentage });
            }
        }
        return items;
    }

    // Extract total results count with retry
    let totalResults = "Unknown";
    const countSpan = await waitForElement('search-results #count span.flex');
    if (countSpan) {
        totalResults = countSpan.innerText.trim();
    }

    // --- Extract Assignees using data-assignee attribute ---
    // First, click Expand to show all items
    let expandBtn = findElementByText('Expand');
    if (expandBtn) {
        expandBtn.click();
        // Wait a bit for expansion animation
        await new Promise(r => setTimeout(r, 200));
    }

    let topAssignees = extractSummaryItemsByAttribute('data-assignee');
    if (topAssignees.length === 0) {
        topAssignees = null;
    }

    // CPCs are extracted separately via two-step process in Rust
    let topCpcs = null;

    // --- Extract Patents with retry ---
    const items = await waitForElements("search-result-item");

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
        top_assignees: topAssignees,
        top_cpcs: topCpcs,
        patents: patents
    };
})()

