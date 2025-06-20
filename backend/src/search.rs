use actix_web::{web, HttpResponse, Responder};
use rusqlite::params; // Removed Result as it's not directly used here
use serde::{Deserialize, Serialize};
use crate::db::AppState; // Assuming AppState will be in db.rs

#[derive(Deserialize, Debug)]
pub struct SearchParams {
    pub q: String,
}

#[derive(Serialize, Debug)]
pub struct SearchResultItem {
    pub id: i64,
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub location: Option<String>,
}

pub async fn search_entities(
    data: web::Data<AppState>,
    params: web::Query<SearchParams>,
) -> impl Responder {
    let query = format!("%{}%", params.q);
    let conn = data.db.lock().unwrap();
    let mut results = Vec::new();

    // Search Places
    let mut stmt_places = conn
        .prepare("SELECT id, name, description, location FROM places WHERE name LIKE ?1 OR description LIKE ?1")
        .unwrap();
    let places_iter = stmt_places
        .query_map(params![&query], |row| {
            Ok(SearchResultItem {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: "place".to_string(),
                description: row.get(2)?,
                location: row.get(3)?,
            })
        })
        .unwrap();
    for place in places_iter {
        results.push(place.unwrap());
    }

    // Search Accommodations
    let mut stmt_accommodations = conn
        .prepare("SELECT id, name, description, location FROM accommodations WHERE name LIKE ?1 OR description LIKE ?1")
        .unwrap();
    let accommodations_iter = stmt_accommodations
        .query_map(params![&query], |row| {
            Ok(SearchResultItem {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: "accommodation".to_string(),
                description: row.get(2)?,
                location: row.get(3)?,
            })
        })
        .unwrap();
    for acc in accommodations_iter {
        results.push(acc.unwrap());
    }

    // Search Restaurants
    let mut stmt_restaurants = conn
        .prepare("SELECT id, name, description, location FROM restaurants WHERE name LIKE ?1 OR description LIKE ?1")
        .unwrap();
    let restaurants_iter = stmt_restaurants
        .query_map(params![&query], |row| {
            Ok(SearchResultItem {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: "restaurant".to_string(),
                description: row.get(2)?,
                location: row.get(3)?,
            })
        })
        .unwrap();
    for res in restaurants_iter {
        results.push(res.unwrap());
    }

    HttpResponse::Ok().json(results)
}
