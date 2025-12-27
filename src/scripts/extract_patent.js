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

    // Extract Related Applications
    let relatedApplication = null;
    let claimingPriority = [];
    let familyApplications = [];

    // Helper to extract application info from a row
    const extractAppInfo = (row) => {
        const appNumEl = row.querySelector('[itemprop="applicationNumber"]');
        const priorityDateEl = row.querySelector('[itemprop="priorityDate"]');
        const filingDateEl = row.querySelector('[itemprop="filingDate"]');
        const titleEl = row.querySelector('[itemprop="title"]');

        if (!appNumEl) return null;

        const appNum = appNumEl.innerText.trim();
        // Simple heuristic for country code: first 2 chars of app number if they are letters
        let countryCode = null;
        if (/^[A-Z]{2}/.test(appNum)) {
            countryCode = appNum.substring(0, 2);
        }

        return {
            application_number: appNum,
            country_code: countryCode,
            priority_date: priorityDateEl ? priorityDateEl.innerText.trim() : null,
            filing_date: filingDateEl ? filingDateEl.innerText.trim() : null,
            title: titleEl ? titleEl.innerText.trim() : null
        };
    };

    // Extract Claims Priority
    const priorityRows = document.querySelectorAll('tr[itemprop="appsClaimingPriority"]');
    for (const row of priorityRows) {
        const info = extractAppInfo(row);
        if (info) claimingPriority.push(info);
    }

    // Extract Family Applications
    const familyRows = document.querySelectorAll('tr[itemprop="applications"]');
    for (const row of familyRows) {
        const info = extractAppInfo(row);
        if (info) familyApplications.push(info);
    }

    // Method 3: Worldwide Applications Timeline (Fallback)
    if (claimingPriority.length === 0 && familyApplications.length === 0) {
        const timeline = document.querySelector('.application-timeline');
        if (timeline) {
            const modifiers = timeline.querySelectorAll('state-modifier[data-result^="patent/"]');
            for (const mod of modifiers) {
                const resultPath = mod.getAttribute('data-result');
                const id = resultPath.split('/')[1];

                let appNum = null;
                let filingDate = null;
                let legalStatus = null;

                const tooltip = mod.nextElementSibling;
                if (tooltip && tooltip.tagName === 'OVERLAY-TOOLTIP') {
                    const lines = tooltip.innerText.split('\n');
                    for (const line of lines) {
                        const trimmed = line.trim();
                        if (trimmed.startsWith('Application number:')) {
                            appNum = trimmed.replace('Application number:', '').trim();
                        } else if (trimmed.startsWith('Filing date:')) {
                            filingDate = trimmed.replace('Filing date:', '').trim();
                        }
                    }
                }

                if (id) {
                    // Determine if it's family or priority based on context or just add to family for now
                    // The timeline usually shows the family.
                    // Filter out the current patent if needed, but useful to include.

                    const info = {
                        application_number: appNum || id, // Use ID as fallback for app number
                        country_code: id.substring(0, 2),
                        priority_date: null,
                        filing_date: filingDate,
                        title: null // Title not usually in timeline tooltip
                    };

                    // Avoid duplicates
                    if (!familyApplications.some(existing => existing.application_number === info.application_number)) {
                        familyApplications.push(info);
                    }
                }
            }
        }
    }

    // Method 1: Look for "Related Applications" section in description
    const headings = Array.from(document.querySelectorAll("h2, h3, h4, div.heading, b, strong, heading"));
    for (const h of headings) {
        if (h.innerText.trim().toUpperCase().includes("RELATED APPLICATIONS") || h.innerText.trim().toUpperCase().includes("CROSS-REFERENCE")) {
            let sibling = h.nextElementSibling;
            if (sibling) {
                relatedApplication = sibling.innerText.trim();
            }
            break;
        }
    }

    // Method 2: Look for extract specific text patterns at the beginning of the description
    if (!relatedApplication && descParas.length > 0) {
        const firstPara = descParas[0].text;
        if (firstPara.match(/(?:division|continuation|continuation-in-part) of/i)) {
            relatedApplication = firstPara;
        }
    }

    return {
        title: title,
        abstract: abstract,
        description_paragraphs: descParas.length > 0 ? descParas : null,
        claims: claimsArray.length > 0 ? claimsArray : null,
        images: images.length > 0 ? images : null,
        filing_date: filingDate,
        assignee: assignee,
        related_application: relatedApplication,
        claiming_priority: claimingPriority.length > 0 ? claimingPriority : null,
        family_applications: familyApplications.length > 0 ? familyApplications : null
    };
})()
