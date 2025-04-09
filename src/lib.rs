use axum::{
    extract::{Json, Path},
    http::StatusCode,
};
use diesel::prelude::*;
use dotenvy::dotenv;
use models::NewSessionCookie;
use std::{env::var, fs};

pub mod models;
pub mod schema;
mod solutions;

#[derive(serde::Deserialize)]
pub struct Params {
    year: u32,
    day: u32,
}

#[derive(serde::Deserialize)]
pub struct SessionCookieBody {
    username: String,
    val: String,
}

fn establish_connection() -> Result<PgConnection, StatusCode> {
    dotenv().ok();

    let database_url = var("DATABASE_URL").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    PgConnection::establish(&database_url).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn request_puzzle_input(year: u32, day: u32) -> Result<String, StatusCode> {
    use self::schema::session_cookies::dsl::*;

    dotenv().ok();

    let session = session_cookies
        .filter(username.eq(&var("CURRENT_USER").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
        .select(val)
        .first::<String>(&mut establish_connection()?)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    reqwest::Client::new()
        .get(format!("https://adventofcode.com/{year}/day/{day}/input"))
        .header(reqwest::header::COOKIE, format!("session={session}"))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .text()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)
}

pub async fn handler(Path(Params { year, day }): Path<Params>) -> Result<String, StatusCode> {
    let filename = format!("input_{year}_{day}.txt");

    Ok(solutions::get_solution(year, day)?(
        if let Ok(input) = fs::read_to_string(&filename) {
            input
        } else {
            let response = request_puzzle_input(year, day).await?;

            if fs::write(&filename, &response).is_err() {
                let _ = fs::remove_file(&filename);
            }

            response
        },
    ))
}

pub async fn session_cookie_handler(
    Json(SessionCookieBody { username, val }): Json<SessionCookieBody>,
) -> Result<StatusCode, StatusCode> {
    diesel::insert_into(crate::schema::session_cookies::table)
        .values(&NewSessionCookie {
            username: &username,
            val: &val,
        })
        .execute(&mut establish_connection()?)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}
