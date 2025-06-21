use actix_web::{web, HttpResponse, Responder};
use rusqlite::params;
use serde::{Deserialize, Serialize};
 // Although Connection is wrapped in Mutex in AppState, individual handlers might need Mutex for other shared resources if requirements change. It's also good for consistency.
use crate::db::AppState;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Place {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
}

pub async fn get_places(data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut stmt = match conn.prepare("SELECT id, name, description, location FROM places") {
        Ok(stmt) => stmt,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let place_iter = match stmt.query_map([], |row| {
        Ok(Place {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            location: row.get(3)?,
        })
    }) {
        Ok(place_iter) => place_iter,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let mut places = Vec::new();
    for place in place_iter {
        places.push(place.unwrap());
    }

    HttpResponse::Ok().json(places)
}

pub async fn add_place(data: web::Data<AppState>, place: web::Json<Place>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut new_place = place.into_inner();

    match conn.execute(
        "INSERT INTO places (name, description, location) VALUES (?1, ?2, ?3)",
        params![new_place.name, new_place.description, new_place.location],
    ) {
        Ok(updated_rows) => {
            if updated_rows == 0 {
                return HttpResponse::InternalServerError().body("Failed to insert place");
            }
            new_place.id = Some(conn.last_insert_rowid());
            HttpResponse::Created().json(new_place)
        }
        Err(e) => {
            eprintln!("Failed to insert place: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to insert place: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, http::StatusCode, HttpRequest, body::to_bytes};
    use rusqlite::Connection;
    use std::sync::Mutex;
    use crate::db::AppState;
    use std::fs;

    // Helper to create an in-memory DB AppState for testing
    fn setup_test_app() -> AppState {
        let conn = Connection::open_in_memory().unwrap();
        let schema = fs::read_to_string("../schema.sql")
            .or_else(|_| fs::read_to_string("schema.sql"))
            .expect("Should have been able to read the schema.sql file");
        conn.execute_batch(&schema).unwrap();
        AppState { db: Mutex::new(conn) }
    }

    // Helper to create a default HttpRequest
    fn default_req() -> HttpRequest {
        test::TestRequest::default().to_http_request()
    }

    #[actix_web::test]
    async fn test_add_get_place() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();

        // Test Add Place
        let new_place = Place {
            id: None,
            name: "Test Landmark".to_string(),
            description: Some("A significant place".to_string()),
            location: Some("Test City Center".to_string()),
        };
        let resp_add = add_place(app_state.clone(), web::Json(new_place.clone())).await;

        let http_resp_add = resp_add.respond_to(&http_req);
        assert_eq!(http_resp_add.status(), StatusCode::CREATED);

        let body_bytes_add = match to_bytes(http_resp_add.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for add_place"),
        };
        let added_place: Place = serde_json::from_slice(&body_bytes_add).expect("Failed to deserialize added place");
        assert!(added_place.id.is_some());
        assert_eq!(added_place.name, "Test Landmark");

        let place_id = added_place.id.unwrap();

        // Test Get Single Place
        let resp_get = get_place(app_state.clone(), web::Path::from(place_id)).await;
        let http_resp_get = resp_get.respond_to(&http_req);
        assert_eq!(http_resp_get.status(), StatusCode::OK);
        let body_bytes_get = match to_bytes(http_resp_get.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for get_place"),
        };
        let fetched_place: Place = serde_json::from_slice(&body_bytes_get).expect("Failed to deserialize fetched place");
        assert_eq!(fetched_place.id, Some(place_id));
        assert_eq!(fetched_place.name, "Test Landmark");

        // Test Get All Places
        let resp_get_all = get_places(app_state.clone()).await;
        let http_resp_get_all = resp_get_all.respond_to(&http_req);
        assert_eq!(http_resp_get_all.status(), StatusCode::OK);
        let body_bytes_get_all = match to_bytes(http_resp_get_all.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for get_places"),
        };
        let all_places: Vec<Place> = serde_json::from_slice(&body_bytes_get_all).expect("Failed to deserialize all places");
        assert_eq!(all_places.len(), 1);
        assert_eq!(all_places[0].id, Some(place_id));
    }

    #[actix_web::test]
    async fn test_update_place() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();

        let initial_place = Place {
            id: None,
            name: "Old Cafe".to_string(),
            description: Some("Vintage style".to_string()),
            location: Some("Historic District".to_string()),
        };
        let add_resp = add_place(app_state.clone(), web::Json(initial_place.clone())).await;
        let add_body_bytes = match to_bytes(add_resp.respond_to(&http_req).into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for add_place in update_place test"),
        };
        let added_place: Place = serde_json::from_slice(&add_body_bytes).expect("Failed to deserialize added place in update");
        let place_id = added_place.id.unwrap();

        let updated_details = Place {
            id: None,
            name: "New Modern Cafe".to_string(),
            description: Some("Sleek and new".to_string()),
            location: Some("Downtown".to_string()),
        };

        let update_resp = update_place(app_state.clone(), web::Path::from(place_id), web::Json(updated_details.clone())).await; // Clone updated_details
        let http_update_resp = update_resp.respond_to(&http_req);
        assert_eq!(http_update_resp.status(), StatusCode::OK);
        let update_body_bytes = match to_bytes(http_update_resp.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for update_place"),
        };
        let updated_place_resp: Place = serde_json::from_slice(&update_body_bytes).expect("Failed to deserialize updated place");
        assert_eq!(updated_place_resp.id, Some(place_id));
        assert_eq!(updated_place_resp.name, "New Modern Cafe");

        // Verify update by fetching again
        let get_resp = get_place(app_state.clone(), web::Path::from(place_id)).await;
        let http_get_resp = get_resp.respond_to(&http_req);
        let get_body_bytes = match to_bytes(http_get_resp.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for get_place after update"),
        };
        let fetched_updated_place: Place = serde_json::from_slice(&get_body_bytes).expect("Failed to deserialize fetched place after update");
        assert_eq!(fetched_updated_place.name, "New Modern Cafe");
    }

    #[actix_web::test]
    async fn test_delete_place() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();

        let place_to_delete = Place {
            id: None,
            name: "Temporary Site".to_string(),
            description: None,
            location: None,
        };
        let add_resp = add_place(app_state.clone(), web::Json(place_to_delete.clone())).await;
        let add_body_bytes = match to_bytes(add_resp.respond_to(&http_req).into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for add_place in delete_place test"),
        };
        let added_place: Place = serde_json::from_slice(&add_body_bytes).expect("Failed to deserialize place for delete");
        let place_id = added_place.id.unwrap();

        let delete_resp = delete_place(app_state.clone(), web::Path::from(place_id)).await;
        let http_delete_resp = delete_resp.respond_to(&http_req);
        assert_eq!(http_delete_resp.status(), StatusCode::NO_CONTENT);

        let get_resp_after_delete = get_place(app_state.clone(), web::Path::from(place_id)).await;
        let http_get_resp_after_delete = get_resp_after_delete.respond_to(&http_req);
        assert_eq!(http_get_resp_after_delete.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_get_place_not_found() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();
        let resp = get_place(app_state.clone(), web::Path::from(777_i64)).await;
        let http_resp = resp.respond_to(&http_req);
        assert_eq!(http_resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_update_place_not_found() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();
        let updated_details = Place {
            id: None,
            name: "Ghost Place".to_string(),
            description: Some("You can't see me".to_string()),
            location: Some("Limbo".to_string()),
        };
        let resp = update_place(app_state.clone(), web::Path::from(777_i64), web::Json(updated_details.clone())).await; // Clone updated_details
        let http_resp = resp.respond_to(&http_req);
        assert_eq!(http_resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_delete_place_not_found() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();
        let resp = delete_place(app_state.clone(), web::Path::from(777_i64)).await;
        let http_resp = resp.respond_to(&http_req);
        assert_eq!(http_resp.status(), StatusCode::NOT_FOUND);
    }
}

pub async fn get_place(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
    let place_id = path.into_inner();
    let conn = data.db.lock().unwrap();

    match conn.query_row(
        "SELECT id, name, description, location FROM places WHERE id = ?1",
        params![place_id],
        |row| {
            Ok(Place {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                location: row.get(3)?,
            })
        },
    ) {
        Ok(place) => HttpResponse::Ok().json(place),
        Err(rusqlite::Error::QueryReturnedNoRows) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn update_place(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    place_data: web::Json<Place>,
) -> impl Responder {
    let place_id = path.into_inner();
    let conn = data.db.lock().unwrap();
    let place = place_data.into_inner();

    match conn.execute(
        "UPDATE places SET name = ?1, description = ?2, location = ?3 WHERE id = ?4",
        params![place.name, place.description, place.location, place_id],
    ) {
        Ok(updated_rows) => {
            if updated_rows == 0 {
                HttpResponse::NotFound().finish()
            } else {
                HttpResponse::Ok().json(Place {
                    id: Some(place_id),
                    name: place.name,
                    description: place.description,
                    location: place.location,
                })
            }
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn delete_place(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
    let place_id = path.into_inner();
    let conn = data.db.lock().unwrap();

    match conn.execute("DELETE FROM places WHERE id = ?1", params![place_id]) {
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
