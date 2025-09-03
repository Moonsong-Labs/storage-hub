//! This module contains the handlers for the file management endpoints
//!
//! TODO: move the rest of the endpoints as they are implemented

pub async fn get_file_info(
    State(services): State<Services>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let payload = extract_bearer_token(&auth)?;
    let address = payload
        .get("address")
        .and_then(|a| a.as_str())
        .unwrap_or(MOCK_ADDRESS);

    let response = services
        .msp
        .get_file_info(&bucket_id, address, &file_key)
        .await?;
    Ok(Json(response))
}
