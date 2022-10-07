#![allow(non_snake_case, non_camel_case_types)]

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{ser::SerializeStruct, Serialize};
use thiserror::Error;

macro_rules! error_types {
    ($(
        #[$error_def:meta]
        $variant_name:ident($(#[$meta:meta])? $inner:path)
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
            StatusCode::BAD_REQUEST,
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
    UNKNOWN(String),

    #[error("Anyhow provoked error has occurred: {0}")]
    PROVOKED(#[from] anyhow::Error),

    #[error("An SQL provoked error has occurred: {0}")]
    SQL_ERROR(#[from] sqlx::Error)
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
        Json::into_response(Json(self))
    }
}

pub type Payload<T> = axum::response::Result<Possible<T>, ServerError>;

pub fn Payload<T: Serialize>(data: T) -> Payload<T> {
    Ok(Possible::Payload {
        success: true,
        data,
    })
}

pub fn Error<T: Serialize>(error: ServerError) -> Payload<T> {
    Err(error)
}
