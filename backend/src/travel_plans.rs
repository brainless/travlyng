use actix_web::{web, HttpResponse, Responder};
use rusqlite::params; // Removed Result as it's not directly used here, Connection is used via AppState
use serde::{Deserialize, Serialize};
use crate::db::AppState;

#[derive(Serialize, Deserialize, Debug, Clone)] // Added Clone
pub struct PlanItem {
    pub id: Option<i64>,
    pub plan_id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub visit_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlanItemRequest { // For POST/PUT requests for PlanItem
    pub entity_type: String,
    pub entity_id: i64,
    pub visit_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TravelPlan {
    pub id: Option<i64>,
    pub name: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub items: Option<Vec<PlanItem>>, // Populated when fetching a single plan
}

// --- TravelPlan Handlers ---

pub async fn get_plans(data: web::Data<AppState>) -> impl Responder {
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

pub async fn add_plan(data: web::Data<AppState>, plan_data: web::Json<TravelPlan>) -> impl Responder {
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

pub async fn get_plan(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
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

pub async fn update_plan(
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

pub async fn delete_plan(data: web::Data<AppState>, path: web::Path<i64>) -> impl Responder {
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

pub async fn add_plan_item(
    data: web::Data<AppState>,
    path: web::Path<i64>, // plan_id
    item_data: web::Json<PlanItemRequest>,
) -> impl Responder {
    let plan_id = path.into_inner();
    let conn = data.db.lock().unwrap();
    let item_req = item_data.into_inner();

    // Optional: Check if plan_id exists
    // let plan_exists: Result<i64> = conn.query_row(
    // "SELECT 1 FROM travel_plans WHERE id = ?1",
    // params![plan_id],
    // |row| row.get(0),
    // );
    // if plan_exists.is_err() {
    // return HttpResponse::NotFound().body("Plan not found");
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

pub async fn update_plan_item(
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

pub async fn delete_plan_item(
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
