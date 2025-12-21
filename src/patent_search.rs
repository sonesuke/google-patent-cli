use anyhow::Result;

use crate::cdp::{CdpBrowser, CdpPage};
use crate::models::{Patent, SearchOptions, SearchResult};

pub struct PatentSearcher {
    browser: CdpBrowser,
}

impl PatentSearcher {
    pub async fn new(
        browser_path: Option<std::path::PathBuf>,
        headless: bool,
        debug: bool,
    ) -> Result<Self> {
        let args = vec!["--disable-blink-features=AutomationControlled"];
        let browser = CdpBrowser::launch(browser_path, args, headless, debug).await?;

        Ok(Self { browser })
    }

    /// Search for patents or fetch a specific patent
    pub async fn search(&self, options: &SearchOptions) -> Result<SearchResult> {
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
        let _ = page
            .wait_for_element(
                "div.description-line[num], div.claim[num], img[src*='patentimages']",
                15,
            )
            .await?;

        page.get_html().await
    }

    async fn search_internal(&self, options: &SearchOptions) -> Result<SearchResult> {
        let page_ws_url = self.browser.new_page().await?;
        let page = CdpPage::new(&page_ws_url).await?;

        let base_url = options.to_url()?;

        if let Some(patent_number) = &options.patent_number {
            // Single patent lookup - no pagination needed
            page.goto(&base_url).await?;

            // Wait for meta description tag to ensure page is loaded
            let loaded = page.wait_for_element("meta[name='description']", 15).await?;
            if !loaded {
                return Err(anyhow::anyhow!("Page failed to load within timeout"));
            }
            // Additional wait for description paragraphs, claims, and images to appear
            let _ = page
                .wait_for_element(
                    "div.description-line[num], div.claim[num], img[src*='patentimages']",
                    15,
                )
                .await?;

            // Single patent page - extract structured data
            let result = page.evaluate(include_str!("scripts/extract_patent.js")).await?;

            // For single patent, total_results is "1".
            let patents = parse_single_patent_result(result, patent_number, base_url)?;
            Ok(SearchResult { total_results: "1".to_string(), patents })
        } else {
            // Search results page - may need pagination
            let mut all_patents: Vec<Patent> = Vec::new();
            let limit = options.limit.unwrap_or(10);
            let mut total_results_str = "Unknown".to_string();

            // Calculate how many pages we need to fetch
            // Google Patents shows 10 results per page
            let pages_needed = limit.div_ceil(10);

            for page_num in 0..pages_needed {
                // Construct URL with page parameter
                // First page (page_num=0): no &page parameter
                // Second page (page_num=1): &page=1
                // Third page (page_num=2): &page=2, etc.
                let page_url = if page_num == 0 {
                    base_url.clone()
                } else {
                    format!("{}&page={}", base_url, page_num)
                };

                page.goto(&page_url).await?;

                // Wait for results to load
                let loaded = page.wait_for_element(".search-result-item", 15).await?;
                if !loaded {
                    // No results on this page, stop pagination
                    break;
                }

                let results =
                    page.evaluate(include_str!("scripts/extract_search_results.js")).await?;

                let sr: SearchResult = serde_json::from_value(results)?;

                // Only capture total results from the first page (or all pages, should be same)
                if page_num == 0 {
                    total_results_str = sr.total_results;
                }

                let page_patents = sr.patents;

                // If we got no results, stop pagination
                if page_patents.is_empty() {
                    break;
                }

                all_patents.extend(page_patents);

                // If we've collected enough results, stop
                if all_patents.len() >= limit {
                    break;
                }
            }

            // Truncate to exact limit
            if all_patents.len() > limit {
                all_patents.truncate(limit);
            }

            Ok(SearchResult { total_results: total_results_str, patents: all_patents })
        }
    }
}

