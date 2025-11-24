use anyhow::Result;

use crate::cdp::{CdpBrowser, CdpPage};
use crate::models::{Patent, SearchOptions};

pub struct PatentSearcher {
    browser: CdpBrowser,
}

impl PatentSearcher {
    pub async fn new(headless: bool, debug: bool) -> Result<Self> {
        let args = vec!["--disable-blink-features=AutomationControlled"];
        let browser = CdpBrowser::launch(None, args, headless, debug).await?;

        Ok(Self { browser })
    }

    /// Search for patents or fetch a specific patent
    pub async fn search(&self, options: &SearchOptions) -> Result<Vec<Patent>> {
        self.search_internal(options).await
    }

    /// Get raw HTML for a patent page (for debugging)
    pub async fn get_raw_html(&self, patent_number: &str) -> Result<String> {
        let url = format!("https://patents.google.com/patent/{}", patent_number);
        let page_ws_url = self.browser.new_page().await?;
        let page = CdpPage::new(&page_ws_url).await?;
        
        page.goto(&url).await?;
        
        // Wait for page to load (meta description)
        let loaded = page.wait_for_element("meta[name='description']", 15).await?;
        if !loaded {
            return Err(anyhow::anyhow!("Page failed to load within timeout"));
        }
        // Additional wait for description paragraphs, claims, and images to appear
        let _ = page.wait_for_element("div.description-line[num], div.claim[num], img[src*='patentimages']", 15).await?;
        // Additional wait for description paragraphs, claims, and images to appear
        let _ = page.wait_for_element("div.description-line[num], div.claim[num], img[src*='patentimages']", 15).await?;
        // Additional wait for description paragraphs, claims, and images to appear
        let _ = page.wait_for_element("div.description-line[num], div.claim[num], img[src*='patentimages']", 15).await?;
        
        page.get_html().await
    }


    #[allow(clippy::option_if_let_else)]
    async fn search_internal(&self, options: &SearchOptions) -> Result<Vec<Patent>> {
        let page_ws_url = self.browser.new_page().await?;
        let page = CdpPage::new(&page_ws_url).await?;

        let url = if let Some(patent_number) = &options.patent_number {
            format!("https://patents.google.com/patent/{}", patent_number)
        } else if let Some(query) = &options.query {
            let mut url_str = format!("https://patents.google.com/?q={}", query.replace(' ', "+"));

            if let Some(after) = &options.after_date {
                url_str.push_str(&format!("&after={}", after));
            }

            if let Some(before) = &options.before_date {
                url_str.push_str(&format!("&before={}", before));
            }

            url_str
        } else {
            return Err(anyhow::anyhow!("Must provide either --query or --patent"));
        };

        page.goto(&url).await?;

        if let Some(patent_number) = &options.patent_number {
            // Wait for meta description tag to ensure page is loaded
            let loaded = page.wait_for_element("meta[name='description']", 15).await?;
            if !loaded {
                return Err(anyhow::anyhow!("Page failed to load within timeout"));
            }
            // Additional wait for description paragraphs, claims, and images to appear
            let _ = page.wait_for_element("div.description-line[num], div.claim[num], img[src*='patentimages']", 15).await?;
            
            // Single patent page - extract structured data
            let result = page
                .evaluate(
                    r#"
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
                    const descParas = Array.from(document.querySelectorAll('div.description-line[num]')).map(el => ({
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
                    
                    return {
                        title: title,
                        abstract: abstract,
                        description_paragraphs: descParas.length > 0 ? descParas : null,
                        claims: claimsArray.length > 0 ? claimsArray : null,
                        images: images.length > 0 ? images : null,
                        filing_date: filingDate
                    };
                })()
            "#,
                )
                .await?;

            let title = result["title"]
                .as_str()
                .unwrap_or("No Title")
                .to_string();
            let abstract_text = result["abstract"].as_str().map(String::from);
            
            // Parse description paragraphs
            let description_paragraphs = if let Some(paras) = result["description_paragraphs"].as_array() {
                let parsed: Vec<crate::models::DescriptionParagraph> = paras
                    .iter()
                    .filter_map(|p| {
                        Some(crate::models::DescriptionParagraph {
                            number: p["number"].as_str()?.to_string(),
                            id: p["id"].as_str()?.to_string(),
                            text: p["text"].as_str()?.to_string(),
                        })
                    })
                    .collect();
                if parsed.is_empty() { None } else { Some(parsed) }
            } else {
                None
            };
            
            // Parse claims
            let claims = if let Some(claims_arr) = result["claims"].as_array() {
                let parsed: Vec<crate::models::Claim> = claims_arr
                    .iter()
                    .filter_map(|c| {
                        Some(crate::models::Claim {
                            number: c["number"].as_str()?.to_string(),
                            id: c["id"].as_str()?.to_string(),
                            text: c["text"].as_str()?.to_string(),
                        })
                    })
                    .collect();
                if parsed.is_empty() { None } else { Some(parsed) }
            } else {
                None
            };
            
            // Parse images
            let images = if let Some(imgs) = result["images"].as_array() {
                let parsed: Vec<crate::models::PatentImage> = imgs
                    .iter()
                    .filter_map(|img| {
                        Some(crate::models::PatentImage {
                            url: img["url"].as_str()?.to_string(),
                            figure_number: img["figure_number"].as_str().map(String::from),
                        })
                    })
                    .collect();
                if parsed.is_empty() { None } else { Some(parsed) }
            } else {
                None
            };
            
            let filing_date = result["filing_date"].as_str().map(String::from);

            Ok(vec![Patent {
                id: patent_number.clone(),
                title,
                abstract_text,
                description_paragraphs,
                claims,
                images,
                snippet: None,
                description: None,
                filing_date,
                url,
            }])
        } else {
            // Search results page - use JavaScript evaluation
            
            // Wait for results to load
            // We wait for the PDF link class which is specific to results
            let loaded = page.wait_for_element(".search-result-item", 15).await?;
            if !loaded {
                // If it times out, it might be because there are no results, or network issues.
            }

            let results = page
                .evaluate(
                    r#"
                (() => {
                    const items = document.querySelectorAll("search-result-item");
                    
                    return Array.from(items)
                        .map(item => {
                            // Title
                            const titleEl = item.querySelector(".result-title h3 raw-html span");
                            const title = titleEl ? titleEl.innerText.trim() : "No Title";
                            
                            // ID
                            const idEl = item.querySelector(".pdfLink span");
                            const id = idEl ? idEl.innerText.trim() : "Unknown";
                            
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
            "#,
                )
                .await?;

            let mut patents: Vec<Patent> = serde_json::from_value(results)?;
            
            if let Some(limit) = options.limit {
                if patents.len() > limit {
                    patents.truncate(limit);
                }
            }
            
            Ok(patents)
        }
    }
}
