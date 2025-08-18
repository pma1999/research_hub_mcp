# Examples and Usage Guides

This directory contains examples and usage guides for rust-research-mcp.

## 📚 Quick Start Examples

### Basic Configuration

**Claude Desktop Setup:**
```json
{
  "mcpServers": {
    "rust-research-mcp": {
      "command": "/opt/homebrew/bin/rust-research-mcp",
      "args": ["--config", "~/.config/rust-research-mcp/config.toml"]
    }
  }
}
```

**Basic Configuration File:**
```toml
[server]
port = 3000
host = "localhost"
timeout_secs = 30

[research_source]
max_results_per_provider = 10
timeout_secs = 30
concurrent_searches = 3

[downloads]
directory = "~/Downloads/papers"
max_concurrent = 3
timeout_secs = 60

[logging]
level = "info"
```

## 🔍 Search Examples

### 1. Computer Science Papers

**Query:** "machine learning transformers"
```
Expected Sources: arXiv (priority 95), OpenReview (priority 85), Semantic Scholar (priority 82)
Results: Recent ML papers with focus on transformer architectures
```

**Query:** "neural networks deep learning"
```
Expected Sources: arXiv, Semantic Scholar, OpenReview
Results: Comprehensive deep learning research papers
```

### 2. Biomedical Research

**Query:** "cancer immunotherapy clinical trials"
```
Expected Sources: PubMed Central (priority 89), Semantic Scholar (priority 82)
Results: Clinical trial papers and immunotherapy research
```

**Query:** "COVID-19 vaccine efficacy"
```
Expected Sources: PubMed Central, bioRxiv (priority 88), Semantic Scholar
Results: Recent vaccine research and clinical data
```

### 3. Physics and Mathematics

**Query:** "quantum computing algorithms"
```
Expected Sources: arXiv (priority 95), Semantic Scholar (priority 82)
Results: Latest quantum computing research
```

**Query:** "differential equations numerical methods"
```
Expected Sources: arXiv, Semantic Scholar, CrossRef (priority 80)
Results: Mathematical research and computational methods
```

### 4. Social Sciences

**Query:** "economic policy behavioral economics"
```
Expected Sources: SSRN (priority 78), Semantic Scholar, CrossRef
Results: Economic research and policy papers
```

### 5. Open Access Research

**Query:** "open access climate change"
```
Expected Sources: Unpaywall (priority 76), CORE (priority 74), MDPI (priority 75)
Results: Freely available climate research
```

## 📥 Download Examples

### DOI-based Downloads

**Example DOIs to try:**
```
10.1038/s41586-021-03819-2  # Nature paper
10.1126/science.abc123      # Science paper
10.1145/3394486.3403043     # ACM paper
```

**Command:**
```bash
# Via Claude: "Download paper with DOI 10.1038/s41586-021-03819-2"
```

### Title-based Search and Download

**Example workflow:**
1. Search: "BERT language model attention"
2. Review results from multiple providers
3. Select paper for download
4. System attempts download across providers with fallback

## 🎯 Provider-Specific Examples

### arXiv (Priority: 95)
**Best for:** Physics, CS, Mathematics preprints
**Example:** "quantum machine learning"
```
Search Type: Title/Keywords
Expected Results: Latest preprints in quantum ML
PDF Access: Direct (always available)
```

### PubMed Central (Priority: 89)
**Best for:** Biomedical and life sciences
**Example:** "CRISPR gene editing"
```
Search Type: Title/Author/Keywords
Expected Results: Peer-reviewed biomedical papers
PDF Access: Often available (PMC open access)
```

### OpenReview (Priority: 85)
**Best for:** ML conference papers
**Example:** "attention mechanisms transformer"
```
Search Type: Title/Keywords
Expected Results: NeurIPS, ICLR, ICML papers
PDF Access: Conference papers with reviews
```

### Semantic Scholar (Priority: 82)
**Best for:** Cross-disciplinary AI-powered search
**Example:** "interdisciplinary machine learning"
```
Search Type: All types supported
Expected Results: High-quality papers with semantic understanding
PDF Access: When available from publishers
```

### CrossRef (Priority: 80)
**Best for:** DOI resolution and metadata
**Example:** DOI lookup
```
Search Type: DOI (primary), Title/Author
Expected Results: Authoritative metadata
PDF Access: Links to publisher sites
```

