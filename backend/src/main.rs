use actix_web::{web, App, HttpServer};

// Declare modules
mod accommodations;
mod db;
mod places;
mod restaurants;
mod search;
mod travel_plans;

#[cfg(test)]
mod tests {
    // Explicitly import necessary items
    // use crate::db::AppState; // No longer needed if init_test_db_app is removed or doesn't use it directly
    // use crate::restaurants::{self, Restaurant}; // Moved
    // use crate::places; // No longer needed as no tests remain in main.rs

    // use actix_web::{test, web, App as ActixApp, http::StatusCode}; // Partially moved or covered by specific needs
    // use serde_json::json; // Moved
    // use std::fs; // Moved (as part of init_test_db_app)
    // use std::sync::Mutex; // Moved (as part of init_test_db_app)
    // use rusqlite::Connection; // Moved (as part of init_test_db_app)

    // If init_test_db_app specific to main.rs tests is needed later, it should be redefined here.
    // For now, assuming it was primarily for restaurant tests.
    // If there are other tests in main.rs that need a similar setup,
    // they might need their own version or a shared one if applicable.

    // Example: If there were tests for `places` that needed init_test_db_app,
    // that function would need to be kept or re-created here,
    // and its App setup would include `places` routes.
    // For now, the original init_test_db_app is removed as it was tailored for restaurant tests.

    // Any tests specific to main.rs or other modules (like places, if tested from main) would remain here.
    // For instance, if there was a test_get_places, it would be here.
    // e.g.
    // #[actix_web::test]
    // async fn test_get_places_example() { ... }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db_connection = match db::init_db() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to initialize database: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "DB init failed",
            ));
        }
    };

    let app_state = web::Data::new(db::AppState {
        db: std::sync::Mutex::new(db_connection),
    });

    println!("Starting server at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(
                web::scope("/places")
                    .route("", web::get().to(places::get_places))
                    .route("", web::post().to(places::add_place))
                    .route("/{id}", web::get().to(places::get_place))
                    .route("/{id}", web::put().to(places::update_place))
                    .route("/{id}", web::delete().to(places::delete_place)),
            )
            .service(
                web::scope("/accommodations")
                    .route("", web::get().to(accommodations::get_accommodations))
                    .route("", web::post().to(accommodations::add_accommodation))
                    .route("/{id}", web::get().to(accommodations::get_accommodation))
                    .route("/{id}", web::put().to(accommodations::update_accommodation))
                    .route("/{id}", web::delete().to(accommodations::delete_accommodation)),
            )
            .service(
                web::scope("/restaurants")
                    .route("", web::get().to(restaurants::get_restaurants))
                    .route("", web::post().to(restaurants::add_restaurant))
                    .route("/{id}", web::get().to(restaurants::get_restaurant))
                    .route("/{id}", web::put().to(restaurants::update_restaurant))
                    .route("/{id}", web::delete().to(restaurants::delete_restaurant)),
            )
            .route("/search", web::get().to(search::search_entities))
            .service(
                web::scope("/plans")
                    .route("", web::get().to(travel_plans::get_plans))
                    .route("", web::post().to(travel_plans::add_plan))
                    .route("/{id}", web::get().to(travel_plans::get_plan))
                    .route("/{id}", web::put().to(travel_plans::update_plan))
                    .route("/{id}", web::delete().to(travel_plans::delete_plan))
                    .route("/{plan_id}/items", web::post().to(travel_plans::add_plan_item))
                    .route(
                        "/{plan_id}/items/{item_id}",
                        web::put().to(travel_plans::update_plan_item),
                    )
                    .route(
                        "/{plan_id}/items/{item_id}",
                        web::delete().to(travel_plans::delete_plan_item),
                    ),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
