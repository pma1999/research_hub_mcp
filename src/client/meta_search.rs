use crate::client::providers::{
    ArxivProvider, BiorxivProvider, CoreProvider, CrossRefProvider, MdpiProvider, OpenReviewProvider, ProviderError, ProviderResult, 
    PubMedCentralProvider, ResearchGateProvider, SciHubProvider, SemanticScholarProvider, SsrnProvider, 
    SearchContext, SearchQuery, SearchType, SourceProvider, UnpaywallProvider,
};
use crate::client::PaperMetadata;
use crate::Config;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Configuration for meta-search behavior
#[derive(Debug, Clone)]
pub struct MetaSearchConfig {
    /// Maximum number of providers to query in parallel
    pub max_parallel_providers: usize,
    /// Timeout for each provider
    pub provider_timeout: Duration,
    /// Whether to continue searching if some providers fail
    pub continue_on_failure: bool,
    /// Whether to deduplicate results
    pub deduplicate_results: bool,
    /// Minimum relevance score to include results
    pub min_relevance_score: f64,
}

impl Default for MetaSearchConfig {
    fn default() -> Self {
        Self {
            max_parallel_providers: 3,
            provider_timeout: Duration::from_secs(30),
            continue_on_failure: true,
            deduplicate_results: true,
            min_relevance_score: 0.0,
        }
    }
}

/// Result from meta-search across multiple providers
#[derive(Debug, Clone)]
pub struct MetaSearchResult {
    /// All papers found across providers
    pub papers: Vec<PaperMetadata>,
    /// Results grouped by source
    pub by_source: HashMap<String, Vec<PaperMetadata>>,
    /// Total search time
    pub total_search_time: Duration,
    /// Number of providers that succeeded
    pub successful_providers: usize,
    /// Number of providers that failed
    pub failed_providers: usize,
    /// Errors from failed providers
    pub provider_errors: HashMap<String, String>,
    /// Metadata from all providers
    pub provider_metadata: HashMap<String, HashMap<String, String>>,
}

/// Client that performs meta-search across multiple academic sources
pub struct MetaSearchClient {
    providers: Vec<Arc<dyn SourceProvider>>,
    config: MetaSearchConfig,
    rate_limiters: Arc<RwLock<HashMap<String, Instant>>>,
}

impl MetaSearchClient {
    /// Create a new meta-search client
    pub fn new(_app_config: Config, meta_config: MetaSearchConfig) -> Result<Self, ProviderError> {
        let mut providers: Vec<Arc<dyn SourceProvider>> = Vec::new();

        // Add CrossRef provider (highest priority for authoritative metadata)
        providers.push(Arc::new(CrossRefProvider::new(None)?)); // TODO: Get email from config

        // Add Semantic Scholar provider (very high priority for PDF access + metadata)
        providers.push(Arc::new(SemanticScholarProvider::new(None)?)); // TODO: Get API key from config

        // Add Unpaywall provider (high priority for legal free PDF discovery)
        providers.push(Arc::new(UnpaywallProvider::new_with_default_email()?)); // TODO: Get email from config

        // Add PubMed Central provider (very high priority for biomedical papers)
        providers.push(Arc::new(PubMedCentralProvider::new(None)?)); // TODO: Get API key from config

        // Add CORE provider (high priority for open access collection)
        providers.push(Arc::new(CoreProvider::new(None)?)); // TODO: Get API key from config

        // Add SSRN provider (high priority for recent papers and preprints)
        providers.push(Arc::new(SsrnProvider::new()?));

        // Add arXiv provider (high priority for CS/physics/math)
        providers.push(Arc::new(ArxivProvider::new()?));

        // Add bioRxiv provider (biology preprints)
        providers.push(Arc::new(BiorxivProvider::new()?));

        // Add OpenReview provider (high priority for ML conference papers)
        providers.push(Arc::new(OpenReviewProvider::new()?));

        // Add MDPI provider (good priority for open access journals)
        providers.push(Arc::new(MdpiProvider::new()?));

        // Add ResearchGate provider (lower priority due to access limitations)
        providers.push(Arc::new(ResearchGateProvider::new()?));

        // Add Sci-Hub provider (lowest priority, for full-text access)
        providers.push(Arc::new(SciHubProvider::new()?));

        info!(
            "Initialized meta-search client with {} providers",
            providers.len()
        );

        Ok(Self {
            providers,
            config: meta_config,
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get list of available providers
    pub fn providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|p| p.name().to_string())
            .collect()
    }

    /// Perform health checks on all providers
    pub async fn health_check(&self) -> HashMap<String, bool> {
        let context = self.create_search_context().await;
        let mut results = HashMap::new();

        for provider in &self.providers {
            let health = provider.health_check(&context).await.unwrap_or(false);
            results.insert(provider.name().to_string(), health);

            if health {
                info!("Provider {} is healthy", provider.name());
            } else {
                warn!("Provider {} is unhealthy", provider.name());
            }
        }

        results
    }

    /// Search across multiple providers
    pub async fn search(&self, query: &SearchQuery) -> Result<MetaSearchResult, ProviderError> {
        let start_time = Instant::now();
        info!(
            "Starting meta-search for: {} (type: {:?})",
            query.query, query.search_type
        );

        // Create search context
        let context = self.create_search_context().await;

        // Filter providers based on query type and supported features
        let suitable_providers = self.filter_providers_for_query(query).await;
        info!(
            "Using {} providers for search: {:?}",
            suitable_providers.len(),
            suitable_providers
                .iter()
                .map(|p| p.name())
                .collect::<Vec<_>>()
        );

        // Search providers in parallel (with concurrency limit)
        let mut provider_results = Vec::new();
        let mut provider_errors = HashMap::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_parallel_providers,
        ));

