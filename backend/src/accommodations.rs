use actix_web::{web, HttpResponse, Responder};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use crate::db::AppState;

#[derive(Serialize, Deserialize, Debug)]
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
