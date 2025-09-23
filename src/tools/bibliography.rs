use crate::{Config, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument};

/// Input parameters for the bibliography tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BibliographyInput {
    /// List of DOIs or paper identifiers
    pub identifiers: Vec<String>,

    /// Citation format (bibtex, apa, mla, chicago, ieee)
    #[serde(default = "default_format")]
    pub format: CitationFormat,

    /// Include abstract in citation
    #[serde(default)]
    pub include_abstract: bool,

    /// Include keywords in citation
    #[serde(default)]
    pub include_keywords: bool,
}

const fn default_format() -> CitationFormat {
    CitationFormat::BibTeX
}

/// Citation format types
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CitationFormat {
    #[default]
    BibTeX,
    APA,
    MLA,
    Chicago,
    IEEE,
    Harvard,
}

/// Result of bibliography generation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BibliographyResult {
    /// Generated citations
    pub citations: Vec<Citation>,

    /// Combined bibliography text
    pub bibliography: String,

    /// Format used
    pub format: CitationFormat,

    /// Errors encountered for specific papers
    pub errors: Vec<CitationError>,
}

/// Individual citation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Citation {
    /// Paper identifier (DOI or URL)
    pub identifier: String,

    /// Citation text
    pub text: String,

    /// Citation key (for BibTeX)
    pub key: Option<String>,

    /// Paper metadata
    pub metadata: PaperMetadata,
}

/// Citation error
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CitationError {
    /// Paper identifier that failed
    pub identifier: String,

    /// Error message
    pub message: String,
}

/// Paper metadata for citations
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PaperMetadata {
    pub title: String,
    pub authors: Vec<String>,
    pub year: Option<i32>,
    pub journal: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
    pub publication_date: Option<String>,
}

/// Bibliography generation tool
#[derive(Debug, Clone)]
pub struct BibliographyTool {
    _config: Arc<Config>,
}

impl BibliographyTool {
    /// Create a new bibliography tool
    pub const fn new(config: Arc<Config>) -> Result<Self> {
        Ok(Self { _config: config })
    }

    /// Generate bibliography from paper identifiers
    #[instrument(skip(self))]
    pub async fn generate(&self, input: BibliographyInput) -> Result<BibliographyResult> {
        info!(
            "Generating bibliography for {} papers in {:?} format",
            input.identifiers.len(),
            input.format
        );

        let mut citations = Vec::new();
        let mut errors = Vec::new();

        for identifier in &input.identifiers {
            match self.fetch_metadata(identifier) {
                Ok(metadata) => {
                    let citation = self.format_citation(
                        &metadata,
                        identifier,
                        &input.format,
                        input.include_abstract,
                        input.include_keywords,
                    );
                    citations.push(citation);
                }
                Err(e) => {
                    errors.push(CitationError {
                        identifier: identifier.clone(),
                        message: e.to_string(),
                    });
                }
            }
        }

        // Generate combined bibliography
        let bibliography = self.combine_citations(&citations, &input.format);

        Ok(BibliographyResult {
            citations,
            bibliography,
            format: input.format.clone(),
            errors,
        })
    }

    /// Fetch metadata for a paper
    fn fetch_metadata(&self, identifier: &str) -> Result<PaperMetadata> {
        // In a real implementation, this would query CrossRef, Semantic Scholar, etc.
        // For now, we'll create mock metadata
        info!("Fetching metadata for: {}", identifier);

        // Mock implementation - replace with actual API calls
        Ok(PaperMetadata {
            title: format!("Paper Title for {identifier}"),
            authors: vec!["Smith, J.".to_string(), "Doe, A.".to_string()],
            year: Some(2024),
            journal: Some("Journal of Computer Science".to_string()),
            volume: Some("42".to_string()),
            issue: Some("3".to_string()),
            pages: Some("123-145".to_string()),
            doi: Some(identifier.to_string()),
            url: Some(format!("https://doi.org/{identifier}")),
            abstract_text: Some("This paper presents...".to_string()),
            keywords: vec!["machine learning".to_string(), "algorithms".to_string()],
            publication_date: Some("2024-01-15".to_string()),
        })
    }

    /// Format a citation based on the selected style
    fn format_citation(
        &self,
        metadata: &PaperMetadata,
        identifier: &str,
        format: &CitationFormat,
        include_abstract: bool,
        include_keywords: bool,
    ) -> Citation {
        let text = match format {
            CitationFormat::BibTeX => {
                self.format_bibtex(metadata, include_abstract, include_keywords)
            }
            CitationFormat::APA => self.format_apa(metadata),
            CitationFormat::MLA => self.format_mla(metadata),
            CitationFormat::Chicago => self.format_chicago(metadata),
            CitationFormat::IEEE => self.format_ieee(metadata),
            CitationFormat::Harvard => self.format_harvard(metadata),
        };

        let key = match format {
            CitationFormat::BibTeX => Some(self.generate_bibtex_key(metadata)),
            _ => None,
        };

        Citation {
            identifier: identifier.to_string(),
            text,
            key,
            metadata: metadata.clone(),
        }
    }

