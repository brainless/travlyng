use actix_web::{web, HttpResponse, Responder};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use crate::db::AppState;

#[derive(Serialize, Deserialize, Debug)]
pub struct Restaurant {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
}

// Handler functions for Restaurants
pub async fn get_restaurants(data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut stmt = match conn.prepare("SELECT id, name, description, location FROM restaurants") {
        Ok(stmt) => stmt,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let restaurant_iter = match stmt.query_map([], |row| {
        Ok(Restaurant {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            location: row.get(3)?,
        })
    }) {
        Ok(iter) => iter,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let mut restaurants = Vec::new();
    for res in restaurant_iter {
        restaurants.push(res.unwrap());
    }

    HttpResponse::Ok().json(restaurants)
}

pub async fn add_restaurant(
    data: web::Data<AppState>,
    res: web::Json<Restaurant>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut new_res = res.into_inner();

    match conn.execute(
        "INSERT INTO restaurants (name, description, location) VALUES (?1, ?2, ?3)",
        params![new_res.name, new_res.description, new_res.location],
    ) {
        Ok(updated_rows) => {
            if updated_rows == 0 {
                return HttpResponse::InternalServerError().body("Failed to insert restaurant");
            }
            new_res.id = Some(conn.last_insert_rowid());
            HttpResponse::Created().json(new_res)
        }
        Err(e) => {
            eprintln!("Failed to insert restaurant: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to insert restaurant: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{test, web, App as ActixApp};
    use rusqlite::Connection;
    use std::fs;
    use std::sync::Mutex;
    use crate::db::AppState;
    use crate::restaurants; // Import the parent module

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
        let schema = fs::read_to_string("schema.sql") // Corrected path for schema.sql
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
            // Note: If other routes are needed by tests in this module, they should be added here.
            // For now, only restaurant routes are included as per the context of these tests.
    }

    use actix_web::{http::StatusCode};
    use serde_json::json;
    use super::Restaurant; // Import Restaurant from parent module

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

pub async fn get_restaurant(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
    let res_id = path.into_inner();
    let conn = data.db.lock().unwrap();

    match conn.query_row(
        "SELECT id, name, description, location FROM restaurants WHERE id = ?1",
        params![res_id],
        |row| {
            Ok(Restaurant {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                location: row.get(3)?,
            })
        },
    ) {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(rusqlite::Error::QueryReturnedNoRows) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn update_restaurant(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    res_data: web::Json<Restaurant>,
) -> impl Responder {
    let res_id = path.into_inner();
    let conn = data.db.lock().unwrap();
    let res = res_data.into_inner();

    match conn.execute(
        "UPDATE restaurants SET name = ?1, description = ?2, location = ?3 WHERE id = ?4",
        params![res.name, res.description, res.location, res_id],
    ) {
        Ok(updated_rows) => {
            if updated_rows == 0 {
                HttpResponse::NotFound().finish()
            } else {
                HttpResponse::Ok().json(Restaurant {
                    id: Some(res_id),
                    name: res.name,
                    description: res.description,
                    location: res.location,
                })
            }
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn delete_restaurant(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
    let res_id = path.into_inner();
    let conn = data.db.lock().unwrap();

    match conn.execute("DELETE FROM restaurants WHERE id = ?1", params![res_id]) {
        Ok(deleted_rows) => {
            if deleted_rows == 0 {
                HttpResponse::NotFound().finish()
            } else {
                HttpResponse::NoContent().finish()
            }
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
