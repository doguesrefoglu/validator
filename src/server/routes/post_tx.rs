use std::sync::{atomic::Ordering, RwLock};

use crate::{
    database::schema::transactions::dsl::*, key_manager, server::error::ValidatorServerError,
    state::ValidatorState, types::DbPool,
};
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use bundlr_sdk::{deep_hash::DeepHashChunk, deep_hash_sync::deep_hash_sync};
use bytes::Bytes;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use paris::error;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ValidatorSignature {
    public: String,
    signature: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PostTxBody {
    id: String,
    signature: String,
    block: i64,
    address: String,
    #[serde(default)]
    validator_signatures: Vec<ValidatorSignature>,
}

// Receive Bundlr transaction receipt
pub async fn post_tx<Config, KeyManager>(
    ctx: Data<Config>,
    body: Json<PostTxBody>,
    db: Data<DbPool>,
    _awc_client: Data<awc::Client>,
    validators: Data<RwLock<Vec<String>>>,
) -> actix_web::Result<HttpResponse, ValidatorServerError>
where
    Config: super::sign::Config<KeyManager> + 'static,
    KeyManager: key_manager::KeyManager + Clone + Send + 'static,
{
    let s = ctx.get_validator_state().load(Ordering::SeqCst);
    if s != ValidatorState::Leader {
        return Ok(HttpResponse::BadRequest().finish());
    }

    let _validators = validators.into_inner();
    let body = body.into_inner();

    let exists = {
        let conn = db.get().map_err(|err| {
            error!("Failed to get database connection: {:?}", err);
            ValidatorServerError::InternalError
        })?;
        let filter = id.eq(body.id.clone());
        actix_rt::task::spawn_blocking(move || {
            match transactions.filter(filter).count().get_result(&conn) {
                Ok(0) => Ok(false),
                Ok(_) => Ok(true),
                Err(err) => Err(err),
            }
        })
    };

    if let Ok(true) = exists
        .await
        .map_err(|_| ValidatorServerError::InternalError)?
    {
        return Ok(HttpResponse::Accepted().finish());
    }

    // Check address is valid

    let key_manager = ctx.key_manager().clone();

    let (valid, body) = actix_rt::task::spawn_blocking(move || {
        let hash = deep_hash_body(&body).unwrap();
        let valid = key_manager.verify_validator_signature(&hash, body.signature.as_bytes());
        (valid, body)
    })
    .await?;

    if !valid {
        tracing::info!("Received invalid signature");
        return Ok(HttpResponse::BadRequest().finish());
    };

    add_to_db(&body).await?;

    Ok(HttpResponse::Ok().finish())
}

// TODO: Fix this
fn deep_hash_body(body: &PostTxBody) -> Result<Bytes, ValidatorServerError> {
    deep_hash_sync(DeepHashChunk::Chunks(vec![
        DeepHashChunk::Chunk(body.id.clone().into()),
        DeepHashChunk::Chunk(body.block.to_string().into()),
    ]))
    .map_err(|_| ValidatorServerError::InternalError)
}

async fn add_to_db(_body: &PostTxBody) -> Result<(), ValidatorServerError> {
    Ok(())
}
