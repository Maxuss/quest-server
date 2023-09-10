#![allow(non_snake_case, non_camel_case_types)]

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{ser::SerializeStruct, Serialize};
use thiserror::Error;

macro_rules! error_types {
    ($(
        #[$error_def:meta]
        $status_code:ident $variant_name:ident($(#[$meta:meta])? $inner:path)
    ),* $(,)?) => {
        #[derive(Debug, Error)]
        pub enum ServerError {
            $(
                #[$error_def]
                $variant_name($(#[$meta])? $inner),
            )*
        }

        impl ServerError {
            pub fn variant_name(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant_name(_) => stringify!($variant_name),
                    )*
                }
            }

            pub fn value(&self) -> String {
                match self {
                    $(
                        Self::$variant_name(v) => v.to_string(),
                    )*
                }
            }

            pub fn status_code(&self) -> StatusCode {
                match self {
                    $(
                        Self::$variant_name(_) => StatusCode::$status_code,
                    )*
                }
            }
        }

        impl Serialize for ServerError {
            fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut st = ser.serialize_struct("ServerError", 2)?;
                st.serialize_field("kind", self.variant_name())?;
                st.serialize_field("message", &self.value())?;
                st.end()
            }
        }
    };
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status_code(),
            Possible::<()>::Error {
                success: false,
                error: self,
            },
        )
            .into_response()
    }
}

error_types! {
    #[error("An unknown error has occurred: {0}")]
    INTERNAL_SERVER_ERROR UNKNOWN(String),

    #[error("Anyhow-provoked error has occurred: {0}")]
    BAD_REQUEST PROVOKED(#[from] anyhow::Error),

    #[error("A database provoked error has occurred: {0}")]
    INTERNAL_SERVER_ERROR MONGO_ERROR(#[from] mongodb::error::Error),

    #[error("Invalid data format provided: {0}")]
    BAD_REQUEST INVALID_FORMAT(String),

    #[error("Could not find data: {0}")]
    NOT_FOUND NOT_FOUND(String),

    #[error("An internal IO error has occurred: {0}")]
    INTERNAL_SERVER_ERROR IO_ERROR(#[from] std::io::Error)
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Possible<T: Serialize> {
    Payload {
        success: bool,
        #[serde(flatten)]
        data: T,
    },
    Error {
        success: bool,
        error: ServerError,
    },
}

impl<T: Serialize> IntoResponse for Possible<T> {
    fn into_response(self) -> axum::response::Response {
        match self {
            Possible::Payload { .. } => Json::into_response(Json(self)),
            Possible::Error { success, error } => {
                (error.status_code(), Json(Self::Error { success, error })).into_response()
            }
        }
    }
}

pub type Payload<T> = axum::response::Result<Possible<T>, ServerError>;

#[inline]
pub fn Payload<T: Serialize>(data: T) -> Payload<T> {
    Ok(Possible::Payload {
        success: true,
        data,
    })
}

#[inline]
pub fn Error<T: Serialize>(error: ServerError) -> Payload<T> {
    Err(error)
}
