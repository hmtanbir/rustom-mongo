use crate::errors::AppError;
use axum::{
    Json, async_trait,
    extract::{FromRequest, Request},
};
use serde::de::DeserializeOwned;

pub struct AppJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for AppJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(value) => Ok(AppJson(value.0)),
            Err(rejection) => {
                use axum::extract::rejection::JsonRejection;
                use std::error::Error;

                let msg = match &rejection {
                    JsonRejection::JsonDataError(err) => {
                        let mut deepest_err: &dyn Error = err;
                        while let Some(source) = deepest_err.source() {
                            deepest_err = source;
                        }
                        let mut error_msg = deepest_err.to_string();
                        if let Some(idx) = error_msg.find(" at line ") {
                            error_msg.truncate(idx);
                        }
                        error_msg
                    }
                    JsonRejection::JsonSyntaxError(err) => {
                        let mut deepest_err: &dyn Error = err;
                        while let Some(source) = deepest_err.source() {
                            deepest_err = source;
                        }
                        let mut error_msg = deepest_err.to_string();
                        if let Some(idx) = error_msg.find(" at line ") {
                            error_msg.truncate(idx);
                        }
                        error_msg
                    }
                    _ => rejection.to_string(),
                };

                tracing::debug!("Json parse error: {:?}", rejection);
                Err(AppError::InvalidInput(msg))
            }
        }
    }
}
