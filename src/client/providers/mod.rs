pub mod arxiv;
pub mod crossref;
pub mod sci_hub;
pub mod traits;

pub use traits::{SourceProvider, SearchQuery, SearchContext, ProviderResult, ProviderError, SearchType};
pub use sci_hub::SciHubProvider;
pub use arxiv::ArxivProvider;
pub use crossref::CrossRefProvider;