//! Authentication middleware for WebSocket connections.
//!
//! Supports API key authentication via header or query parameter,
//! and optional JWT validation (feature-gated).

use std::collections::HashSet;
use std::sync::Arc;

use tracing::{debug, warn};

/// Authentication configuration.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Whether authentication is required
    pub require_auth: bool,
    /// Valid API keys (if empty and require_auth is true, all requests are rejected)
    pub api_keys: HashSet<String>,
    /// JWT configuration (feature-gated)
    #[cfg(feature = "jwt")]
    pub jwt_config: Option<JwtConfig>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            require_auth: false,
            api_keys: HashSet::new(),
            #[cfg(feature = "jwt")]
            jwt_config: None,
        }
    }
}

impl AuthConfig {
    /// Create a new auth config that doesn't require authentication.
    pub fn no_auth() -> Self {
        Self::default()
    }

    /// Create a new auth config that requires API key authentication.
    pub fn with_api_keys(keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            require_auth: true,
            api_keys: keys.into_iter().map(Into::into).collect(),
            #[cfg(feature = "jwt")]
            jwt_config: None,
        }
    }

    /// Add an API key to the valid keys set.
    pub fn add_api_key(&mut self, key: impl Into<String>) {
        self.api_keys.insert(key.into());
    }

    /// Check if a given API key is valid.
    pub fn is_valid_api_key(&self, key: &str) -> bool {
        self.api_keys.contains(key)
    }
}

/// JWT configuration for token validation.
#[cfg(feature = "jwt")]
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for HMAC validation or public key for RSA/EC
    pub secret: String,
    /// Expected issuer (iss claim)
    pub issuer: Option<String>,
    /// Expected audience (aud claim)
    pub audience: Option<String>,
    /// Algorithm to use for validation
    pub algorithm: JwtAlgorithm,
}

/// Supported JWT algorithms.
#[cfg(feature = "jwt")]
#[derive(Debug, Clone, Copy, Default)]
pub enum JwtAlgorithm {
    #[default]
    HS256,
    HS384,
    HS512,
    RS256,
    RS384,
    RS512,
}

/// Result of authentication attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum AuthResult {
    /// Authentication successful
    Authenticated {
        /// Identity of the authenticated client (API key ID or JWT subject)
        identity: String,
    },
    /// No authentication provided but not required
    Anonymous,
    /// Authentication failed
    Failed {
        /// Reason for failure
        reason: String,
    },
}

impl AuthResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Authenticated { .. } | Self::Anonymous)
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Failed { reason } => Some(reason),
            _ => None,
        }
    }

    pub fn identity(&self) -> Option<&str> {
        match self {
            Self::Authenticated { identity } => Some(identity),
            _ => None,
        }
    }
}

