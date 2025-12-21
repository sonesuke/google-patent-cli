use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DescriptionParagraph {
    pub number: String,
    pub id: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claim {
    pub number: String,
    pub id: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatentImage {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub figure_number: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub url: String,
}

#[derive(Debug, Default)]
pub struct SearchOptions {
    pub query: Option<String>,
    pub assignee: Option<String>,
    pub country: Option<String>,
    pub patent_number: Option<String>,
    pub after_date: Option<String>,
    pub before_date: Option<String>,
    pub limit: Option<usize>,
}

impl SearchOptions {
    pub fn to_url(&self) -> anyhow::Result<String> {
        self.patent_number.as_ref().map_or_else(
            || {
                self.query.as_ref().map_or_else(
                    || Err(anyhow::anyhow!("Must provide either --query or --patent")),
                    |query| {
                        let mut url_str =
                            format!("https://patents.google.com/?q={}", query.replace(' ', "+"));

                        if let Some(assignee) = &self.assignee {
                            url_str.push_str(&format!("&assignee={}", assignee.replace(' ', "+")));
                        }

                        if let Some(country) = &self.country {
                            url_str.push_str(&format!("&country={}", country));
                        }

                        if let Some(after) = &self.after_date {
                            url_str.push_str(&format!("&after={}", after));
                        }

                        if let Some(before) = &self.before_date {
                            url_str.push_str(&format!("&before={}", before));
                        }

                        Ok(url_str)
                    },
                )
            },
            |patent_number| Ok(format!("https://patents.google.com/patent/{}", patent_number)),
        )
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
            assignee: None,
            country: None,
            patent_number: None,
            after_date: None,
            before_date: None,
            limit: Some(10),
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

        // Test query URL
        let options = SearchOptions { query: Some("foo bar".to_string()), ..Default::default() };
        assert_eq!(options.to_url().unwrap(), "https://patents.google.com/?q=foo+bar");

        // Test query with assignee
        let options = SearchOptions {
            query: Some("foo".to_string()),
            assignee: Some("Google LLC".to_string()),
            country: None,
            ..Default::default()
        };
        assert_eq!(
            options.to_url().unwrap(),
            "https://patents.google.com/?q=foo&assignee=Google+LLC"
        );

        // Test query with country
        let options = SearchOptions {
            query: Some("foo".to_string()),
            country: Some("JP".to_string()),
            ..Default::default()
        };
        assert_eq!(options.to_url().unwrap(), "https://patents.google.com/?q=foo&country=JP");

        // Test query with dates
        let options = SearchOptions {
            query: Some("foo".to_string()),
            after_date: Some("2020-01-01".to_string()),
            before_date: Some("2021-01-01".to_string()),
            ..Default::default()
        };
        let url = options.to_url().unwrap();
        assert!(url.contains("q=foo"));
        assert!(url.contains("after=2020-01-01"));
        assert!(url.contains("before=2021-01-01"));

        // Test error
        let options = SearchOptions::default();
        assert!(options.to_url().is_err());
    }
}
