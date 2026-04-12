use crate::core::models::{Patent, SearchOptions, SearchResult};
use crate::core::{BrowserManager, CdpPage};
use crate::core::{Error, Result};
use async_trait::async_trait;

// API response types for Google Patents /xhr/query endpoint
#[derive(serde::Deserialize)]
struct ApiResponse {
    results: ApiResults,
}

#[derive(serde::Deserialize)]
struct ApiResults {
    total_num_results: u64,
    cluster: Vec<ApiCluster>,
}

#[derive(serde::Deserialize)]
struct ApiCluster {
    result: Vec<ApiPatentEntry>,
}

#[derive(serde::Deserialize)]
struct ApiPatentEntry {
    patent: ApiPatent,
}

#[derive(serde::Deserialize)]
struct ApiPatent {
    title: Option<String>,
    snippet: Option<String>,
    filing_date: Option<String>,
    assignee: Option<String>,
    publication_number: Option<String>,
}

fn convert_api_response(api: ApiResponse) -> SearchResult {
    let patents = api
        .results
        .cluster
        .iter()
        .flat_map(|cluster| cluster.result.iter())
        .map(|entry| {
            let p = &entry.patent;
            let id = p.publication_number.clone().unwrap_or_default();
            Patent {
                id: id.clone(),
                title: p.title.clone().unwrap_or_default(),
                abstract_text: None,
                description_paragraphs: None,
                claims: None,
                images: None,
                snippet: p.snippet.clone(),
                description: None,
                filing_date: p.filing_date.clone(),
                assignee: p.assignee.clone(),
                related_application: None,
                claiming_priority: None,
                family_applications: None,
                legal_status: None,
                url: format!("https://patents.google.com/patent/{}", id),
            }
        })
        .collect();

    SearchResult {
        total_results: api.results.total_num_results.to_string(),
        top_assignees: None,
        top_cpcs: None,
        patents,
    }
}

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
            // Search results page - fetch via /xhr/query API
            let mut all_patents: Vec<Patent> = Vec::new();
            let limit = options.limit.unwrap_or(10);
            let mut total_results_str = "Unknown".to_string();
            let mut top_assignees: Option<Vec<crate::core::models::SummaryItem>> = None;
            let mut top_cpcs: Option<Vec<crate::core::models::SummaryItem>> = None;

            if self.verbose {
                eprintln!("Fetching search results (limit: {})...", limit);
            }

            // Append num=100 to base_url to fetch more results per page if needed
            let base_url = if limit > 10 { format!("{}&num=100", base_url) } else { base_url };

            // Calculate pagination
            let results_per_page = if limit > 10 { 100 } else { 10 };
            let pages_needed = limit.div_ceil(results_per_page);

            for page_num in 0..pages_needed {
                let page_url = if page_num == 0 {
                    base_url.clone()
                } else {
                    format!("{}&page={}", base_url, page_num)
                };

                if self.verbose {
                    eprintln!("Loading page {} of {}...", page_num + 1, pages_needed);
                    eprintln!("URL: {}", page_url);
                }

                page.goto(&page_url).await?;

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

                // Build API URL from the search URL
                let api_path =
                    base_url.strip_prefix("https://patents.google.com/").unwrap_or(&base_url);
                let api_url = format!("/xhr/query?url={}", api_path);
                let fetch_script = format!(
                    r#"(async () => {{
                        try {{
                            const resp = await fetch("{}");
                            if (!resp.ok) return {{ error: "HTTP " + resp.status }};
                            return await resp.json();
                        }} catch(e) {{
                            return {{ error: e.message }};
                        }}
                    }})()"#,
                    api_url
                );

                let api_result = page.evaluate(&fetch_script).await?;

                if self.verbose {
                    if let Some(err) = api_result.get("error") {
                        eprintln!("API error: {}", err);
                    } else {
                        eprintln!("API response received");
                    }
                }

                let sr = serde_json::from_value::<ApiResponse>(api_result)
                    .map_err(|e| Error::Search(format!("Failed to parse API response: {}", e)))
                    .map(convert_api_response)?;

                if page_num == 0 {
                    total_results_str = sr.total_results.clone();
                    if self.verbose {
                        eprintln!("Total results found: {}", total_results_str);
                    }
                    top_assignees = sr.top_assignees;
                    top_cpcs = sr.top_cpcs;
                }

                let page_patents = sr.patents;

                if self.verbose {
                    eprintln!("Found {} patents on this page", page_patents.len());
                }

                if page_patents.is_empty() {
                    break;
                }

                all_patents.extend(page_patents);

                if all_patents.len() >= limit {
                    break;
                }
            }
            let _ = page.close().await;

            if self.verbose {
                eprintln!("Total patents collected: {}", all_patents.len());
            }

            if all_patents.len() > limit {
                if self.verbose {
                    eprintln!("Truncating to limit: {}", limit);
                }
                all_patents.truncate(limit);
            }

            Ok(SearchResult {
                total_results: total_results_str,
                top_assignees,
                top_cpcs,
                patents: all_patents,
            })
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
