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
    pub url: String,
}

#[derive(Debug)]
pub struct SearchOptions {
    pub query: Option<String>,
    pub patent_number: Option<String>,
    pub after_date: Option<String>,
    pub before_date: Option<String>,
    pub limit: Option<usize>,
}
