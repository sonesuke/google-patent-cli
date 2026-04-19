use crate::core::models::{Patent, SearchOptions, SearchResult};
use crate::core::{BrowserManager, CdpPage};
use crate::core::{Error, Result};
use async_trait::async_trait;

#[async_trait]
pub trait PatentSearch: Send + Sync {
    async fn search(&self, options: &SearchOptions) -> Result<SearchResult>;
}

pub struct PatentSearcher {
    browser_manager: BrowserManager,
    verbose: bool,
}

#[async_trait]
impl PatentSearch for PatentSearcher {
    /// Search for patents or fetch a specific patent
    async fn search(&self, options: &SearchOptions) -> Result<SearchResult> {
        self.search_internal(options).await
    }
}

impl PatentSearcher {
    pub async fn new(
        browser_path: Option<std::path::PathBuf>,
        headless: bool,
        debug: bool,
        verbose: bool,
        chrome_args: Vec<String>,
    ) -> Result<Self> {
        let browser_manager = BrowserManager::new(browser_path, headless, debug, chrome_args);

        Ok(Self { browser_manager, verbose })
    }

    async fn search_internal(&self, options: &SearchOptions) -> Result<SearchResult> {
        let browser = self.browser_manager.get_browser().await?;
        let page_ws_url = browser.new_page().await?;
        let page = CdpPage::new(&page_ws_url, std::time::Duration::from_secs(30)).await?;

        let base_url = options.to_url()?;

        if self.verbose {
            eprintln!("Search URL: {}", base_url);
        }

        if let Some(patent_number) = &options.patent_number {
            // Single patent lookup - no pagination needed
            if self.verbose {
                eprintln!("Fetching single patent: {}", patent_number);
            }
            page.goto(&base_url).await?;

            // Check for bot detection / rate limiting page
            let title = page
                .evaluate("document.title")
                .await
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            if title == "Sorry..." {
                let _ = page.close().await;
                return Err(Error::Search(
                    "Google blocked this request (bot detection / rate limiting). \
                     The IP address may be temporarily blocked. Try again later."
                        .to_string(),
                ));
            }

            if self.verbose {
                eprintln!("Waiting for page to load...");
            }
            // Wait for meta description or title tag to ensure page is loaded
            let loaded = page
                .wait_for_element("meta[name='description'], meta[name='DC.title']", 15)
                .await?;
            if !loaded {
                return Err(Error::Search("Page failed to load within timeout".to_string()));
            }
            // Additional wait for description paragraphs, claims, and images to appear
            let _ = page
                .wait_for_element(
                    "div.description-paragraph[num], div.claim[num], img[src*='patentimages']",
                    15,
                )
                .await?;

            // Give a little time for all dynamic content (like claims) to fully render
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

            if self.verbose {
                eprintln!("Extracting patent data...");
            }
            // Single patent page - extract structured data
            let result = page.evaluate(include_str!("scripts/extract_patent.js")).await?;

            // For single patent, total_results is "1".
            let patents = parse_single_patent_result(result, patent_number, base_url)?;
            let _ = page.close().await;

            Ok(SearchResult {
                total_results: "1".to_string(),
                top_assignees: None,
                top_cpcs: None,
                patents,
            })
        } else {
            // Search results page - scrape from DOM
            let limit = options.limit.unwrap_or(10);

            if self.verbose {
                eprintln!("Fetching search results (limit: {})...", limit);
            }

            page.goto(&base_url).await?;

            // Check for bot detection / rate limiting page
            let title = page
                .evaluate("document.title")
                .await
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            if title == "Sorry..." {
                let _ = page.close().await;
                return Err(Error::Search(
                    "Google blocked this request (bot detection / rate limiting). \
                     The IP address may be temporarily blocked. Try again later."
                        .to_string(),
                ));
            }

            if self.verbose {
                eprintln!("Waiting for search results to load...");
            }
            // Wait for search results to render
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            if self.verbose {
                eprintln!("Extracting search results from DOM...");
            }
            let result = page.evaluate(include_str!("scripts/extract_search_results.js")).await?;
            let mut sr: SearchResult = serde_json::from_value(result)
                .map_err(|e| Error::Search(format!("Failed to parse search results: {}", e)))?;

            let _ = page.close().await;

            if self.verbose {
                eprintln!("Total results found: {}", sr.total_results);
                eprintln!("Patents on page: {}", sr.patents.len());
            }

            if sr.patents.len() > limit {
                sr.patents.truncate(limit);
            }

            Ok(sr)
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
        let parsed: Vec<crate::core::models::DescriptionParagraph> = paras
            .iter()
            .filter_map(|p| {
                Some(crate::core::models::DescriptionParagraph {
                    number: p["number"].as_str()?.to_string(),
                    id: p["id"].as_str()?.to_string(),
                    text: p["text"].as_str()?.to_string(),
                })
            })
            .collect();
        if parsed.is_empty() { None } else { Some(parsed) }
    });

    // Parse claims
    let claims = result["claims"].as_array().and_then(|claims_arr| {
        let parsed: Vec<crate::core::models::Claim> = claims_arr
            .iter()
            .filter_map(|c| {
                Some(crate::core::models::Claim {
                    number: c["number"].as_str()?.to_string(),
                    id: c["id"].as_str()?.to_string(),
                    text: c["text"].as_str()?.to_string(),
                })
            })
            .collect();
        if parsed.is_empty() { None } else { Some(parsed) }
    });

    // Parse images
    let images = result["images"].as_array().and_then(|imgs| {
        let parsed: Vec<crate::core::models::PatentImage> = imgs
            .iter()
            .filter_map(|img| {
                Some(crate::core::models::PatentImage {
                    url: img["url"].as_str()?.to_string(),
                    figure_number: img["figure_number"].as_str().map(String::from),
                })
            })
            .collect();
        if parsed.is_empty() { None } else { Some(parsed) }
    });

    let filing_date = result["filing_date"].as_str().map(String::from);
    let assignee = result["assignee"].as_str().map(String::from);
    let related_application: Option<String> =
        result["related_application"].as_str().map(String::from);
    let claiming_priority: Option<Vec<crate::core::models::ApplicationInfo>> =
        serde_json::from_value(result["claiming_priority"].clone()).unwrap_or(None);
    let family_applications: Option<Vec<crate::core::models::ApplicationInfo>> =
        serde_json::from_value(result["family_applications"].clone()).unwrap_or(None);
    let legal_status = result["legal_status"].as_str().map(String::from);

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
        related_application,
        claiming_priority,
        family_applications,
        legal_status,
        url,
    }])
}

#[cfg(test)]
fn parse_search_results(results: serde_json::Value, limit: Option<usize>) -> Result<SearchResult> {
    let mut sr: SearchResult = serde_json::from_value(results)?;

    if let Some(limit) = limit
        && sr.patents.len() > limit
    {
        sr.patents.truncate(limit);
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
            "Patent": [
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
