use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DescriptionParagraph {
    pub number: String,
    pub id: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Claim {
    pub number: String,
    pub id: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PatentImage {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub figure_number: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SummaryItem {
    pub name: String,
    pub percentage: String,
}

#[derive(Debug, Serialize, Deserialize, Default, schemars::JsonSchema)]
pub struct SearchResult {
    pub total_results: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_assignees: Option<Vec<SummaryItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_cpcs: Option<Vec<SummaryItem>>,
    pub patents: Vec<Patent>,
}

#[derive(Debug, Serialize, Deserialize, Default, schemars::JsonSchema)]
pub struct Patent {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_paragraphs: Option<Vec<DescriptionParagraph>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claims: Option<Vec<Claim>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<PatentImage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filing_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_application: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claiming_priority: Option<Vec<ApplicationInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_applications: Option<Vec<ApplicationInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal_status: Option<String>,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ApplicationInfo {
    pub application_number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filing_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Default)]
pub struct SearchOptions {
    pub query: Option<String>,
    pub assignee: Option<Vec<String>>,
    pub country: Option<String>,
    pub patent_number: Option<String>,
    // Priority date filters (earliest filing date)
    pub priority_after: Option<String>,
    pub priority_before: Option<String>,
    // Publication date filters
    pub publication_after: Option<String>,
    pub publication_before: Option<String>,
    // Filing date filters
    pub filing_after: Option<String>,
    pub filing_before: Option<String>,
    pub limit: Option<usize>,
    /// Language/locale for the patent page (e.g., "ja", "en", "zh")
    pub language: Option<String>,
}

impl SearchOptions {
    pub fn to_url(&self) -> crate::core::Result<String> {
        if let Some(patent_number) = &self.patent_number {
            return self.language.as_ref().map_or_else(
                || Ok(format!("https://patents.google.com/patent/{}", patent_number)),
                |lang| {
                    Ok(format!("https://patents.google.com/patent/{}?hl={}", patent_number, lang))
                },
            );
        }

        let mut url = Url::parse("https://patents.google.com/")?;

        // Use a block to scope the mutable borrow of serializer
        {
            let mut serializer = url.query_pairs_mut();

            let mut q_parts = Vec::new();

            if let Some(query) = &self.query {
                q_parts.push(query.clone());
            }

            // Assignee is handled manually later to support comma separation

            if !q_parts.is_empty() {
                serializer.append_pair("q", &q_parts.join(" "));
            }

            if let Some(country) = &self.country {
                serializer.append_pair("country", country);
                match country.to_uppercase().as_str() {
                    "JP" => {
                        serializer.append_pair("language", "JAPANESE");
                    }
                    "CN" => {
                        serializer.append_pair("language", "CHINESE");
                    }
                    _ => {}
                }
            }

            // Priority date filters
            // Convert YYYY-MM-DD to YYYYMMDD format for Google Patents
            if let Some(after) = &self.priority_after {
                serializer.append_pair("after", &format!("priority:{}", after.replace("-", "")));
            }
            if let Some(before) = &self.priority_before {
                serializer.append_pair("before", &format!("priority:{}", before.replace("-", "")));
            }

            // Publication date filters
            if let Some(after) = &self.publication_after {
                serializer.append_pair("after", &format!("publication:{}", after.replace("-", "")));
            }
            if let Some(before) = &self.publication_before {
                serializer
                    .append_pair("before", &format!("publication:{}", before.replace("-", "")));
            }

            // Filing date filters
            if let Some(after) = &self.filing_after {
                serializer.append_pair("after", &format!("filing:{}", after.replace("-", "")));
            }
            if let Some(before) = &self.filing_before {
                serializer.append_pair("before", &format!("filing:{}", before.replace("-", "")));
            }

            if let Some(language) = &self.language {
                serializer.append_pair("hl", language);
            }
        }

        let mut url_str = url.to_string();

        // Manually append assignee parameter if present
        if let Some(assignees) = &self.assignee {
            if !assignees.is_empty() {
                let encoded_assignees: Vec<String> = assignees
                    .iter()
                    .map(|a| {
                        // Encode each assignee value, including quotes, using form_urlencoded logic
                        let quoted = format!("\"{}\"", a);
                        url::form_urlencoded::byte_serialize(quoted.as_bytes()).collect::<String>()
                    })
                    .collect();

                // Determine if we need to add '?' or '&'
                let separator = if !url_str.contains('?') {
                    "?"
                } else if url_str.ends_with('?') {
                    ""
                } else {
                    "&"
                };
                url_str.push_str(&format!("{}assignee={}", separator, encoded_assignees.join(",")));
            }
        }

        // Manual check for empty params (after constructing)
        // Check if url string ends with / or /? and has no params
        if !url_str.contains('?') || url_str.ends_with('?') {
            return Err(crate::core::Error::Search(
                "Must provide either --query, --assignee or --patent".to_string(),
            ));
        }

        Ok(url_str)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_patent_deserialization() {
        let json = r#"{
            "id": "US9152718B2",
            "title": "System and method for interactive big data analysis",
            "url": "https://patents.google.com/patent/US9152718B2"
        }"#;

        let patent: Patent = serde_json::from_str(json).unwrap();
        assert_eq!(patent.id, "US9152718B2");
        assert_eq!(patent.title, "System and method for interactive big data analysis");
        assert_eq!(patent.url, "https://patents.google.com/patent/US9152718B2");
        assert!(patent.abstract_text.is_none());
    }

    #[test]
    fn test_search_options_creation() {
        let options = SearchOptions {
            query: Some("test".to_string()),
            limit: Some(10),
            ..Default::default()
        };

        assert_eq!(options.query.as_deref(), Some("test"));
        assert_eq!(options.limit, Some(10));
    }

    #[test]
    fn test_search_options_to_url() {
        // Test patent number URL
        let options =
            SearchOptions { patent_number: Some("US9152718B2".to_string()), ..Default::default() };
        assert_eq!(options.to_url().unwrap(), "https://patents.google.com/patent/US9152718B2");

        let options = SearchOptions { query: Some("foo bar".to_string()), ..Default::default() };
        // q=foo+bar
        assert_eq!(options.to_url().unwrap(), "https://patents.google.com/?q=foo+bar");

        // Test assignee only (single assignee)
        let options =
            SearchOptions { assignee: Some(vec!["Google LLC".to_string()]), ..Default::default() };
        // assignee="Google LLC" -> encoded %22Google%20LLC%22
        let url = options.to_url().unwrap();

        // Since no other params, it should start with ?assignee=
        // form_urlencoded::byte_serialize uses + for spaces in query values
        assert!(url.contains("?assignee=%22Google+LLC%22"));

        // Test assignee (multiple assignees)
        let options = SearchOptions {
            assignee: Some(vec!["Google LLC".to_string(), "Microsoft Corp".to_string()]),
            ..Default::default()
        };
        // assignee="Google LLC","Microsoft Corp"
        // Encoded individual values, joined by comma
        let url = options.to_url().unwrap();
        assert!(url.contains("?assignee=%22Google+LLC%22,%22Microsoft+Corp%22"));

        // Test assignee (comma handling)
        let options = SearchOptions {
            assignee: Some(vec!["Salesforce.com, inc.".to_string()]),
            ..Default::default()
        };
        let url = options.to_url().unwrap();
        // assignee="Salesforce.com, inc."
        // comma inside quotes encoded as %2C. space as %20.
        // %22Salesforce.com%2C%20inc.%22
        assert!(url.contains("?assignee=%22Salesforce.com%2C+inc.%22"));

        // Test query with assignee
        let options = SearchOptions {
            query: Some("foo".to_string()),
            assignee: Some(vec!["Google LLC".to_string()]),
            country: None,
            ..Default::default()
        };
        // q=foo&assignee="Google LLC"
        // q is added via serializer (foo). assignee appended manually (&assignee=...)
        let url = options.to_url().unwrap();
        assert!(url.contains("q=foo"));
        assert!(url.contains("&assignee=%22Google+LLC%22"));

        // Test query with country (JP should add language=JAPANESE)
        let options = SearchOptions {
            query: Some("foo".to_string()),
            country: Some("JP".to_string()),
            ..Default::default()
        };
        assert_eq!(
            options.to_url().unwrap(),
            "https://patents.google.com/?q=foo&country=JP&language=JAPANESE"
        );

        // Test query with country (CN should add language=CHINESE)
        let options = SearchOptions {
            query: Some("foo".to_string()),
            country: Some("CN".to_string()),
            ..Default::default()
        };
        assert_eq!(
            options.to_url().unwrap(),
            "https://patents.google.com/?q=foo&country=CN&language=CHINESE"
        );

        // Test query with country (US should NOT add language)
        let options = SearchOptions {
            query: Some("foo".to_string()),
            country: Some("US".to_string()),
            ..Default::default()
        };
        assert_eq!(options.to_url().unwrap(), "https://patents.google.com/?q=foo&country=US");

        // Test query with priority dates
        let options = SearchOptions {
            query: Some("foo".to_string()),
            priority_after: Some("2020-01-01".to_string()),
            priority_before: Some("2021-01-01".to_string()),
            ..Default::default()
        };
        let url = options.to_url().unwrap();
        assert!(url.contains("q=foo"));
        // Date is converted from YYYY-MM-DD to YYYYMMDD format
        assert!(url.contains("after=priority%3A20200101"));
        assert!(url.contains("before=priority%3A20210101"));

        // Test error
        let options = SearchOptions::default();
        assert!(options.to_url().is_err());

        // Test patent with language (hl parameter)
        let options = SearchOptions {
            patent_number: Some("US9152718B2".to_string()),
            language: Some("ja".to_string()),
            ..Default::default()
        };
        assert_eq!(
            options.to_url().unwrap(),
            "https://patents.google.com/patent/US9152718B2?hl=ja"
        );

        // Test search with language
        let options = SearchOptions {
            query: Some("foo".to_string()),
            language: Some("ja".to_string()),
            ..Default::default()
        };
        assert_eq!(options.to_url().unwrap(), "https://patents.google.com/?q=foo&hl=ja");

        // Test search with country and explicit language override
        let options = SearchOptions {
            query: Some("foo".to_string()),
            country: Some("JP".to_string()),
            language: Some("en".to_string()),
            ..Default::default()
        };
        let url = options.to_url().unwrap();
        assert!(url.contains("country=JP"));
        assert!(url.contains("language=JAPANESE"));
        assert!(url.contains("hl=en"));
    }
}
