//! Network operations module

pub mod utils;

// Network modules will be implemented here
// For now, we include the utilities that support uri and get_url modules

pub use utils::{
    AuthError, AuthHandler, AuthMethod, BodyFormat, CertificateError, CertificateInfo,
    CertificateManager, FollowRedirects, HttpClientError, HttpClientWrapper, HttpMethod,
    HttpResponse, RequestConfig,
};
