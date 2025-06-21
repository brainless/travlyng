use actix_web::{web, HttpResponse, Responder};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
 // Although Connection is wrapped in Mutex in AppState, individual handlers might need Mutex for other shared resources if requirements change. It's also good for consistency.
use crate::db::AppState;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct Place {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
}

#[utoipa::path(
    get,
    path = "/places",
    responses(
        (status = 200, description = "List all places", body = [Place])
    )
)]
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

#[utoipa::path(
    post,
    path = "/places",
    request_body = Place,
    responses(
        (status = 201, description = "Place created successfully", body = Place),
        (status = 500, description = "Failed to insert place")
    )
)]
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

#[utoipa::path(
    get,
    path = "/places/{id}",
    params(
        ("id" = i64, Path, description = "Place id")
    ),
    responses(
        (status = 200, description = "Found place", body = Place),
        (status = 404, description = "Place not found"),
        (status = 500, description = "Internal server error")
    )
)]
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

#[utoipa::path(
    put,
    path = "/places/{id}",
    params(
        ("id" = i64, Path, description = "Place id")
    ),
    request_body = Place,
    responses(
        (status = 200, description = "Place updated successfully", body = Place),
        (status = 404, description = "Place not found"),
        (status = 500, description = "Internal server error")
    )
)]
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

#[utoipa::path(
    delete,
    path = "/places/{id}",
    params(
        ("id" = i64, Path, description = "Place id")
    ),
    responses(
        (status = 204, description = "Place deleted successfully"),
        (status = 404, description = "Place not found"),
        (status = 500, description = "Internal server error")
    )
)]
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
