use actix_web::{HttpResponse, web::Data};

use crate::{server::error::ValidatorServerError, types::DbPool};

type X = (String,);

struct Y {}

pub async fn get_tx(db: Data<DbPool>) -> actix_web::Result<HttpResponse, ValidatorServerError> {
    let mut conn = db.acquire()
        .await
        .unwrap();

    sqlx::query_as!(
        X,
        "SELECT id FROM transactions WHERE id = ?",
        ""
    )
    .fetch_one(&conn);

    if let Ok(r) = res {
        Ok(HttpResponse::Ok().json(r))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}