fn parse_single_patent_result(
    result: serde_json::Value,
    patent_number: &str,
    url: String,
) -> Result<Vec<Patent>> {
    let title = result["title"].as_str().unwrap_or("No Title").to_string();
    let abstract_text = result["abstract"].as_str().map(String::from);

    // Parse description paragraphs
    let description_paragraphs = result["description_paragraphs"].as_array().and_then(|paras| {
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
        if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        }
    });

    // Parse claims
    let claims = result["claims"].as_array().and_then(|claims_arr| {
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
        if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        }
    });

    // Parse images
    let images = result["images"].as_array().and_then(|imgs| {
        let parsed: Vec<crate::models::PatentImage> = imgs
            .iter()
            .filter_map(|img| {
                Some(crate::models::PatentImage {
                    url: img["url"].as_str()?.to_string(),
                    figure_number: img["figure_number"].as_str().map(String::from),
                })
            })
            .collect();
        if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        }
    });

    let filing_date = result["filing_date"].as_str().map(String::from);
    let assignee = result["assignee"].as_str().map(String::from);

    Ok(vec![Patent {
        id: patent_number.to_string(),
        title,
        abstract_text,
        description_paragraphs,
        claims,
        images,
        snippet: None,
        description: None,
        filing_date,
        assignee,
        url,
    }])
}

