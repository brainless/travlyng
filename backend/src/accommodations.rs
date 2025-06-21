use actix_web::{web, HttpResponse, Responder};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use crate::db::AppState;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Accommodation {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
}

pub async fn get_accommodations(data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut stmt = match conn.prepare("SELECT id, name, description, location FROM accommodations") {
        Ok(stmt) => stmt,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let accommodation_iter = match stmt.query_map([], |row| {
        Ok(Accommodation {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            location: row.get(3)?,
        })
    }) {
        Ok(iter) => iter,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let mut accommodations = Vec::new();
    for acc in accommodation_iter {
        accommodations.push(acc.unwrap());
    }

    HttpResponse::Ok().json(accommodations)
}

pub async fn add_accommodation(
    data: web::Data<AppState>,
    acc: web::Json<Accommodation>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut new_acc = acc.into_inner();

    match conn.execute(
        "INSERT INTO accommodations (name, description, location) VALUES (?1, ?2, ?3)",
        params![new_acc.name, new_acc.description, new_acc.location],
    ) {
        Ok(updated_rows) => {
            if updated_rows == 0 {
                return HttpResponse::InternalServerError().body("Failed to insert accommodation");
            }
            new_acc.id = Some(conn.last_insert_rowid());
            HttpResponse::Created().json(new_acc)
        }
        Err(e) => {
            eprintln!("Failed to insert accommodation: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to insert accommodation: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, http::StatusCode, HttpRequest, body::to_bytes}; // Added to_bytes
    use rusqlite::Connection;
    use std::sync::Mutex;
    use crate::db::AppState; // Use AppState from db module
    use std::fs;

    // Helper to create an in-memory DB AppState for testing
    fn setup_test_app() -> AppState {
        let conn = Connection::open_in_memory().unwrap();
        // Read schema.sql relative to Cargo.toml (which is in backend directory)
        let schema = fs::read_to_string("../schema.sql")
            .or_else(|_| fs::read_to_string("schema.sql")) // Fallback for when CWD is backend/
            .expect("Should have been able to read the schema.sql file");
        conn.execute_batch(&schema).unwrap();
        AppState { db: Mutex::new(conn) }
    }

    // Helper to create a default HttpRequest
    fn default_req() -> HttpRequest {
        test::TestRequest::default().to_http_request()
    }

    #[actix_web::test]
    async fn test_add_get_accommodation() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();

        // Test Add Accommodation
        let new_acc = Accommodation {
            id: None,
            name: "Test Hotel".to_string(),
            description: Some("A nice place to stay".to_string()),
            location: Some("Test City".to_string()),
        };

        let resp_add = add_accommodation(app_state.clone(), web::Json(new_acc.clone())).await; // Clone new_acc

        let http_resp_add = resp_add.respond_to(&http_req); // Pass http_req
        assert_eq!(http_resp_add.status(), StatusCode::CREATED);

        let body_bytes = match to_bytes(http_resp_add.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read response body for add_accommodation"),
        };
        let added_acc: Accommodation = serde_json::from_slice(&body_bytes).expect("Failed to deserialize body");
        assert!(added_acc.id.is_some());
        assert_eq!(added_acc.name, "Test Hotel");

        let acc_id = added_acc.id.unwrap();

        // Test Get Single Accommodation
        let resp_get = get_accommodation(app_state.clone(), web::Path::from(acc_id)).await;
        let http_resp_get = resp_get.respond_to(&http_req);
        assert_eq!(http_resp_get.status(), StatusCode::OK);
        let body_bytes_get = match to_bytes(http_resp_get.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read response body for get_accommodation"),
        };
        let fetched_acc: Accommodation = serde_json::from_slice(&body_bytes_get).expect("Failed to deserialize body");
        assert_eq!(fetched_acc.id, Some(acc_id));
        assert_eq!(fetched_acc.name, "Test Hotel");

        // Test Get All Accommodations
        let resp_get_all = get_accommodations(app_state.clone()).await;
        let http_resp_get_all = resp_get_all.respond_to(&http_req);
        assert_eq!(http_resp_get_all.status(), StatusCode::OK);
        let body_bytes_get_all = match to_bytes(http_resp_get_all.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read response body for get_accommodations"),
        };
        let all_accs: Vec<Accommodation> = serde_json::from_slice(&body_bytes_get_all).expect("Failed to deserialize body");
        assert_eq!(all_accs.len(), 1);
        assert_eq!(all_accs[0].id, Some(acc_id));
    }

    #[actix_web::test]
    async fn test_update_accommodation() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();

        // First, add an accommodation
        let initial_acc = Accommodation {
            id: None,
            name: "Initial Hotel".to_string(),
            description: Some("Okay".to_string()),
            location: Some("Old Town".to_string()),
        };
        let resp_add = add_accommodation(app_state.clone(), web::Json(initial_acc.clone())).await;
        let resp_add_body_bytes = match to_bytes(resp_add.respond_to(&http_req).into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read response body for add in update_accommodation test"),
        };
        let added_acc: Accommodation = serde_json::from_slice(&resp_add_body_bytes).expect("Failed to deserialize added acc");
        let acc_id = added_acc.id.unwrap();

        // Now, update it
        let payload_for_update = Accommodation {
            id: None,
            name: "Updated Hotel".to_string(),
            description: Some("Much better".to_string()),
            location: Some("New City".to_string()),
        };

        let update_resp = update_accommodation(app_state.clone(), web::Path::from(acc_id), web::Json(payload_for_update)).await;
        let http_update_resp = update_resp.respond_to(&http_req);
        assert_eq!(http_update_resp.status(), StatusCode::OK);
        let update_body_bytes = match to_bytes(http_update_resp.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read response body for update_accommodation"),
        };
        let updated_acc_resp: Accommodation = serde_json::from_slice(&update_body_bytes).expect("Failed to deserialize updated acc");
        assert_eq!(updated_acc_resp.id, Some(acc_id));
        assert_eq!(updated_acc_resp.name, "Updated Hotel");
        assert_eq!(updated_acc_resp.description, Some("Much better".to_string()));

        // Verify by getting the accommodation again
        let get_resp = get_accommodation(app_state.clone(), web::Path::from(acc_id)).await;
        let http_get_resp = get_resp.respond_to(&http_req);
        let get_body_bytes = match to_bytes(http_get_resp.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read response body for get_accommodation after update"),
        };
        let fetched_updated_acc: Accommodation = serde_json::from_slice(&get_body_bytes).expect("Failed to deserialize fetched updated acc");
        assert_eq!(fetched_updated_acc.name, "Updated Hotel");
    }

    #[actix_web::test]
    async fn test_delete_accommodation() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();

        // Add an accommodation to delete
        let acc_to_delete = Accommodation {
            id: None,
            name: "To Be Deleted".to_string(),
            description: None,
            location: None,
        };
        let resp_add = add_accommodation(app_state.clone(), web::Json(acc_to_delete.clone())).await;
        let resp_add_body_bytes = match to_bytes(resp_add.respond_to(&http_req).into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read response body for add_accommodation in delete test"),
        };
        let added_acc: Accommodation = serde_json::from_slice(&resp_add_body_bytes).expect("Failed to deserialize acc for delete");
        let acc_id = added_acc.id.unwrap();

        // Delete the accommodation
        let delete_resp = delete_accommodation(app_state.clone(), web::Path::from(acc_id)).await;
        let http_delete_resp = delete_resp.respond_to(&http_req);
        assert_eq!(http_delete_resp.status(), StatusCode::NO_CONTENT);

        // Try to get the deleted accommodation (should be 404)
        let get_resp_after_delete = get_accommodation(app_state.clone(), web::Path::from(acc_id)).await;
        let http_get_resp_after_delete = get_resp_after_delete.respond_to(&http_req);
        assert_eq!(http_get_resp_after_delete.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_get_accommodation_not_found() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();
        let resp = get_accommodation(app_state.clone(), web::Path::from(999_i64)).await;
        let http_resp = resp.respond_to(&http_req);
        assert_eq!(http_resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_update_accommodation_not_found() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();
        let updated_details = Accommodation {
            id: None,
            name: "Non Existent".to_string(),
            description: Some("This should not be found".to_string()),
            location: Some("Nowhere".to_string()),
        };
        let resp = update_accommodation(app_state.clone(), web::Path::from(999_i64), web::Json(updated_details)).await;
        let http_resp = resp.respond_to(&http_req);
        assert_eq!(http_resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_delete_accommodation_not_found() {
        let app_state = web::Data::new(setup_test_app());
        let http_req = default_req();
        let resp = delete_accommodation(app_state.clone(), web::Path::from(999_i64)).await;
        let http_resp = resp.respond_to(&http_req);
        assert_eq!(http_resp.status(), StatusCode::NOT_FOUND);
    }
}

pub async fn get_accommodation(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
    let acc_id = path.into_inner();
    let conn = data.db.lock().unwrap();

    match conn.query_row(
        "SELECT id, name, description, location FROM accommodations WHERE id = ?1",
        params![acc_id],
        |row| {
            Ok(Accommodation {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                location: row.get(3)?,
            })
        },
    ) {
        Ok(acc) => HttpResponse::Ok().json(acc),
        Err(rusqlite::Error::QueryReturnedNoRows) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn update_accommodation(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    acc_data: web::Json<Accommodation>,
) -> impl Responder {
    let acc_id = path.into_inner();
    let conn = data.db.lock().unwrap();
    let acc = acc_data.into_inner();

    match conn.execute(
        "UPDATE accommodations SET name = ?1, description = ?2, location = ?3 WHERE id = ?4",
        params![acc.name, acc.description, acc.location, acc_id],
    ) {
        Ok(updated_rows) => {
            if updated_rows == 0 {
                HttpResponse::NotFound().finish()
            } else {
                HttpResponse::Ok().json(Accommodation {
                    id: Some(acc_id),
                    name: acc.name,
                    description: acc.description,
                    location: acc.location,
                })
            }
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn delete_accommodation(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
    let acc_id = path.into_inner();
    let conn = data.db.lock().unwrap();

    match conn.execute("DELETE FROM accommodations WHERE id = ?1", params![acc_id]) {
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
