(() => {
    // Extract title from document.title
    let title = "No Title";
    const docTitle = document.title;
    if (docTitle) {
        const parts = docTitle.split(' - ');
        if (parts.length >= 2) {
            title = parts.slice(1, -1).join(' - ').trim();
        }
    }

    // Get abstract from meta description
    const metaDesc = document.querySelector('meta[name="description"]');
    const abstract = metaDesc ? metaDesc.getAttribute('content').trim() : null;

    // Extract description paragraphs with numbers
    const descParas = Array.from(document.querySelectorAll('div.description-paragraph[num]')).map(el => ({
        number: el.getAttribute('num'),
        id: el.id,
        text: el.innerText.trim()
    }));

    // Fallback for unstructured description (e.g., Japanese patents)
    if (descParas.length === 0) {
        // Look for "Description" heading
        const headings = Array.from(document.querySelectorAll("h2, h3, h4, b, strong"));
        let foundHeading = null;
        for (const h of headings) {
            if (h.innerText.trim() === "Description") {
                foundHeading = h;
                break;
            }
        }

        if (foundHeading) {
            let textContent = "";
            let sibling = foundHeading.nextSibling;

            while (sibling) {
                if (sibling.nodeType === 3) { // TEXT_NODE
                    if (sibling.textContent && sibling.textContent.trim() !== "") {
                        textContent += sibling.textContent.trim() + "\n";
                    }
                } else if (sibling.nodeType === 1) { // ELEMENT_NODE
                    const tag = sibling.tagName.toUpperCase();
                    // Stop at next section heading
                    if (tag === "H2" || tag === "H3" || tag === "H4" || (tag === "SECTION" && sibling.innerText.includes("Claims"))) {
                        break;
                    }
                    // Check if it's the Claims heading
                    if (sibling.innerText.trim() === "Claims") {
                        break;
                    }
                    textContent += sibling.innerText.trim() + "\n";
                }
                sibling = sibling.nextSibling;
            }

            if (textContent.trim() !== "") {
                descParas.push({
                    number: "00001",
                    id: "DESC-FULL",
                    text: textContent
                });
            }
        }
    }

    // Extract claims with numbers
    const claimsArray = Array.from(document.querySelectorAll('div.claim[num]')).map(el => ({
        number: el.getAttribute('num'),
        id: el.id,
        text: el.innerText.trim()
    }));

    // Extract images
    const images = Array.from(document.querySelectorAll('img[src*="patentimages"]')).map(img => {
        const src = img.src;
        const match = src.match(/D(\d+)\.png$/);
        return {
            url: src,
            figure_number: match ? `D${match[1]}` : null
        };
    });

    // Get filing date from meta tags
    let filingDate = null;
    const metaDate = document.querySelector('meta[name="DC.date"][scheme="dateSubmitted"]');
    if (metaDate) {
        filingDate = metaDate.getAttribute('content');
    }

    // Get assignee from meta tags or DL
    let assignee = null;
    const metaAssignee = document.querySelector('meta[name="DC.contributor"][scheme="assignee"]');
    if (metaAssignee) {
        assignee = metaAssignee.getAttribute('content');
    }

    if (!assignee) {
        // Fallback: Look for "Current Assignee" or "Original Assignee" in definitions
        const dts = document.querySelectorAll('dt');
        for (const dt of dts) {
            const text = dt.innerText.trim();
            if (text === "Current Assignee" || text === "Original Assignee" || text === "Assignee") {
                const dd = dt.nextElementSibling;
                if (dd && dd.tagName === 'DD') {
                    assignee = dd.innerText.trim();
                    break;
                }
            }
        }
    }

    return {
        title: title,
        abstract: abstract,
        description_paragraphs: descParas.length > 0 ? descParas : null,
        claims: claimsArray.length > 0 ? claimsArray : null,
        images: images.length > 0 ? images : null,
        filing_date: filingDate,
        assignee: assignee
    };
})()
