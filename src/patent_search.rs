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

        let url = options.to_url()?;

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
                .evaluate(include_str!("scripts/extract_patent.js"))
                .await?;

            parse_single_patent_result(result, patent_number, url)
        } else {

            // Search results page - use JavaScript evaluation
            
            // Wait for results to load
            // We wait for the PDF link class which is specific to results
            let loaded = page.wait_for_element(".search-result-item", 15).await?;
            if !loaded {
                // If it times out, it might be because there are no results, or network issues.
            }

            let results = page
                .evaluate(include_str!("scripts/extract_search_results.js"))
                .await?;

            parse_search_results(results, options.limit)
        }
    }
}

fn parse_single_patent_result(result: serde_json::Value, patent_number: &str, url: String) -> Result<Vec<Patent>> {
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
        id: patent_number.to_string(),
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
}

fn parse_search_results(results: serde_json::Value, limit: Option<usize>) -> Result<Vec<Patent>> {
    let mut patents: Vec<Patent> = serde_json::from_value(results)?;
    
    if let Some(limit) = limit {
        if patents.len() > limit {
            patents.truncate(limit);
        }
    }
    
    Ok(patents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_search_results_limit() {
        let results = json!([
            {
                "id": "Unknown",
                "title": "Anomaly detection based on ensemble machine learning model",
                "snippet": "accessing the entity profile...",
                "filing_date": "2015-08-31",
                "url": "https://patents.google.com/patent/Unknown"
            },
            {
                "id": "DE102018215057B4",
                "title": "Machine learning device, robot system and machine learning method",
                "snippet": "Machine learning method which is carried out...",
                "filing_date": "2017-09-12",
                "url": "https://patents.google.com/patent/DE102018215057B4"
            },
            {
                "id": "US11694122B2",
                "title": "Distributed machine learning systems, apparatus, and methods",
                "snippet": "A distributed, online machine learning system...",
                "filing_date": "2016-07-18",
                "url": "https://patents.google.com/patent/US11694122B2"
            }
        ]);

        let patents = parse_search_results(results.clone(), Some(2)).unwrap();
        assert_eq!(patents.len(), 2);
        assert_eq!(patents[0].title, "Anomaly detection based on ensemble machine learning model");
        assert_eq!(patents[1].id, "DE102018215057B4");

        let patents = parse_search_results(results, None).unwrap();
        assert_eq!(patents.len(), 3);
        assert_eq!(patents[2].id, "US11694122B2");
    }

    #[test]
    fn test_parse_single_patent() {
        let result = json!({
            "title": "System and method for interactive big data analysis",
            "abstract": "A system and method for interactive big data analysis...",
            "filing_date": "2013-08-06",
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

        let patents = parse_single_patent_result(result, "US9152718B2", "https://patents.google.com/patent/US9152718B2".to_string()).unwrap();
        assert_eq!(patents.len(), 1);
        let p = &patents[0];
        assert_eq!(p.id, "US9152718B2");
        assert_eq!(p.title, "System and method for interactive big data analysis");
        assert_eq!(p.abstract_text.as_deref(), Some("A system and method for interactive big data analysis..."));
        assert_eq!(p.filing_date.as_deref(), Some("2013-08-06"));
        
        let paras = p.description_paragraphs.as_ref().unwrap();
        assert_eq!(paras.len(), 1);
        assert_eq!(paras[0].text, "CROSS-REFERENCE TO RELATED APPLICATIONS");

        let claims = p.claims.as_ref().unwrap();
        assert_eq!(claims.len(), 1);
        assert!(claims[0].text.starts_with("1. A non-transitory machine-readable storage medium"));
    }
}
