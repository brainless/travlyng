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