### SSRN (Priority: 78)
**Best for:** Social sciences working papers
**Example:** "behavioral economics financial markets"
```
Search Type: Title/Keywords/Author
Expected Results: Latest working papers in economics
PDF Access: Often available directly
```

### Unpaywall (Priority: 76)
**Best for:** Legal free PDF discovery
**Example:** Any DOI
```
Search Type: DOI (primary)
Expected Results: Legal open access versions
PDF Access: Direct links to free PDFs
```

### MDPI (Priority: 75)
**Best for:** Open access journals
**Example:** "sensors IoT machine learning"
```
Search Type: Title/Keywords
Expected Results: Open access journal articles
PDF Access: Direct (open access)
```

### CORE (Priority: 74)
**Best for:** Large open access collection
**Example:** "sustainability renewable energy"
```
Search Type: Title/Keywords
Expected Results: Open access papers from repositories
PDF Access: Repository links
```

### bioRxiv (Priority: 88 for biology)
**Best for:** Biology preprints
**Example:** "protein folding AlphaFold"
```
Search Type: Title/Keywords
Expected Results: Latest biology preprints
PDF Access: Direct (always available)
```

## 🔧 Advanced Configuration

### Multi-Domain Research Setup

```toml
[research_source]
# Optimize for multi-disciplinary research
max_results_per_provider = 15
timeout_secs = 45
concurrent_searches = 5

# Provider-specific settings
[research_source.provider_config]
arxiv_priority_boost = 5  # Extra boost for physics/CS
pubmed_priority_boost = 10  # Extra boost for biomedical
semantic_scholar_api_key = "your-api-key"  # For higher rate limits
```

### High-Volume Research Setup

```toml
[downloads]
directory = "~/Research/Papers"
max_concurrent = 5
timeout_secs = 120
organize_by_date = true
organize_by_subject = true

[research_source]
max_results_per_provider = 25
parallel_provider_calls = true
aggressive_caching = true
```

### Privacy-Focused Setup

```toml
[server]
# Use local-only configuration
host = "127.0.0.1"
disable_telemetry = true

[research_source]
# Minimize external calls
max_results_per_provider = 5
respect_robots_txt = true
user_agent = "Academic Research Tool"
```

## 🚨 Troubleshooting Examples

### Common Issues and Solutions

**Issue:** "No results found"
```
Possible causes:
1. Query too specific - try broader terms
2. Spelling errors - check query spelling
3. Provider temporary issues - check logs

Solution: Try alternative queries or check provider status
```

**Issue:** "Download failed"
```
Possible causes:
1. Paper not freely available
2. Network connectivity issues
3. Provider rate limiting

Solution: System will try multiple providers automatically
```

**Issue:** "Connection timeout"
```
Possible causes:
1. Slow network connection
2. Provider server issues
3. Firewall blocking requests

Solution: Increase timeout in configuration
```

## 📊 Performance Optimization

### Search Performance Tips

1. **Use Specific Terms:** "BERT transformer" vs "machine learning"
2. **Leverage Provider Strengths:** arXiv for CS, PubMed for medicine
3. **DOI When Available:** Direct DOI lookup is fastest
4. **Author Search:** Use full names: "Geoffrey Hinton"

### Download Optimization

1. **Parallel Downloads:** Enable concurrent downloads
2. **Provider Fallback:** System automatically tries multiple sources
3. **Resume Support:** Partial downloads can be resumed
4. **Local Caching:** Avoid re-downloading same papers

## 🎓 Academic Workflow Integration

### Literature Review Workflow

1. **Broad Search:** Start with general terms
2. **Refine Results:** Use filters and specific providers
3. **Follow Citations:** Use DOI lookup for referenced papers
4. **Organize Downloads:** Use date/subject organization

### Research Project Setup

1. **Create Project Directory:** Organize by research topic
2. **Configure Providers:** Optimize for your field
3. **Set Up Monitoring:** Track download success rates
4. **Regular Updates:** Keep provider list current

## 📋 Best Practices

### Ethical Usage

- **Rate Limiting:** Don't overwhelm providers
- **Terms Compliance:** Respect all provider ToS
- **Citation:** Properly cite downloaded papers
- **Legal Access:** Only access papers you have rights to

### Technical Best Practices

- **Regular Updates:** Keep software updated
- **Backup Configuration:** Save working configurations
- **Monitor Logs:** Check for errors and warnings
- **Test Regularly:** Verify provider functionality

---

For more examples and advanced usage, check the project documentation and community discussions!