        let mut tasks = Vec::new();
        for provider in suitable_providers {
            let provider = provider.clone();
            let query = query.clone();
            let context = context.clone();
            let semaphore = semaphore.clone();
            let timeout_duration = self.config.provider_timeout;

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();

                // Apply rate limiting
                if let Err(e) = Self::apply_rate_limit(&provider).await {
                    return (provider.name().to_string(), Err(e));
                }

                // Search with timeout
                let result = timeout(timeout_duration, provider.search(&query, &context)).await;

                match result {
                    Ok(Ok(provider_result)) => (provider.name().to_string(), Ok(provider_result)),
                    Ok(Err(e)) => (provider.name().to_string(), Err(e)),
                    Err(_) => (provider.name().to_string(), Err(ProviderError::Timeout)),
                }
            });

            tasks.push(task);
        }

        // Collect results
        for task in tasks {
            match task.await {
                Ok((provider_name, Ok(result))) => {
                    info!(
                        "Provider {} returned {} results",
                        provider_name,
                        result.papers.len()
                    );
                    provider_results.push((provider_name, result));
                }
                Ok((provider_name, Err(error))) => {
                    warn!("Provider {} failed: {}", provider_name, error);
                    provider_errors.insert(provider_name, error.to_string());
                }
                Err(e) => {
                    error!("Task failed: {}", e);
                }
            }
        }

        // Aggregate results
        let meta_result = self
            .aggregate_results(provider_results, provider_errors, start_time)
            .await;

        info!(
            "Meta-search completed: {} total papers from {} providers in {:?}",
            meta_result.papers.len(),
            meta_result.successful_providers,
            meta_result.total_search_time
        );

        Ok(meta_result)
    }

    /// Search for a paper by DOI across providers that support it
    pub async fn get_by_doi(&self, doi: &str) -> Result<Option<PaperMetadata>, ProviderError> {
        info!("Searching for DOI: {}", doi);

        let context = self.create_search_context().await;

        // Try providers in priority order
        let mut providers: Vec<_> = self.providers.iter().collect();
        providers.sort_by_key(|p| std::cmp::Reverse(p.priority()));

        for provider in providers {
            if provider.supported_search_types().contains(&SearchType::Doi) {
                // Apply rate limiting
                if let Err(e) = Self::apply_rate_limit(provider).await {
                    warn!("Rate limit hit for {}: {}", provider.name(), e);
                    continue;
                }

                match provider.get_by_doi(doi, &context).await {
                    Ok(Some(paper)) => {
                        info!("Found paper for DOI {} from {}", doi, provider.name());
                        return Ok(Some(paper));
                    }
                    Ok(None) => {
                        debug!("DOI {} not found in {}", doi, provider.name());
                    }
                    Err(e) => {
                        warn!("Error searching {} for DOI {}: {}", provider.name(), doi, e);
                    }
                }
            }
        }

        info!("DOI {} not found in any provider", doi);
        Ok(None)
    }

    /// Create search context with common settings
    async fn create_search_context(&self) -> SearchContext {
        SearchContext {
            timeout: self.config.provider_timeout,
            user_agent: "rust-research-mcp/0.2.1 (Academic Research Tool)".to_string(),
            rate_limit: Some(Duration::from_millis(1000)),
            headers: HashMap::new(),
        }
    }

    /// Filter providers based on query characteristics
    async fn filter_providers_for_query(
        &self,
        query: &SearchQuery,
    ) -> Vec<Arc<dyn SourceProvider>> {
        let mut suitable = Vec::new();

        for provider in &self.providers {
            // Check if provider supports the search type
            if provider
                .supported_search_types()
                .contains(&query.search_type)
                || provider
                    .supported_search_types()
                    .contains(&SearchType::Auto)
            {
                suitable.push(provider.clone());
            }
        }

        // Apply intelligent priority ordering based on query characteristics
        self.apply_intelligent_priority_ordering(&mut suitable, query).await;

        suitable
    }

    /// Apply intelligent priority ordering based on query characteristics
    async fn apply_intelligent_priority_ordering(
        &self,
        providers: &mut Vec<Arc<dyn SourceProvider>>,
        query: &SearchQuery,
    ) {
        let query_lower = query.query.to_lowercase();
        
        // Create priority adjustments based on query analysis
        let mut provider_scores: Vec<(Arc<dyn SourceProvider>, i32)> = providers
            .iter()
            .map(|provider| {
                let base_priority = provider.priority() as i32;
                let mut adjusted_priority = base_priority;
                
                // Domain-specific priority adjustments
                adjusted_priority += self.calculate_domain_priority_boost(provider, &query_lower);
                
                // Search type priority adjustments
                adjusted_priority += self.calculate_search_type_priority_boost(provider, query);
                
                // Content availability priority adjustments
                adjusted_priority += self.calculate_content_priority_boost(provider, &query_lower);
                
                // Time-sensitive priority adjustments
                adjusted_priority += self.calculate_temporal_priority_boost(provider, &query_lower);
                
                (provider.clone(), adjusted_priority)
            })
            .collect();
        
        // Sort by adjusted priority (highest first)
        provider_scores.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
        
        // Update the providers vector with the new ordering
        *providers = provider_scores.into_iter().map(|(provider, _)| provider).collect();
        
        debug!(
            "Reordered providers based on query analysis: {:?}",
            providers.iter().map(|p| p.name()).collect::<Vec<_>>()
        );
    }
    
    /// Calculate domain-specific priority boost
    fn calculate_domain_priority_boost(&self, provider: &Arc<dyn SourceProvider>, query: &str) -> i32 {
        let provider_name = provider.name();
        
        // Computer Science & Machine Learning
        if self.contains_cs_ml_keywords(query) {
            match provider_name {
                "arxiv" => 15,        // arXiv is primary for CS papers
                "openreview" => 12,   // OpenReview for ML conference papers
                "semantic_scholar" => 8, // Good for CS papers
                "core" => 5,          // Open access CS papers
                _ => 0,
            }
        }
        // Biomedical & Life Sciences
        else if self.contains_biomedical_keywords(query) {
            match provider_name {
                "pubmed_central" => 15, // Primary for biomedical
                "biorxiv" => 12,       // Biology preprints
                "semantic_scholar" => 8, // Good coverage
                "unpaywall" => 5,      // Often has biomedical papers
                _ => 0,
            }
        }
        // Physics & Mathematics
        else if self.contains_physics_math_keywords(query) {
            match provider_name {
                "arxiv" => 20,        // Primary for physics/math
                "crossref" => 8,      // Good metadata
                "semantic_scholar" => 5,
                _ => 0,
            }
        }
        // Social Sciences & Economics
        else if self.contains_social_science_keywords(query) {
            match provider_name {
                "ssrn" => 15,         // Primary for social sciences
                "crossref" => 8,      // Good metadata
                "semantic_scholar" => 5,
                _ => 0,
            }
        }
        // Open Access & General Academic
        else if self.contains_open_access_keywords(query) {
            match provider_name {
                "unpaywall" => 12,    // Specialized in open access
                "core" => 10,         // Large open access collection
                "mdpi" => 8,          // Open access publisher
                "biorxiv" => 5,       // Open preprints
                "arxiv" => 5,         // Open preprints
                _ => 0,
            }
        }
        else {
            0 // No domain-specific boost
        }
    }
    
    /// Calculate search type priority boost
    fn calculate_search_type_priority_boost(&self, provider: &Arc<dyn SourceProvider>, query: &SearchQuery) -> i32 {
        let provider_name = provider.name();
        
        match query.search_type {
            SearchType::Doi => {
                // DOI searches work best with metadata providers
                match provider_name {
                    "crossref" => 10,     // Best for DOI resolution
                    "unpaywall" => 8,     // Good DOI support
                    "semantic_scholar" => 6,
                    "pubmed_central" => 5, // Good for biomedical DOIs
                    _ => 0,
                }
            }
            SearchType::Author => {
                // Author searches work best with comprehensive databases
                match provider_name {
                    "semantic_scholar" => 10, // Excellent author disambiguation
                    "crossref" => 8,          // Good author metadata
                    "pubmed_central" => 6,    // Good for biomedical authors
                    "core" => 5,              // Large author database
                    _ => 0,
                }
            }
            SearchType::Title => {
                // All providers are generally good for title searches
                2 // Small boost for all
            }
            SearchType::Keywords => {
                // Keyword searches benefit from full-text providers
                match provider_name {
                    "semantic_scholar" => 8,  // Good semantic search
                    "core" => 6,              // Full-text search
                    "unpaywall" => 4,         // Good coverage
                    _ => 0,
                }
            }
            SearchType::Subject => {
                // Subject searches benefit from specialized providers
                match provider_name {
                    "arxiv" => 8,             // Good subject classification
                    "pubmed_central" => 8,    // Medical subjects
                    "semantic_scholar" => 6,  // AI-powered classification
                    _ => 0,
                }
            }
            SearchType::Auto => {
                0 // No specific boost for auto searches
            }
        }
    }
    
    /// Calculate content availability priority boost
    fn calculate_content_priority_boost(&self, provider: &Arc<dyn SourceProvider>, query: &str) -> i32 {
        let provider_name = provider.name();
        
        // If query suggests need for full-text/PDF access
        if query.contains("pdf") || query.contains("full text") || query.contains("download") {
            match provider_name {
                "arxiv" => 12,            // Always has PDFs
                "biorxiv" => 12,          // Always has PDFs
                "unpaywall" => 10,        // Specialized in free PDFs
                "semantic_scholar" => 8,  // Often has PDF links
                "pubmed_central" => 8,    // Often has full text
                "mdpi" => 8,              // Open access PDFs
                "ssrn" => 6,              // Often has PDFs
                "core" => 6,              // Often has full text
                "sci_hub" => 15,          // Always tries for PDFs (but lowest base priority)
                _ => 0,
            }
        }
        // If query suggests need for recent/preprint content
        else if query.contains("recent") || query.contains("preprint") || query.contains("2024") || query.contains("2023") {
            match provider_name {
                "arxiv" => 10,            // Latest preprints
                "biorxiv" => 10,          // Latest biology preprints
                "ssrn" => 8,              // Recent working papers
                "openreview" => 6,        // Recent ML papers
                _ => 0,
            }
        }
        else {
            0
        }
    }
    
    /// Calculate temporal priority boost for time-sensitive queries
    fn calculate_temporal_priority_boost(&self, provider: &Arc<dyn SourceProvider>, query: &str) -> i32 {
        let provider_name = provider.name();
        
        // Boost for recent year mentions
        if query.contains("2024") || query.contains("2023") {
            match provider_name {
                "arxiv" => 8,             // Best for recent preprints
                "biorxiv" => 8,           // Recent biology papers
                "ssrn" => 6,              // Recent working papers
                "openreview" => 6,        // Recent ML conference papers
                "semantic_scholar" => 4,  // Good recent coverage
                _ => 0,
            }
        }
        // Boost for historical content
        else if query.contains("historical") || query.contains("classic") || query.contains("1990") || query.contains("2000") {
            match provider_name {
                "crossref" => 8,          // Comprehensive historical metadata
                "pubmed_central" => 6,    // Long history for biomedical
                "semantic_scholar" => 4,  // Good historical coverage
                _ => 0,
            }
        }
        else {
            0
        }
    }
    
    /// Check if query contains computer science/ML keywords
    fn contains_cs_ml_keywords(&self, query: &str) -> bool {
        let cs_keywords = [
            "computer science", "machine learning", "deep learning", "neural network", 
            "artificial intelligence", "ai", "ml", "algorithm", "data structure",
            "programming", "software", "computer vision", "nlp", "natural language",
            "database", "distributed system", "security", "cryptography", "compiler",
            "operating system", "network", "internet", "web", "mobile", "app",
            "tensorflow", "pytorch", "keras", "python", "java", "c++", "javascript",
            "transformer", "bert", "gpt", "lstm", "cnn", "gan", "reinforcement learning"
        ];
        
        cs_keywords.iter().any(|&keyword| query.contains(keyword))
    }
    
    /// Check if query contains biomedical keywords
    fn contains_biomedical_keywords(&self, query: &str) -> bool {
        let bio_keywords = [
            "medicine", "medical", "biology", "biomedical", "clinical", "patient",
            "disease", "cancer", "drug", "therapy", "treatment", "diagnosis",
            "gene", "genome", "protein", "dna", "rna", "cell", "molecular",
            "pharmaceutical", "clinical trial", "epidemiology", "public health",
            "neuroscience", "cardiology", "oncology", "immunology", "microbiology",
            "biochemistry", "genetics", "pathology", "pharmacology", "physiology"
        ];
        
        bio_keywords.iter().any(|&keyword| query.contains(keyword))
    }
    
    /// Check if query contains physics/math keywords
    fn contains_physics_math_keywords(&self, query: &str) -> bool {
        let physics_math_keywords = [
            "physics", "quantum", "relativity", "mechanics", "thermodynamics",
            "electromagnetism", "optics", "astronomy", "astrophysics", "cosmology",
            "mathematics", "algebra", "calculus", "geometry", "topology", "statistics",
            "probability", "number theory", "differential equation", "linear algebra",
            "mathematical", "theorem", "proof", "formula", "equation"
        ];
        
        physics_math_keywords.iter().any(|&keyword| query.contains(keyword))
    }
    
    /// Check if query contains social science keywords
    fn contains_social_science_keywords(&self, query: &str) -> bool {
        let social_keywords = [
            "economics", "economic", "finance", "financial", "business", "management",
            "psychology", "sociology", "political science", "anthropology", "education",
            "law", "legal", "policy", "social", "society", "culture", "history",
            "literature", "philosophy", "linguistics", "communication", "media",
            "marketing", "accounting", "organization", "leadership", "strategy"
        ];
        
        social_keywords.iter().any(|&keyword| query.contains(keyword))
    }
    
    /// Check if query contains open access keywords
    fn contains_open_access_keywords(&self, query: &str) -> bool {
        let oa_keywords = [
            "open access", "free", "libre", "creative commons", "cc by", "cc0",
            "public domain", "open source", "preprint", "repository", "institutional",
            "self-archived", "green oa", "gold oa", "hybrid", "subscription"
        ];
        
        oa_keywords.iter().any(|&keyword| query.contains(keyword))
    }

    /// Apply rate limiting for a provider
    async fn apply_rate_limit(provider: &Arc<dyn SourceProvider>) -> Result<(), ProviderError> {
        // Simple rate limiting - wait for base delay since last request
        let base_delay = provider.base_delay();

        // For now, just wait the base delay
        // In a more sophisticated implementation, we'd track per-provider timing
        tokio::time::sleep(base_delay).await;

        Ok(())
    }

    /// Aggregate results from multiple providers
    async fn aggregate_results(
        &self,
        provider_results: Vec<(String, ProviderResult)>,
        provider_errors: HashMap<String, String>,
        start_time: Instant,
    ) -> MetaSearchResult {
        let mut all_papers = Vec::new();
        let mut by_source = HashMap::new();
        let mut provider_metadata = HashMap::new();

        // Collect all papers and organize by source
        for (source, result) in provider_results.iter() {
            by_source.insert(source.clone(), result.papers.clone());
            provider_metadata.insert(source.clone(), result.metadata.clone());
            all_papers.extend(result.papers.clone());
        }

        // Deduplicate if requested
        if self.config.deduplicate_results {
            all_papers = self.deduplicate_papers(all_papers).await;
        }

        // Filter by relevance score if needed
        if self.config.min_relevance_score > 0.0 {
            // Note: We'd need to add relevance scoring to PaperMetadata
            // For now, include all papers
        }

        // Sort by source priority and then by some relevance metric
        // For now, just keep the order

        MetaSearchResult {
            papers: all_papers,
            by_source,
            total_search_time: start_time.elapsed(),
            successful_providers: provider_results.len(),
            failed_providers: provider_errors.len(),
            provider_errors,
            provider_metadata,
        }
    }

    /// Deduplicate papers based on DOI and title similarity
    async fn deduplicate_papers(&self, papers: Vec<PaperMetadata>) -> Vec<PaperMetadata> {
        let original_count = papers.len();
        let mut unique_papers = Vec::new();
        let mut seen_dois = HashSet::new();
        let mut seen_titles = HashSet::new();

        for paper in papers {
            let mut is_duplicate = false;

            // Check DOI duplicates
            if !paper.doi.is_empty() {
                if seen_dois.contains(&paper.doi) {
                    is_duplicate = true;
                } else {
                    seen_dois.insert(paper.doi.clone());
                }
            }

            // Check title duplicates (case-insensitive, normalized)
            if !is_duplicate {
                if let Some(title) = &paper.title {
                    let normalized_title = title.to_lowercase().replace(&[' ', '\t', '\n'], "");
                    if seen_titles.contains(&normalized_title) {
                        is_duplicate = true;
                    } else {
                        seen_titles.insert(normalized_title);
                    }
                }
            }

            if !is_duplicate {
                unique_papers.push(paper);
            }
        }

        debug!(
            "Deduplicated {} papers to {} unique papers",
            original_count,
            unique_papers.len()
        );

        unique_papers
    }

    /// Try to get a PDF URL from any provider, cascading through them by priority
    pub async fn get_pdf_url_cascade(&self, doi: &str) -> Result<Option<String>, ProviderError> {
        info!("Attempting cascade PDF retrieval for DOI: {}", doi);
        
        let context = self.create_search_context().await;
        
        // Sort providers by priority (highest first)
        let mut providers: Vec<_> = self.providers.iter().collect();
        providers.sort_by_key(|p| std::cmp::Reverse(p.priority()));
        
        let mut last_error = None;
        
        for provider in providers {
            info!("Trying PDF retrieval from provider: {} (priority: {})", 
                provider.name(), provider.priority());
            
            // Apply rate limiting
            if let Err(e) = Self::apply_rate_limit(provider).await {
                warn!("Rate limit hit for {}: {}", provider.name(), e);
                last_error = Some(e);
                continue;
            }
            
            // Try to get PDF URL from this provider
            match provider.get_pdf_url(doi, &context).await {
                Ok(Some(pdf_url)) => {
                    info!("Successfully found PDF URL from {}: {}", provider.name(), pdf_url);
                    return Ok(Some(pdf_url));
                }
                Ok(None) => {
                    debug!("Provider {} has no PDF for DOI: {}", provider.name(), doi);
                }
                Err(e) => {
                    warn!("Provider {} failed to get PDF: {}", provider.name(), e);
                    last_error = Some(e);
                }
            }
        }
        
        // If we get here, no provider could provide a PDF
        if let Some(error) = last_error {
            Err(error)
        } else {
            info!("No provider could find a PDF for DOI: {}", doi);
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_meta_search_client_creation() {
        let config = Config::default();
        let meta_config = MetaSearchConfig::default();
        let client = MetaSearchClient::new(config, meta_config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_provider_listing() {
        let config = Config::default();
        let meta_config = MetaSearchConfig::default();
        let client = MetaSearchClient::new(config, meta_config).unwrap();

        let providers = client.providers();
        assert!(providers.contains(&"arxiv".to_string()));
        assert!(providers.contains(&"biorxiv".to_string()));
        assert!(providers.contains(&"core".to_string()));
        assert!(providers.contains(&"crossref".to_string()));
        assert!(providers.contains(&"semantic_scholar".to_string()));
        assert!(providers.contains(&"unpaywall".to_string()));
        assert!(providers.contains(&"ssrn".to_string()));
        assert!(providers.contains(&"sci_hub".to_string()));
    }

    #[tokio::test]
    async fn test_deduplication() {
        let config = Config::default();
        let meta_config = MetaSearchConfig::default();
        let client = MetaSearchClient::new(config, meta_config).unwrap();

        let papers = vec![
            PaperMetadata {
                doi: "10.1038/nature12373".to_string(),
                title: Some("Test Paper".to_string()),
                authors: vec!["Author 1".to_string()],
                journal: Some("Nature".to_string()),
                year: Some(2023),
                abstract_text: None,
                pdf_url: None,
                file_size: None,
            },
            PaperMetadata {
                doi: "10.1038/nature12373".to_string(), // Same DOI
                title: Some("Test Paper".to_string()),
                authors: vec!["Author 1".to_string()],
                journal: Some("Nature".to_string()),
                year: Some(2023),
                abstract_text: None,
                pdf_url: None,
                file_size: None,
            },
        ];

        let deduplicated = client.deduplicate_papers(papers).await;
        assert_eq!(deduplicated.len(), 1);
    }
}