    /// Format as BibTeX
    fn format_bibtex(
        &self,
        metadata: &PaperMetadata,
        include_abstract: bool,
        include_keywords: bool,
    ) -> String {
        let key = self.generate_bibtex_key(metadata);
        let mut parts = vec![
            format!("@article{{{},", key),
            format!("  title = {{{}}},", metadata.title),
            format!("  author = {{{}}},", metadata.authors.join(" and ")),
        ];

        if let Some(year) = metadata.year {
            parts.push(format!("  year = {{{year}}},"));
        }

        if let Some(ref journal) = metadata.journal {
            parts.push(format!("  journal = {{{journal}}},"));
        }

        if let Some(ref volume) = metadata.volume {
            parts.push(format!("  volume = {{{volume}}},"));
        }

        if let Some(ref issue) = metadata.issue {
            parts.push(format!("  number = {{{issue}}},"));
        }

        if let Some(ref pages) = metadata.pages {
            parts.push(format!("  pages = {{{pages}}},"));
        }

        if let Some(ref doi) = metadata.doi {
            parts.push(format!("  doi = {{{doi}}},"));
        }

        if let Some(ref url) = metadata.url {
            parts.push(format!("  url = {{{url}}},"));
        }

        if include_abstract {
            if let Some(ref abstract_text) = metadata.abstract_text {
                parts.push(format!("  abstract = {{{abstract_text}}},"));
            }
        }

        if include_keywords && !metadata.keywords.is_empty() {
            parts.push(format!(
                "  keywords = {{{}}},",
                metadata.keywords.join(", ")
            ));
        }

        // Remove trailing comma from last entry
        if let Some(last) = parts.last_mut() {
            if last.ends_with(',') {
                last.pop();
            }
        }

        parts.push("}".to_string());
        parts.join("\n")
    }

    /// Generate BibTeX key
    fn generate_bibtex_key(&self, metadata: &PaperMetadata) -> String {
        let first_author = metadata
            .authors
            .first()
            .and_then(|a| a.split(',').next())
            .unwrap_or("Unknown");

        let year = metadata
            .year
            .map_or_else(|| "0000".to_string(), |y| y.to_string());

        let title_word = metadata.title.split_whitespace().next().unwrap_or("Paper");

        format!(
            "{}{}{}",
            first_author.replace(' ', ""),
            year,
            title_word.chars().take(4).collect::<String>()
        )
    }

    /// Format as APA
    fn format_apa(&self, metadata: &PaperMetadata) -> String {
        let authors = self.format_authors_apa(&metadata.authors);
        let year = metadata
            .year
            .map_or_else(|| "(n.d.)".to_string(), |y| format!("({y})"));

        let mut citation = format!("{}. {}. {}.", authors, year, metadata.title);

        if let Some(ref journal) = metadata.journal {
            citation.push_str(&format!(" {journal}"));

            if let Some(ref volume) = metadata.volume {
                citation.push_str(&format!(", {volume}"));

                if let Some(ref issue) = metadata.issue {
                    citation.push_str(&format!("({issue})"));
                }
            }

            if let Some(ref pages) = metadata.pages {
                citation.push_str(&format!(", {pages}"));
            }
        }

        if let Some(ref doi) = metadata.doi {
            citation.push_str(&format!(". https://doi.org/{doi}"));
        }

        citation
    }

    /// Format authors for APA style
    fn format_authors_apa(&self, authors: &[String]) -> String {
        match authors.len() {
            0 => "Unknown".to_string(),
            1 => authors[0].clone(),
            2 => format!("{}, & {}", authors[0], authors[1]),
            _ => {
                let first_authors = &authors[..authors.len() - 1];
                let last_author = &authors[authors.len() - 1];
                format!("{}, & {}", first_authors.join(", "), last_author)
            }
        }
    }

    /// Format as MLA
    fn format_mla(&self, metadata: &PaperMetadata) -> String {
        let authors = if metadata.authors.len() > 1 {
            format!("{}, et al", metadata.authors[0])
        } else {
            metadata
                .authors
                .first()
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string())
        };

        let mut citation = format!("{}. \"{}\"", authors, metadata.title);

