use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_jwt::{Claims, Decoder};
use chrono::{DateTime, Utc};
use tracing::{debug, warn};

use crate::{
    constants::mocks::MOCK_ADDRESS, error::Error, models::auth::JwtClaims, services::Services,
};

/// Axum extractor to identify the user.
///
/// Will error if the JWT token is expired or it is otherwise invalid
///
/// If no JWT is present, the user will result "Unauthenticated" and will receive an ID
pub enum User {
    /// Represents an authenticated user
    ///
    /// The user is identified by the address used during the login flow
    Authenticated { address: String },

    /// Represents an unauthenticated user
    ///
    /// The user is identified by this ID, which holds no guarantees in terms of "stickiness" nor "uniqueness"
    Unauthenticated { id: String },
}

enum AuthenticationResult {
    Success(User),
    NoJWT,
    BadJWT(JwtClaims, Error),
    Error(Error),
}

impl User {
    /// Will return a string usable to identify the user for the session
    ///
    /// WARNING: Do not use for identify verification
    pub fn id(&self) -> &String {
        match self {
            User::Authenticated { address } => &address,
            User::Unauthenticated { id } => &id,
        }
    }

    /// Will return the authenticated user address or error if the user is unauthenticated
    pub fn address(&self) -> Result<&String, Error> {
        match self {
            Self::Authenticated { address } => Ok(&address),
            _ => Err(Error::Unauthorized("User not authenticated".to_owned())),
        }
    }

    async fn unauthenticated_from_request_parts<S>(
        _parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Error>
    where
        S: Send + Sync,
    {
        // TODO: determine user ID from peer information in parts
        Ok(Self::Unauthenticated {
            id: "unauthenticated".to_string(),
        })
    }

    /// Verifies the passed in `JwtClaims`
    ///
    /// Returns the user information if the claims are valid and not expired
    // TODO: user logout verification
    fn from_claims(claims: &JwtClaims) -> Result<Self, Error> {
        let now = Utc::now();
        let exp = DateTime::<Utc>::from_timestamp(claims.exp, 0)
            .ok_or_else(|| Error::Unauthorized("Invalid JWT expiry".to_string()))?;
        let iat = DateTime::<Utc>::from_timestamp(claims.iat, 0)
            .ok_or_else(|| Error::Unauthorized("Invalid JWT issuance time".to_string()))?;

        if now >= exp {
            return Err(Error::Unauthorized("Expired JWT".to_string()));
        }

        if iat > now {
            return Err(Error::Unauthorized("JWT issued in the future".to_string()));
        }

        Ok(Self::Authenticated {
            address: claims.address.clone(),
        })
    }

    async fn authenticated_from_request_parts<S>(
        parts: &mut Parts,
        state: &S,
    ) -> AuthenticationResult
    where
        S: Send + Sync,
        Decoder: FromRef<S>,
    {
        // We try to parse the JWT token, if not preset we proceed as Unauthenticated
        let claims = Claims::<JwtClaims>::from_request_parts(parts, state).await;

        match claims {
            Ok(claims) => Self::from_claims(&claims.0)
                .map(AuthenticationResult::Success)
                .unwrap_or_else(|e| AuthenticationResult::BadJWT(claims.0, e)),
            // no JWT
            Err(axum_jwt::Error::AuthorizationHeader) => AuthenticationResult::NoJWT,
            Err(e) => {
                AuthenticationResult::Error(Error::Unauthorized(format!("Invalid JWT: {e:?}")))
            }
        }
    }

    async fn from_request_parts_impl<S>(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, (Option<JwtClaims>, Error)>
    where
        S: Send + Sync,
        Decoder: FromRef<S>,
    {
        match Self::authenticated_from_request_parts(parts, state).await {
            AuthenticationResult::Success(user) => Ok(user),
            AuthenticationResult::NoJWT => Self::unauthenticated_from_request_parts(parts, state)
                .await
                .map_err(|e| (None, e)),
            AuthenticationResult::BadJWT(jwt_claims, error) => Err((Some(jwt_claims), error)),
            AuthenticationResult::Error(error) => Err((None, error)),
        }
    }
}

impl<S> FromRequestParts<S> for User
where
    Decoder: FromRef<S>,
    Services: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let services = Services::from_ref(state);
        let maybe_auth = Self::from_request_parts_impl(parts, state).await;

        match maybe_auth {
            Ok(user) => Ok(user),
            // if services are configured to not validate signature
            Err((claims, e)) if !services.auth.validate_signature => {
                warn!(target: "auth_service::from_request_parts", error = ?e, "Authentication failed");

                // if we were able to retrieve the claims then use the passed in address
                let address = claims
                    .map(|claims| claims.address)
                    .unwrap_or_else(|| MOCK_ADDRESS.to_string());
                debug!(target: "auth_service::from_request_parts", address = %address, "Bypassing authentication");

                return Ok(Self::Authenticated { address });
            }
            Err((_, e)) => Err(e),
        }
    }
}

/// Identical to [`User::Authenticated`] variant
pub struct AuthenticatedUser {
    pub address: String,
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    Decoder: FromRef<S>,
    Services: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = User::from_request_parts(parts, state).await?;

        match user {
            User::Authenticated { address } => Ok(Self { address }),
            _ => Err(Error::Unauthorized(
                "No authentication token provided.".to_owned(),
            )),
        }
    }
}
