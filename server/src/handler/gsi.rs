use anyhow::Result;

use serde::{ Serialize, Deserialize };
use jsonwebtoken::{ decode, decode_header, Algorithm, Validation, DecodingKey };

#[derive(Deserialize)]
struct JsonWebKey {
  #[allow(dead_code)]
  r#use: String,
  #[allow(dead_code)]
  kty: String,
  #[allow(dead_code)]
  alg: String,
  kid: String,
  e: String,
  n: String,
}

#[derive(Deserialize)]
struct GoogleCerts {
  keys: Vec<JsonWebKey>,
}

const GOOGLE_ISSUER: &str = "https://accounts.google.com";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
  iss: String,
  aud: String,
  email: String,
  email_verified: bool,
  name: String,
  iat: u64,
  exp: u64,
}

use isahc::prelude::*;

async fn get_google_pubkeys() -> Result<String> {
  let res = isahc::get_async("https://www.googleapis.com/oauth2/v3/certs").await?.text().await?;
  Ok(res)
}

pub async fn get_email_from_token(token: &str, client_id: &str) -> Result<String> {
  let keys = get_google_pubkeys().await?;
  let google_certs: GoogleCerts = serde_json::from_str(keys.as_str())?;

  let header = decode_header(token)?;
  let jwk: JsonWebKey;
  if let Some(v) = header.kid {
    jwk = google_certs.keys
      .into_iter()
      .find(|x| x.kid == v)
      .ok_or_else(|| anyhow::format_err!("cannot find matching JWK for {}", v))?;
  } else {
    return Err(anyhow::Error::msg("invalid google JWK key format"));
  }

  let key = &DecodingKey::from_rsa_components(jwk.n.as_str(), jwk.e.as_str())?;

  // expired tokens, etc are handled here
  let token_data = decode::<Claims>(token, key, &Validation::new(Algorithm::RS256)).map_err(|e|
    anyhow::format_err!("cannot decode JWT token: {}", e)
  )?;

  if token_data.claims.iss.as_str() != GOOGLE_ISSUER {
    return Err(anyhow::format_err!("wrong issuer"));
  }

  if token_data.claims.aud.as_str() != client_id {
    return Err(anyhow::format_err!("wrong client ID"));
  }

  if token_data.claims.email.is_empty() {
    return Err(anyhow::format_err!("cannot read email from token"));
  }
  if !token_data.claims.email_verified {
    return Err(anyhow::format_err!("email is not verified"));
  }
  Ok(token_data.claims.email)
}