/// Authentication errors.
#[derive(Debug, Clone, PartialEq)]
pub enum AuthError {
    /// No authentication credentials provided
    MissingCredentials,
    /// Invalid API key
    InvalidApiKey,
    /// Invalid JWT token
    #[cfg(feature = "jwt")]
    InvalidToken(String),
    /// JWT token expired
    #[cfg(feature = "jwt")]
    TokenExpired,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCredentials => write!(f, "Missing authentication credentials"),
            Self::InvalidApiKey => write!(f, "Invalid API key"),
            #[cfg(feature = "jwt")]
            Self::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            #[cfg(feature = "jwt")]
            Self::TokenExpired => write!(f, "Token has expired"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Authenticator that validates incoming connections.
#[derive(Debug)]
pub struct Authenticator {
    config: AuthConfig,
}

impl Authenticator {
    /// Create a new authenticator with the given configuration.
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    /// Create an authenticator that doesn't require authentication.
    pub fn no_auth() -> Self {
        Self::new(AuthConfig::no_auth())
    }

    /// Check if authentication is required.
    pub fn requires_auth(&self) -> bool {
        self.config.require_auth
    }

    /// Authenticate a request using the provided credentials.
    pub fn authenticate(&self, credentials: &AuthCredentials) -> AuthResult {
        if !self.config.require_auth {
            // Authentication not required - allow anonymous access
            if let Some(api_key) = &credentials.api_key {
                // But validate if provided
                if self.config.is_valid_api_key(api_key) {
                    debug!("Authenticated with optional API key");
                    return AuthResult::Authenticated {
                        identity: format!("api_key:{}", &api_key[..8.min(api_key.len())]),
                    };
                }
                // Invalid key provided but auth not required - still allow
                debug!("Invalid API key provided but auth not required, allowing anonymous");
            }
            return AuthResult::Anonymous;
        }

        // Authentication required - check credentials
        if let Some(api_key) = &credentials.api_key {
            return self.validate_api_key(api_key);
        }

        #[cfg(feature = "jwt")]
        if let Some(token) = &credentials.jwt_token {
            return self.validate_jwt(token);
        }

        warn!("Authentication required but no credentials provided");
        AuthResult::Failed {
            reason: "Authentication required. Provide X-API-Key header or api_key query parameter."
                .to_string(),
        }
    }

    /// Validate an API key.
    fn validate_api_key(&self, key: &str) -> AuthResult {
        if self.config.is_valid_api_key(key) {
            debug!("Valid API key: {}...", &key[..8.min(key.len())]);
            AuthResult::Authenticated {
                identity: format!("api_key:{}", &key[..8.min(key.len())]),
            }
        } else {
            warn!("Invalid API key attempt: {}...", &key[..8.min(key.len())]);
            AuthResult::Failed {
                reason: "Invalid API key".to_string(),
            }
        }
    }

    /// Validate a JWT token.
    #[cfg(feature = "jwt")]
    fn validate_jwt(&self, token: &str) -> AuthResult {
        use jsonwebtoken::{decode, DecodingKey, Validation};

        let jwt_config = match &self.config.jwt_config {
            Some(config) => config,
            None => {
                return AuthResult::Failed {
                    reason: "JWT authentication not configured".to_string(),
                };
            }
        };

        let mut validation = Validation::default();

        // Set algorithm
        validation.algorithms = vec![match jwt_config.algorithm {
            JwtAlgorithm::HS256 => jsonwebtoken::Algorithm::HS256,
            JwtAlgorithm::HS384 => jsonwebtoken::Algorithm::HS384,
            JwtAlgorithm::HS512 => jsonwebtoken::Algorithm::HS512,
            JwtAlgorithm::RS256 => jsonwebtoken::Algorithm::RS256,
            JwtAlgorithm::RS384 => jsonwebtoken::Algorithm::RS384,
            JwtAlgorithm::RS512 => jsonwebtoken::Algorithm::RS512,
        }];

        // Set issuer validation
        if let Some(issuer) = &jwt_config.issuer {
            validation.set_issuer(&[issuer]);
        }

        // Set audience validation
        if let Some(audience) = &jwt_config.audience {
            validation.set_audience(&[audience]);
        }

        // Decode and validate
        let decoding_key = DecodingKey::from_secret(jwt_config.secret.as_bytes());

        match decode::<JwtClaims>(token, &decoding_key, &validation) {
            Ok(token_data) => {
                debug!("Valid JWT for subject: {}", token_data.claims.sub);
                AuthResult::Authenticated {
                    identity: format!("jwt:{}", token_data.claims.sub),
                }
            }
            Err(e) => {
                warn!("JWT validation failed: {}", e);
                AuthResult::Failed {
                    reason: format!("Invalid token: {}", e),
                }
            }
        }
    }
}

/// JWT claims structure.
#[cfg(feature = "jwt")]
#[derive(Debug, serde::Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: Option<i64>,
    /// Issued at time (Unix timestamp)
    pub iat: Option<i64>,
    /// Issuer
    pub iss: Option<String>,
    /// Audience
    pub aud: Option<String>,
}

/// Credentials extracted from an incoming request.
#[derive(Debug, Default, Clone)]
pub struct AuthCredentials {
    /// API key from X-API-Key header or api_key query parameter
    pub api_key: Option<String>,
    /// JWT token from Authorization: Bearer header
    #[cfg(feature = "jwt")]
    pub jwt_token: Option<String>,
}

impl AuthCredentials {
    /// Create empty credentials.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create credentials with an API key.
    pub fn with_api_key(key: impl Into<String>) -> Self {
        Self {
            api_key: Some(key.into()),
            #[cfg(feature = "jwt")]
            jwt_token: None,
        }
    }