        if let Some(ref journal) = metadata.journal {
            citation.push_str(&format!(". {journal}"));

            if let Some(ref volume) = metadata.volume {
                citation.push_str(&format!(", vol. {volume}"));
            }

            if let Some(ref issue) = metadata.issue {
                citation.push_str(&format!(", no. {issue}"));
            }
        }

        if let Some(year) = metadata.year {
            citation.push_str(&format!(", {year}"));
        }

        if let Some(ref pages) = metadata.pages {
            citation.push_str(&format!(", pp. {pages}"));
        }

        citation.push('.');
        citation
    }

    /// Format as Chicago
    fn format_chicago(&self, metadata: &PaperMetadata) -> String {
        let authors = metadata.authors.join(", ");
        let mut citation = format!("{}. \"{}\"", authors, metadata.title);

        if let Some(ref journal) = metadata.journal {
            citation.push_str(&format!(". {journal}"));

            if let Some(ref volume) = metadata.volume {
                citation.push_str(&format!(" {volume}"));
            }

            if let Some(ref issue) = metadata.issue {
                citation.push_str(&format!(", no. {issue}"));
            }
        }

        if let Some(year) = metadata.year {
            citation.push_str(&format!(" ({year})"));
        }

        if let Some(ref pages) = metadata.pages {
            citation.push_str(&format!(": {pages}"));
        }

        citation.push('.');
        citation
    }

    /// Format as IEEE
    fn format_ieee(&self, metadata: &PaperMetadata) -> String {
        let authors = metadata
            .authors
            .iter()
            .map(|a| {
                let parts: Vec<&str> = a.split(',').collect();
                if parts.len() >= 2 {
                    format!(
                        "{}. {}",
                        parts[1].trim().chars().next().unwrap_or('?'),
                        parts[0].trim()
                    )
                } else {
                    a.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let mut citation = format!("{}, \"{}\"", authors, metadata.title);

        if let Some(ref journal) = metadata.journal {
            citation.push_str(&format!(", {journal}"));

            if let Some(ref volume) = metadata.volume {
                citation.push_str(&format!(", vol. {volume}"));
            }

            if let Some(ref issue) = metadata.issue {
                citation.push_str(&format!(", no. {issue}"));
            }

            if let Some(ref pages) = metadata.pages {
                citation.push_str(&format!(", pp. {pages}"));
            }
        }

        if let Some(year) = metadata.year {
            citation.push_str(&format!(", {year}"));
        }

        citation.push('.');
        citation
    }

    /// Format as Harvard
    fn format_harvard(&self, metadata: &PaperMetadata) -> String {
        let authors = metadata.authors.join(", ");
        let year = metadata
            .year
            .map_or_else(|| "n.d.".to_string(), |y| y.to_string());

        let mut citation = format!("{} {}, '{}'", authors, year, metadata.title);

        if let Some(ref journal) = metadata.journal {
            citation.push_str(&format!(", {journal}"));

            if let Some(ref volume) = metadata.volume {
                citation.push_str(&format!(", vol. {volume}"));
            }

            if let Some(ref issue) = metadata.issue {
                citation.push_str(&format!(", no. {issue}"));
            }

            if let Some(ref pages) = metadata.pages {
                citation.push_str(&format!(", pp. {pages}"));
            }
        }

        citation.push('.');
        citation
    }

    /// Combine citations into a bibliography
    fn combine_citations(&self, citations: &[Citation], format: &CitationFormat) -> String {
        if matches!(format, CitationFormat::BibTeX) {
            citations
                .iter()
                .map(|c| c.text.clone())
                .collect::<Vec<_>>()
                .join("\n\n")
        } else {
            // For other formats, sort alphabetically and number
            let mut sorted_citations = citations.to_vec();
            sorted_citations.sort_by(|a, b| {
                a.metadata
                    .authors
                    .first()
                    .unwrap_or(&String::new())
                    .cmp(b.metadata.authors.first().unwrap_or(&String::new()))
            });

            sorted_citations
                .iter()
                .enumerate()
                .map(|(i, c)| format!("[{}] {}", i + 1, c.text))
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bibtex_key_generation() {
        let config = Arc::new(Config::default());
        let tool = BibliographyTool::new(config).unwrap();

        let metadata = PaperMetadata {
            title: "Test Paper".to_string(),
            authors: vec!["Smith, John".to_string()],
            year: Some(2024),
            journal: None,
            volume: None,
            issue: None,
            pages: None,
            doi: None,
            url: None,
            abstract_text: None,
            keywords: vec![],
            publication_date: None,
        };

        let key = tool.generate_bibtex_key(&metadata);
        assert_eq!(key, "Smith2024Test");
    }
}
