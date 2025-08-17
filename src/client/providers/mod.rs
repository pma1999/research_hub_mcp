pub mod arxiv;
pub mod crossref;
pub mod sci_hub;
pub mod traits;

pub use arxiv::ArxivProvider;
pub use crossref::CrossRefProvider;
pub use sci_hub::SciHubProvider;
pub use traits::{
    ProviderError, ProviderResult, SearchContext, SearchQuery, SearchType, SourceProvider,
};
