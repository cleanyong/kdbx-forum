mod args;
mod db;
mod dto;
mod routes;
mod state;

use std::error::Error;

use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;

use args::Args;
use db::{build_db_key, open_database};
use routes::{
    create_reply, create_thread, get_thread_detail, index, list_categories, list_threads_in_category,
};
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let key = build_db_key(args.password.clone(), &args.keyfile)?;
    let db = open_database(&args.database, &key)?;
    let state = AppState::new(db, args.database.clone(), key);

    let app = Router::new()
        .route("/", get(index))
        .route("/categories", get(list_categories))
        .route("/categories/:id/threads", get(list_threads_in_category))
        .route("/threads/:id", get(get_thread_detail))
        .route("/threads", post(create_thread))
        .route("/threads/:id/replies", post(create_reply))
        .with_state(state);

    let addr = &args.listen;
    println!("Serving kdbx-forum on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