    /// Parse credentials from HTTP headers and query string.
    pub fn from_request(headers: &str, query_string: Option<&str>) -> Self {
        let mut credentials = Self::default();

        // Parse X-API-Key header
        for line in headers.lines() {
            let line_lower = line.to_lowercase();
            if line_lower.starts_with("x-api-key:") {
                if let Some(key) = line.get(10..).map(str::trim) {
                    credentials.api_key = Some(key.to_string());
                }
            }
            #[cfg(feature = "jwt")]
            if line_lower.starts_with("authorization:") {
                if let Some(value) = line.get(14..).map(str::trim) {
                    if value.to_lowercase().starts_with("bearer ") {
                        if let Some(token) = value.get(7..).map(str::trim) {
                            credentials.jwt_token = Some(token.to_string());
                        }
                    }
                }
            }
        }

        // Parse api_key from query string (lower priority than header)
        if credentials.api_key.is_none() {
            if let Some(qs) = query_string {
                for pair in qs.split('&') {
                    if let Some((key, value)) = pair.split_once('=') {
                        if key == "api_key" {
                            credentials.api_key = Some(value.to_string());
                            break;
                        }
                    }
                }
            }
        }

        credentials
    }

    /// Parse credentials from a WebSocket upgrade request path.
    pub fn from_ws_path(path: &str) -> Self {
        let query_string = path.split_once('?').map(|(_, qs)| qs);
        Self::from_request("", query_string)
    }
}

/// Shared authenticator type for use across the server.
pub type SharedAuthenticator = Arc<Authenticator>;

/// Create a new shared authenticator with the given configuration.
pub fn create_authenticator(config: AuthConfig) -> SharedAuthenticator {
    Arc::new(Authenticator::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_default_allows_anonymous() {
        let config = AuthConfig::default();
        assert!(!config.require_auth);
        assert!(config.api_keys.is_empty());
    }

    #[test]
    fn test_auth_config_with_api_keys() {
        let config = AuthConfig::with_api_keys(["key1", "key2"]);
        assert!(config.require_auth);
        assert!(config.is_valid_api_key("key1"));
        assert!(config.is_valid_api_key("key2"));
        assert!(!config.is_valid_api_key("key3"));
    }

    #[test]
    fn test_authenticator_no_auth_allows_anonymous() {
        let auth = Authenticator::no_auth();
        let creds = AuthCredentials::empty();
        let result = auth.authenticate(&creds);
        assert_eq!(result, AuthResult::Anonymous);
    }

    #[test]
    fn test_authenticator_requires_auth_rejects_empty() {
        let config = AuthConfig::with_api_keys(["valid_key"]);
        let auth = Authenticator::new(config);
        let creds = AuthCredentials::empty();
        let result = auth.authenticate(&creds);
        assert!(!result.is_allowed());
    }

    #[test]
    fn test_authenticator_validates_api_key() {
        let config = AuthConfig::with_api_keys(["my_secret_key_12345"]);
        let auth = Authenticator::new(config);

        let valid_creds = AuthCredentials::with_api_key("my_secret_key_12345");
        let result = auth.authenticate(&valid_creds);
        assert!(result.is_allowed());
        assert!(result.identity().unwrap().starts_with("api_key:"));

        let invalid_creds = AuthCredentials::with_api_key("wrong_key");
        let result = auth.authenticate(&invalid_creds);
        assert!(!result.is_allowed());
    }

    #[test]
    fn test_credentials_from_header() {
        let headers = "GET /ws HTTP/1.1\r\nHost: localhost\r\nX-API-Key: test_key_123\r\n";
        let creds = AuthCredentials::from_request(headers, None);
        assert_eq!(creds.api_key.as_deref(), Some("test_key_123"));
    }

    #[test]
    fn test_credentials_from_query_string() {
        let creds = AuthCredentials::from_request("", Some("api_key=query_key_456&other=value"));
        assert_eq!(creds.api_key.as_deref(), Some("query_key_456"));
    }

    #[test]
    fn test_credentials_header_takes_priority() {
        let headers = "X-API-Key: header_key";
        let creds = AuthCredentials::from_request(headers, Some("api_key=query_key"));
        assert_eq!(creds.api_key.as_deref(), Some("header_key"));
    }

    #[test]
    fn test_credentials_from_ws_path() {
        let creds = AuthCredentials::from_ws_path("/ws?api_key=path_key_789");
        assert_eq!(creds.api_key.as_deref(), Some("path_key_789"));
    }

    #[test]
    fn test_auth_result_methods() {
        let authenticated = AuthResult::Authenticated {
            identity: "test".to_string(),
        };
        assert!(authenticated.is_allowed());
        assert!(authenticated.error_message().is_none());
        assert_eq!(authenticated.identity(), Some("test"));

        let anonymous = AuthResult::Anonymous;
        assert!(anonymous.is_allowed());
        assert!(anonymous.identity().is_none());

        let failed = AuthResult::Failed {
            reason: "test error".to_string(),
        };
        assert!(!failed.is_allowed());
        assert_eq!(failed.error_message(), Some("test error"));
    }
}
