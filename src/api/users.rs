use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use log::{debug, error, info};
use serde_json::json;

use crate::api::{does_user_exist, emit_event};
use crate::ApiState;
use crate::commands::RegisterUserCommand;
use crate::events::{UserCreationEvent, UserRegisteredEvent};
use crate::models::{User, UserAuthRecord};

pub async fn register_user(
    State(state): State<ApiState>,
    Json(params): Json<RegisterUserCommand>,
) -> impl IntoResponse {
    debug!("Request received: {:#?}", params);
    let user_id = params.user_id.clone();

    // Make sure the user does not exist
    if does_user_exist(&state.firestore_client, &user_id).await? {
        return Err((
            StatusCode::FOUND,
            Json(json!({"status": "User already exists"})),
        ));
    };

    let auth_record = UserAuthRecord {
        email: params.email.clone(),
        password: params.password.clone(),
    };

    match state
        .firestore_client
        .fluent()
        .insert()
        .into("records")
        .document_id(&params.user_id)
        .object(&auth_record)
        .execute::<UserAuthRecord>()
        .await
    {
        Ok(_) => {
            info!("Auth record created for user: {:#?}", params.user_id);
        }
        Err(e) => {
            error!("Failed to create auth record: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": format!("Failed to create auth record: {}", e)})),
            ));
        }
    };

    emit_event(
        &state.pubsub_client,
        "UserRegisteredEvent",
        &serde_json::to_string(&UserRegisteredEvent {
            user_id: params.user_id.clone(),
            email: params.email.clone(),
            username: params.username.clone(),
            first_name: params.first_name.clone(),
            last_name: params.last_name.clone(),
        })
            .unwrap(),
    )
        .await?;

    return Ok((
        StatusCode::CREATED, "User registered successfully"
    ));
}