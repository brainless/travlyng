use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Debug)]
struct Place {
    id: Option<i64>,
    name: String,
    description: Option<String>,
    location: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Accommodation {
    id: Option<i64>,
    name: String,
    description: Option<String>,
    location: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Restaurant {
    id: Option<i64>,
    name: String,
    description: Option<String>,
    location: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)] // Added Clone
struct PlanItem {
    id: Option<i64>,
    plan_id: i64,
    entity_type: String,
    entity_id: i64,
    visit_date: Option<String>,
    notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PlanItemRequest { // For POST/PUT requests for PlanItem
    entity_type: String,
    entity_id: i64,
    visit_date: Option<String>,
    notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TravelPlan {
    id: Option<i64>,
    name: String,
    start_date: Option<String>,
    end_date: Option<String>,
    items: Option<Vec<PlanItem>>, // Populated when fetching a single plan
}

#[derive(Deserialize, Debug)]
struct SearchParams {
    q: String,
}

#[derive(Serialize, Debug)]
struct SearchResultItem {
    id: i64,
    name: String,
    entity_type: String,
    description: Option<String>,
    location: Option<String>,
}

// Database initialization (moved Data struct here for simplicity)
struct AppState {
    db: Mutex<Connection>,
}

fn init_db() -> Result<Connection> {
    let conn = Connection::open("travel_planner.db")?;
    let schema = fs::read_to_string("backend/schema.sql")
        .expect("Should have been able to read the file");
    conn.execute_batch(&schema)?;
    println!("Database initialized successfully.");
    Ok(conn)
}

async fn get_places(data: web::Data<AppState>) -> impl Responder {
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

async fn add_place(data: web::Data<AppState>, place: web::Json<Place>) -> impl Responder {
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

async fn get_place(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
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

async fn update_place(
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

async fn delete_place(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db_connection = match init_db() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to initialize database: {}", e);
            // Exit if DB initialization fails
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "DB init failed"));
        }
    };

    // Wrap the connection in a Mutex and Arc for thread-safe sharing
    let app_state = web::Data::new(AppState {
        db: Mutex::new(db_connection),
    });

    println!("Starting server at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(
                web::scope("/places")
                    .route("", web::get().to(get_places))
                    .route("", web::post().to(add_place))
                    .route("/{id}", web::get().to(get_place))
                    .route("/{id}", web::put().to(update_place))
                    .route("/{id}", web::delete().to(delete_place)),
            )
            .service(
                web::scope("/accommodations")
                    .route("", web::get().to(get_accommodations))
                    .route("", web::post().to(add_accommodation))
                    .route("/{id}", web::get().to(get_accommodation))
                    .route("/{id}", web::put().to(update_accommodation))
                    .route("/{id}", web::delete().to(delete_accommodation)),
            )
                    .route("/{id}", web::put().to(update_restaurant))
                    .route("/{id}", web::delete().to(delete_restaurant)),
            )
            .route("/search", web::get().to(search_entities))
            .service(
                web::scope("/plans")
                    .route("", web::get().to(get_plans))
                    .route("", web::post().to(add_plan))
                    .route("/{id}", web::get().to(get_plan))
                    .route("/{id}", web::put().to(update_plan))
                    .route("/{id}", web::delete().to(delete_plan))
                    .route("/{plan_id}/items", web::post().to(add_plan_item))
                    .route(
                        "/{plan_id}/items/{item_id}",
                        web::put().to(update_plan_item),
                    )
                    .route(
                        "/{plan_id}/items/{item_id}",
                        web::delete().to(delete_plan_item),
                    ),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

async fn search_entities(
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

// --- TravelPlan Handlers ---

async fn get_plans(data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, start_date, end_date FROM travel_plans")
        .unwrap();
    let plan_iter = stmt
        .query_map([], |row| {
            Ok(TravelPlan {
                id: row.get(0)?,
                name: row.get(1)?,
                start_date: row.get(2)?,
                end_date: row.get(3)?,
                items: None, // Not fetching items for the list view
            })
        })
        .unwrap();

    let mut plans = Vec::new();
    for plan in plan_iter {
        match plan {
            Ok(p) => plans.push(p),
            Err(e) => {
                eprintln!("Error fetching plan: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    }
    HttpResponse::Ok().json(plans)
}

async fn add_plan(data: web::Data<AppState>, plan_data: web::Json<TravelPlan>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut plan = plan_data.into_inner();

    match conn.execute(
        "INSERT INTO travel_plans (name, start_date, end_date) VALUES (?1, ?2, ?3)",
        params![plan.name, plan.start_date, plan.end_date],
    ) {
        Ok(_) => {
            plan.id = Some(conn.last_insert_rowid());
            HttpResponse::Created().json(plan)
        }
        Err(e) => {
            eprintln!("Failed to insert travel_plan: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn get_plan(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
    let plan_id = path.into_inner();
    let conn = data.db.lock().unwrap();

    let mut plan = match conn.query_row(
        "SELECT id, name, start_date, end_date FROM travel_plans WHERE id = ?1",
        params![plan_id],
        |row| {
            Ok(TravelPlan {
                id: row.get(0)?,
                name: row.get(1)?,
                start_date: row.get(2)?,
                end_date: row.get(3)?,
                items: Some(Vec::new()), // Initialize items vector
            })
        },
    ) {
        Ok(p) => p,
        Err(rusqlite::Error::QueryReturnedNoRows) => return HttpResponse::NotFound().finish(),
        Err(e) => {
            eprintln!("Failed to fetch travel_plan: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut stmt_items = conn
        .prepare("SELECT id, plan_id, entity_type, entity_id, visit_date, notes FROM plan_items WHERE plan_id = ?1")
        .unwrap();
    let item_iter = stmt_items
        .query_map(params![plan_id], |row| {
            Ok(PlanItem {
                id: row.get(0)?,
                plan_id: row.get(1)?,
                entity_type: row.get(2)?,
                entity_id: row.get(3)?,
                visit_date: row.get(4)?,
                notes: row.get(5)?,
            })
        })
        .unwrap();

    for item_result in item_iter {
        match item_result {
            Ok(item) => {
                if let Some(ref mut items_vec) = plan.items {
                    items_vec.push(item);
                }
            }
            Err(e) => {
                eprintln!("Error fetching plan item: {}",e);
                // Decide if you want to return partial data or an error
            }
        }
    }

    HttpResponse::Ok().json(plan)
}

async fn update_plan(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    plan_data: web::Json<TravelPlan>,
) -> impl Responder {
    let plan_id = path.into_inner();
    let conn = data.db.lock().unwrap();
    let plan = plan_data.into_inner();

    match conn.execute(
        "UPDATE travel_plans SET name = ?1, start_date = ?2, end_date = ?3 WHERE id = ?4",
        params![plan.name, plan.start_date, plan.end_date, plan_id],
    ) {
        Ok(updated_rows) => {
            if updated_rows == 0 {
                HttpResponse::NotFound().finish()
            } else {
                // Fetch the updated plan to return it, or construct it
                HttpResponse::Ok().json(TravelPlan{
                    id: Some(plan_id),
                    name: plan.name,
                    start_date: plan.start_date,
                    end_date: plan.end_date,
                    items: None, // Not returning items on update for simplicity
                })
            }
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn delete_plan(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
    let plan_id = path.into_inner();
    let conn = data.db.lock().unwrap();

    match conn.execute("DELETE FROM travel_plans WHERE id = ?1", params![plan_id]) {
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

// --- PlanItem Handlers ---

async fn add_plan_item(
    data: web::Data<AppState>,
    path: web::Path<i64>, // plan_id
    item_data: web::Json<PlanItemRequest>,
) -> impl Responder {
    let plan_id = path.into_inner();
    let conn = data.db.lock().unwrap();
    let item_req = item_data.into_inner();

    // Optional: Check if plan_id exists
    // let plan_exists: Result<i64> = conn.query_row(
    //     "SELECT 1 FROM travel_plans WHERE id = ?1",
    //     params![plan_id],
    //     |row| row.get(0),
    // );
    // if plan_exists.is_err() {
    //     return HttpResponse::NotFound().body("Plan not found");
    // }


    let mut new_item = PlanItem {
        id: None,
        plan_id,
        entity_type: item_req.entity_type,
        entity_id: item_req.entity_id,
        visit_date: item_req.visit_date,
        notes: item_req.notes,
    };

    match conn.execute(
        "INSERT INTO plan_items (plan_id, entity_type, entity_id, visit_date, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![new_item.plan_id, new_item.entity_type, new_item.entity_id, new_item.visit_date, new_item.notes],
    ) {
        Ok(_) => {
            new_item.id = Some(conn.last_insert_rowid());
            HttpResponse::Created().json(new_item)
        }
        Err(e) => {
            eprintln!("Failed to insert plan_item: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn update_plan_item(
    data: web::Data<AppState>,
    path: web::Path<(i64, i64)>, // (plan_id, item_id)
    item_data: web::Json<PlanItemRequest>,
) -> impl Responder {
    let (plan_id, item_id) = path.into_inner();
    let conn = data.db.lock().unwrap();
    let item_req = item_data.into_inner();

    // Optional: verify plan_id if necessary, though FK constraint should handle it

    match conn.execute(
        "UPDATE plan_items SET entity_type = ?1, entity_id = ?2, visit_date = ?3, notes = ?4 WHERE id = ?5 AND plan_id = ?6",
        params![item_req.entity_type, item_req.entity_id, item_req.visit_date, item_req.notes, item_id, plan_id],
    ) {
        Ok(updated_rows) => {
            if updated_rows == 0 {
                HttpResponse::NotFound().finish()
            } else {
                HttpResponse::Ok().json(PlanItem { // Return the conceptual updated item
                    id: Some(item_id),
                    plan_id,
                    entity_type: item_req.entity_type,
                    entity_id: item_req.entity_id,
                    visit_date: item_req.visit_date,
                    notes: item_req.notes,
                })
            }
        }
        Err(e) => {
            eprintln!("Failed to update plan_item: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn delete_plan_item(
    data: web::Data<AppState>,
    path: web::Path<(i64, i64)>, // (plan_id, item_id)
) -> impl Responder {
    let (plan_id, item_id) = path.into_inner();
    let conn = data.db.lock().unwrap();

    match conn.execute("DELETE FROM plan_items WHERE id = ?1 AND plan_id = ?2", params![item_id, plan_id]) {
        Ok(deleted_rows) => {
            if deleted_rows == 0 {
                HttpResponse::NotFound().finish()
            } else {
                HttpResponse::NoContent().finish()
            }
        }
        Err(e) => {
            eprintln!("Failed to delete plan_item: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}


// Handler functions for Restaurants
async fn get_restaurants(data: web::Data<AppState>) -> impl Responder {
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

async fn add_restaurant(
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

async fn get_restaurant(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
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

async fn update_restaurant(
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

async fn delete_restaurant(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
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

// Handler functions for Accommodations
async fn get_accommodations(data: web::Data<AppState>) -> impl Responder {
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

async fn add_accommodation(
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

async fn get_accommodation(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
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

async fn update_accommodation(
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

async fn delete_accommodation(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
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