#[cfg(test)]
fn parse_search_results(results: serde_json::Value, limit: Option<usize>) -> Result<SearchResult> {
    let mut sr: SearchResult = serde_json::from_value(results)?;

    if let Some(limit) = limit {
        if sr.patents.len() > limit {
            sr.patents.truncate(limit);
        }
    }

    Ok(sr)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_search_results_limit() {
        let results = json!({
            "total_results": "1000",
            "patents": [
            {
                "id": "Unknown",
                "title": "Anomaly detection based on ensemble machine learning model",
                "snippet": "accessing the entity profile...",
                "filing_date": "2015-08-31",
                "assignee": "Unknown",
                "url": "https://patents.google.com/patent/Unknown"
            },
            {
                "id": "DE102018215057B4",
                "title": "Machine learning device, robot system and machine learning method",
                "snippet": "Machine learning method which is carried out...",
                "filing_date": "2017-09-12",
                "assignee": "Fanuc Corp",
                "url": "https://patents.google.com/patent/DE102018215057B4"
            },
            {
                "id": "US11694122B2",
                "title": "Distributed machine learning systems, apparatus, and methods",
                "snippet": "A distributed, online machine learning system...",
                "filing_date": "2016-07-18",
                "assignee": "Google LLC",
                "url": "https://patents.google.com/patent/US11694122B2"
            }
        ]});

        let sr = parse_search_results(results.clone(), Some(2)).unwrap();
        assert_eq!(sr.patents.len(), 2);
        assert_eq!(
            sr.patents[0].title,
            "Anomaly detection based on ensemble machine learning model"
        );
        assert_eq!(sr.patents[1].id, "DE102018215057B4");
        assert_eq!(sr.total_results, "1000");

        let sr = parse_search_results(results, None).unwrap();
        assert_eq!(sr.patents.len(), 3);
        assert_eq!(sr.patents[2].id, "US11694122B2");
    }

    #[test]
    fn test_parse_single_patent() {
        let result = json!({
            "title": "System and method for interactive big data analysis",
            "abstract": "A system and method for interactive big data analysis...",
            "filing_date": "2013-08-06",
            "assignee": "Google LLC",
            "description_paragraphs": [
                {"number": "0001", "id": "p1", "text": "CROSS-REFERENCE TO RELATED APPLICATIONS"}
            ],
            "claims": [
                {"number": "1", "id": "c1", "text": "1. A non-transitory machine-readable storage medium..."}
            ],
            "images": [
                {"url": "https://patentimages.storage.googleapis.com/.../US09152718-20151006-D00000.png", "figure_number": "D00000"}
            ]
        });

        let patents = parse_single_patent_result(
            result,
            "US9152718B2",
            "https://patents.google.com/patent/US9152718B2".to_string(),
        )
        .unwrap();
        assert_eq!(patents.len(), 1);
        let p = &patents[0];
        assert_eq!(p.id, "US9152718B2");
        assert_eq!(p.title, "System and method for interactive big data analysis");
        assert_eq!(
            p.abstract_text.as_deref(),
            Some("A system and method for interactive big data analysis...")
        );
        assert_eq!(p.filing_date.as_deref(), Some("2013-08-06"));
        assert_eq!(p.assignee.as_deref(), Some("Google LLC"));

        let paras = p.description_paragraphs.as_ref().unwrap();
        assert_eq!(paras.len(), 1);
        assert_eq!(paras[0].text, "CROSS-REFERENCE TO RELATED APPLICATIONS");

        let claims = p.claims.as_ref().unwrap();
        assert_eq!(claims.len(), 1);
        assert!(claims[0].text.starts_with("1. A non-transitory machine-readable storage medium"));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod integration_tests {
    use super::*;

    // Helper to create a searcher
    async fn create_searcher() -> PatentSearcher {
        // Use headless mode for tests
        PatentSearcher::new(None, true, false).await.expect("Failed to create PatentSearcher")
    }

    #[tokio::test]
    async fn test_real_search_query() {
        let searcher = create_searcher().await;
        let options = SearchOptions {
            query: Some("interactive big data analysis".to_string()),
            assignee: None,
            patent_number: None,
            limit: None,
            ..Default::default()
        };

        let results = searcher.search(&options).await.expect("Search failed");

        assert!(!results.patents.is_empty(), "Should return at least one result");
        assert_ne!(results.total_results, "Unknown", "Should return total results");
        // Verify the first result has some expected content
        let first = &results.patents[0];
        assert!(!first.title.is_empty(), "Title should not be empty");
        assert!(
            first.url.contains("patents.google.com"),
            "URL should contain google patents domain"
        );
    }

    #[tokio::test]
    async fn test_real_patent_lookup() {
        let searcher = create_searcher().await;
        let patent_id = "US9152718B2";
        let options = SearchOptions {
            query: None,
            assignee: None,
            patent_number: Some(patent_id.to_string()),
            limit: None,
            ..Default::default()
        };

        let results = searcher.search(&options).await.expect("Patent lookup failed");

        assert_eq!(results.patents.len(), 1, "Should return exactly one patent");
        let patent = &results.patents[0];
        assert_eq!(patent.id, patent_id, "Should return the requested patent ID");
        assert!(!patent.title.is_empty(), "Title should not be empty");
        assert!(patent.abstract_text.is_some(), "Should have abstract");
        assert!(patent.claims.is_some(), "Should have claims");
    }

    #[tokio::test]
    async fn test_real_search_limit() {
        let searcher = create_searcher().await;
        let limit = 2;
        let options = SearchOptions {
            query: Some("machine learning".to_string()),
            patent_number: None,
            limit: Some(limit),
            ..Default::default()
        };

        let results = searcher.search(&options).await.expect("Search with limit failed");

        assert_eq!(results.patents.len(), limit, "Should return exactly {} results", limit);
    }

    #[tokio::test]
    async fn test_real_raw_html() {
        let searcher = create_searcher().await;
        let patent_id = "US9152718B2";

        let html = searcher.get_raw_html(patent_id).await.expect("Failed to get raw HTML");

        assert!(!html.is_empty(), "HTML should not be empty");
        // Check for common HTML elements (DOCTYPE might be lowercase or missing)
        assert!(
            html.to_lowercase().contains("<html") || html.contains("<HTML"),
            "Should contain HTML tag"
        );
        // Check for patent-related content
        let html_lower = html.to_lowercase();
        assert!(
            html_lower.contains("patent") || html_lower.contains("interactive"),
            "Should contain patent-related content"
        );
    }

    #[tokio::test]
    async fn test_real_search_pagination() {
        let searcher = create_searcher().await;
        let limit = 25;
        let options = SearchOptions {
            query: Some("machine learning".to_string()),
            patent_number: None,
            limit: Some(limit),
            ..Default::default()
        };

        let results = searcher.search(&options).await.expect("Pagination search failed");

        assert_eq!(
            results.patents.len(),
            limit,
            "Should return exactly {} results via pagination",
            limit
        );

        // Verify results are unique (no duplicates from pagination)
        let ids: std::collections::HashSet<_> = results.patents.iter().map(|p| &p.id).collect();
        assert_eq!(ids.len(), results.patents.len(), "All results should have unique IDs");
    }
}
