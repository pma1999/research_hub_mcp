pub mod arxiv;
pub mod biorxiv;
pub mod core;
pub mod crossref;
pub mod sci_hub;
pub mod semantic_scholar;
pub mod ssrn;
pub mod traits;
pub mod unpaywall;

pub use arxiv::ArxivProvider;
pub use biorxiv::BiorxivProvider;
pub use core::CoreProvider;
pub use crossref::CrossRefProvider;
pub use sci_hub::SciHubProvider;
pub use semantic_scholar::SemanticScholarProvider;
pub use ssrn::SsrnProvider;
pub use traits::{
    ProviderError, ProviderResult, SearchContext, SearchQuery, SearchType, SourceProvider,
};
pub use unpaywall::UnpaywallProvider;
