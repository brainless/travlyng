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
    use crate::db::AppState;
    use crate::restaurants::{
        self, Restaurant, // Import module for handlers, and struct
    };
    use crate::places; // Import places module for its handlers

    use actix_web::{test, web, App as ActixApp, http::StatusCode};
    use serde_json::json;
    use std::fs;
    use std::sync::Mutex;
    use rusqlite::Connection; // Import Connection for init_test_db_app

    // Helper function to initialize the app with an in-memory DB for tests
    async fn init_test_db_app() -> ActixApp<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        // For tests, schema.sql is expected to be in the root of the 'backend' crate
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB for test");
        let schema = fs::read_to_string("schema.sql")
            .expect("Failed to read schema.sql for tests. Ensure it's in backend/ directory.");
        conn.execute_batch(&schema).expect("Failed to execute schema on in-memory DB");

        let app_state = web::Data::new(AppState { db: Mutex::new(conn) });

        ActixApp::new()
            .app_data(app_state.clone())
            .service(
                web::scope("/restaurants")
                    .route("", web::get().to(restaurants::get_restaurants))
                    .route("", web::post().to(restaurants::add_restaurant))
                    .route("/{id}", web::get().to(restaurants::get_restaurant))
                    .route("/{id}", web::put().to(restaurants::update_restaurant))
                    .route("/{id}", web::delete().to(restaurants::delete_restaurant)),
            )
            .service(web::scope("/places").route("", web::get().to(places::get_places)))
    }

    #[actix_web::test]
    async fn test_add_restaurant() {
        let app_service = test::init_service(init_test_db_app().await).await;
        let new_restaurant_payload = json!({
            "name": "Test Cafe",
            "description": "A lovely place for coffee",
            "location": "123 Test St"
        });

        let req = test::TestRequest::post()
            .uri("/restaurants")
            .set_json(&new_restaurant_payload)
            .to_request();

        let resp = test::call_service(&app_service, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Expected 201 Created");

        let body: Restaurant = test::read_body_json(resp).await;
        assert_eq!(body.name, "Test Cafe");
        assert_eq!(body.description.as_deref(), Some("A lovely place for coffee"));
        assert!(body.id.is_some(), "Expected created restaurant to have an ID");
    }

    #[actix_web::test]
    async fn test_get_restaurants_empty_and_then_one() {
        let app_service = test::init_service(init_test_db_app().await).await;

        let req_empty = test::TestRequest::get().uri("/restaurants").to_request();
        let resp_empty = test::call_service(&app_service, req_empty).await;
        assert_eq!(resp_empty.status(), StatusCode::OK);
        let body_empty: Vec<Restaurant> = test::read_body_json(resp_empty).await;
        assert!(body_empty.is_empty(), "Expected empty list of restaurants initially");

        let new_restaurant_payload = json!({
            "name": "Pizza Place",
            "description": "Best pizza in town",
            "location": "456 Main Ave"
        });
        let add_req = test::TestRequest::post()
            .uri("/restaurants")
            .set_json(&new_restaurant_payload)
            .to_request();
        let add_resp = test::call_service(&app_service, add_req).await;
        assert_eq!(add_resp.status(), StatusCode::CREATED);
        let added_restaurant: Restaurant = test::read_body_json(add_resp).await;

        let req_filled = test::TestRequest::get().uri("/restaurants").to_request();
        let resp_filled = test::call_service(&app_service, req_filled).await;
        assert_eq!(resp_filled.status(), StatusCode::OK);
        let body_filled: Vec<Restaurant> = test::read_body_json(resp_filled).await;
        assert_eq!(body_filled.len(), 1, "Expected one restaurant after adding");
        assert_eq!(body_filled[0].name, "Pizza Place");
        assert_eq!(body_filled[0].id, added_restaurant.id);
    }

    #[actix_web::test]
    async fn test_get_specific_restaurant() {
        let app_service = test::init_service(init_test_db_app().await).await;

        let new_restaurant_payload = json!({"name": "Sushi Spot", "description": "Fresh sushi", "location": "789 Bay Rd"});
        let add_req = test::TestRequest::post().uri("/restaurants").set_json(&new_restaurant_payload).to_request();
        let add_resp = test::call_service(&app_service, add_req).await;
        assert_eq!(add_resp.status(), StatusCode::CREATED);
        let added_restaurant: Restaurant = test::read_body_json(add_resp).await;
        let restaurant_id = added_restaurant.id.unwrap();

        let get_req = test::TestRequest::get().uri(&format!("/restaurants/{}", restaurant_id)).to_request();
        let get_resp = test::call_service(&app_service, get_req).await;
        assert_eq!(get_resp.status(), StatusCode::OK);
        let fetched_restaurant: Restaurant = test::read_body_json(get_resp).await;
        assert_eq!(fetched_restaurant.id, Some(restaurant_id));
        assert_eq!(fetched_restaurant.name, "Sushi Spot");

        let get_non_existent_req = test::TestRequest::get().uri("/restaurants/9999").to_request();
        let get_non_existent_resp = test::call_service(&app_service, get_non_existent_req).await;
        assert_eq!(get_non_existent_resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_update_restaurant() {
        let app_service = test::init_service(init_test_db_app().await).await;

        let initial_payload = json!({"name": "Old Grill", "description": "Steaks and stuff", "location": "1st Street"});
        let add_req = test::TestRequest::post().uri("/restaurants").set_json(&initial_payload).to_request();
        let add_resp = test::call_service(&app_service, add_req).await;
        assert_eq!(add_resp.status(), StatusCode::CREATED);
        let added_restaurant: Restaurant = test::read_body_json(add_resp).await;
        let restaurant_id = added_restaurant.id.unwrap();

        let updated_payload = json!({
            "name": "New Vegan Grill",
            "description": "Plant-based goodness",
            "location": "2nd Avenue"
        });
        let update_req = test::TestRequest::put()
            .uri(&format!("/restaurants/{}", restaurant_id))
            .set_json(&updated_payload)
            .to_request();
        let update_resp = test::call_service(&app_service, update_req).await;
        assert_eq!(update_resp.status(), StatusCode::OK);
        let updated_restaurant_body: Restaurant = test::read_body_json(update_resp).await;
        assert_eq!(updated_restaurant_body.name, "New Vegan Grill");
        assert_eq!(updated_restaurant_body.description.as_deref(), Some("Plant-based goodness"));
        assert_eq!(updated_restaurant_body.id, Some(restaurant_id));

        let get_req = test::TestRequest::get().uri(&format!("/restaurants/{}", restaurant_id)).to_request();
        let get_resp = test::call_service(&app_service, get_req).await;
        let fetched_restaurant: Restaurant = test::read_body_json(get_resp).await;
        assert_eq!(fetched_restaurant.name, "New Vegan Grill");

        let update_non_existent_req = test::TestRequest::put()
            .uri("/restaurants/8888")
            .set_json(&updated_payload)
            .to_request();
        let update_non_existent_resp = test::call_service(&app_service, update_non_existent_req).await;
        assert_eq!(update_non_existent_resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_delete_restaurant() {
        let app_service = test::init_service(init_test_db_app().await).await;

        let payload = json!({"name": "To Be Deleted", "description": "Short lived", "location": "Nowhere"});
        let add_req = test::TestRequest::post().uri("/restaurants").set_json(&payload).to_request();
        let add_resp = test::call_service(&app_service, add_req).await;
        assert_eq!(add_resp.status(), StatusCode::CREATED);
        let added_restaurant: Restaurant = test::read_body_json(add_resp).await;
        let restaurant_id = added_restaurant.id.unwrap();

        let delete_req = test::TestRequest::delete().uri(&format!("/restaurants/{}", restaurant_id)).to_request();
        let delete_resp = test::call_service(&app_service, delete_req).await;
        assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

        let get_req = test::TestRequest::get().uri(&format!("/restaurants/{}", restaurant_id)).to_request();
        let get_resp = test::call_service(&app_service, get_req).await;
        assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

        let delete_non_existent_req = test::TestRequest::delete().uri("/restaurants/7777").to_request();
        let delete_non_existent_resp = test::call_service(&app_service, delete_non_existent_req).await;
        assert_eq!(delete_non_existent_resp.status(), StatusCode::NOT_FOUND);
    }
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
