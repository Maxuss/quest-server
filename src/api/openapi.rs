use axum::response::{IntoResponse, Response};

pub async fn openapi_yml_route() -> &'static str {
    include_str!("openapi.yml")
}

pub async fn openapi_route() -> Response {
    let html = r#"
    <!DOCTYPE html>
    <html>
        <head>
            <title>Quest API spec</title>
            <meta charset="utf-8"/>
            <meta name="viewport" content="width=device-width, initial-scale=1"/>
            <link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet"/>
    
            <style>
                body {
                    margin: 0;
                    padding: 0;
                }
            </style>
        </head>
    
        <body>
            <redoc spec-url="../resources/openapi.yml">
            </redoc>
            <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
        </body>
    </html>
    "#;
    (
        [(
            axum::http::header::CONTENT_TYPE,
            axum::http::header::HeaderValue::from_static("text/html"),
        )],
        html,
    )
        .into_response()
}
