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

#[derive(Serialize, Deserialize, Debug, Clone)] // Added Clone here
pub struct PlanItemRequest { // For POST/PUT requests for PlanItem
    pub entity_type: String,
    pub entity_id: i64,
    pub visit_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    for plan_result in plan_iter {
        match plan_result {
            Ok(p) => plans.push(p),
            Err(e) => {
                eprintln!("Error fetching plan: {}", e);
                // Optionally skip this plan or return an error for the whole request
            }
        }
    }

    let total_count: Result<i64, _> = conn.query_row(
        "SELECT COUNT(*) FROM travel_plans",
        [],
        |row| row.get(0),
    );

    match total_count {
        Ok(count) => {
            let range_header = if plans.is_empty() {
                // The resource is "plans" as per admin/src/App.tsx
                format!("plans 0-0/{}", count)
            } else {
                format!("plans 0-{}/{}", plans.len() -1, count)
            };
            HttpResponse::Ok()
                .insert_header(("Content-Range", range_header))
                .json(plans)
        }
        Err(e) => {
            eprintln!("Failed to get total count for travel_plans: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, http::StatusCode, HttpRequest, body::to_bytes};
    use rusqlite::Connection;
    use std::sync::Mutex;
    use crate::db::AppState;
    use std::fs;

    fn setup_test_app_state() -> AppState {
        let conn = Connection::open_in_memory().unwrap();
        let schema = fs::read_to_string("../schema.sql")
            .or_else(|_| fs::read_to_string("schema.sql"))
            .expect("Should have been able to read the schema.sql file");
        conn.execute_batch(&schema).unwrap();
        AppState { db: Mutex::new(conn) }
    }

    fn default_req() -> HttpRequest {
        test::TestRequest::default().to_http_request()
    }

    // Helper function to add a travel plan and return its ID
    async fn add_test_plan(app_state: &web::Data<AppState>, name: &str, http_req: &HttpRequest) -> i64 {
        let plan = TravelPlan {
            id: None,
            name: name.to_string(),
            start_date: Some("2024-01-01".to_string()),
            end_date: Some("2024-01-05".to_string()),
            items: None,
        };
        let resp = add_plan(app_state.clone(), web::Json(plan.clone())).await;
        let http_resp = resp.respond_to(http_req);
        let body_bytes = match to_bytes(http_resp.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for add_test_plan helper"),
        };
        let added_plan: TravelPlan = serde_json::from_slice(&body_bytes).expect("Failed to deserialize for add_test_plan");
        added_plan.id.unwrap()
    }

    #[actix_web::test]
    async fn test_add_get_travel_plan() {
        let app_state = web::Data::new(setup_test_app_state());
        let http_req = default_req();
        let plan_name = "Adventure Trip";
        let new_plan = TravelPlan {
            id: None,
            name: plan_name.to_string(),
            start_date: Some("2024-03-10".to_string()),
            end_date: Some("2024-03-15".to_string()),
            items: None,
        };

        let resp_add = add_plan(app_state.clone(), web::Json(new_plan.clone())).await;
        let http_resp_add = resp_add.respond_to(&http_req);
        assert_eq!(http_resp_add.status(), StatusCode::CREATED);
        let body_bytes_add = match to_bytes(http_resp_add.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for add_plan"),
        };
        let added_plan: TravelPlan = serde_json::from_slice(&body_bytes_add).expect("Failed to deserialize added plan");
        assert!(added_plan.id.is_some());
        assert_eq!(added_plan.name, plan_name);

        let plan_id = added_plan.id.unwrap();

        // Test Get Single Travel Plan
        let resp_get = get_plan(app_state.clone(), web::Path::from(plan_id)).await;
        let http_resp_get = resp_get.respond_to(&http_req);
        assert_eq!(http_resp_get.status(), StatusCode::OK);
        let body_bytes_get = match to_bytes(http_resp_get.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for get_plan"),
        };
        let fetched_plan: TravelPlan = serde_json::from_slice(&body_bytes_get).expect("Failed to deserialize fetched plan");
        assert_eq!(fetched_plan.id, Some(plan_id));
        assert_eq!(fetched_plan.name, plan_name);
        assert!(fetched_plan.items.is_some()); // Should initialize items vec

        // Test Get All Travel Plans
        let resp_get_all = get_plans(app_state.clone()).await;
        let http_resp_get_all = resp_get_all.respond_to(&http_req);
        assert_eq!(http_resp_get_all.status(), StatusCode::OK);
        let body_bytes_get_all = match to_bytes(http_resp_get_all.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for get_plans"),
        };
        let all_plans: Vec<TravelPlan> = serde_json::from_slice(&body_bytes_get_all).expect("Failed to deserialize all plans");
        assert_eq!(all_plans.len(), 1);
        assert_eq!(all_plans[0].id, Some(plan_id));
    }

    #[actix_web::test]
    async fn test_update_travel_plan() {
        let app_state = web::Data::new(setup_test_app_state());
        let http_req = default_req();
        let plan_id = add_test_plan(&app_state, "Initial Plan", &http_req).await;

        let updated_details = TravelPlan {
            id: None,
            name: "Updated Adventure Plan".to_string(),
            start_date: Some("2024-07-01".to_string()),
            end_date: Some("2024-07-07".to_string()),
            items: None,
        };
        let resp_update = update_plan(app_state.clone(), web::Path::from(plan_id), web::Json(updated_details.clone())).await;
        let http_resp_update = resp_update.respond_to(&http_req);
        assert_eq!(http_resp_update.status(), StatusCode::OK);
        let body_bytes_update = match to_bytes(http_resp_update.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for update_plan"),
        };
        let updated_plan_resp: TravelPlan = serde_json::from_slice(&body_bytes_update).expect("Failed to deserialize updated plan");
        assert_eq!(updated_plan_resp.name, "Updated Adventure Plan");

        // Verify by getting
        let resp_get = get_plan(app_state.clone(), web::Path::from(plan_id)).await;
        let http_resp_get = resp_get.respond_to(&http_req);
        let body_bytes_get = match to_bytes(http_resp_get.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for get_plan after update_plan"),
        };
        let fetched_plan: TravelPlan = serde_json::from_slice(&body_bytes_get).expect("Failed to deserialize fetched plan after update");
        assert_eq!(fetched_plan.name, "Updated Adventure Plan");
    }

    #[actix_web::test]
    async fn test_delete_travel_plan() {
        let app_state = web::Data::new(setup_test_app_state());
        let http_req = default_req();
        let plan_id = add_test_plan(&app_state, "Plan to Delete", &http_req).await;

        let item_req = PlanItemRequest {
            entity_type: "place".to_string(),
            entity_id: 1,
            visit_date: Some("2024-01-01".to_string()),
            notes: Some("Visit museum".to_string()),
        };
        let add_item_resp = add_plan_item(app_state.clone(), web::Path::from(plan_id), web::Json(item_req.clone())).await;
        let _ = add_item_resp.respond_to(&http_req); // Consume responder

        let resp_delete = delete_plan(app_state.clone(), web::Path::from(plan_id)).await;
        let http_resp_delete = resp_delete.respond_to(&http_req);
        assert_eq!(http_resp_delete.status(), StatusCode::NO_CONTENT);

        // Verify plan is deleted
        let resp_get = get_plan(app_state.clone(), web::Path::from(plan_id)).await;
        let http_resp_get = resp_get.respond_to(&http_req);
        assert_eq!(http_resp_get.status(), StatusCode::NOT_FOUND);

        let conn = app_state.db.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM plan_items WHERE plan_id = ?1",
            params![plan_id],
            |row| row.get(0),
        ).unwrap_or(0);
        assert_eq!(count, 0, "Plan items should be deleted when the plan is deleted");
    }

    #[actix_web::test]
    async fn test_travel_plan_not_found_scenarios() {
        let app_state = web::Data::new(setup_test_app_state());
        let http_req = default_req();
        let non_existent_plan_id = 999i64;

        let resp_get = get_plan(app_state.clone(), web::Path::from(non_existent_plan_id)).await;
        assert_eq!(resp_get.respond_to(&http_req).status(), StatusCode::NOT_FOUND);

        let plan_details = TravelPlan { id: None, name: "ghost".into(), start_date: None, end_date: None, items: None };
        let resp_update = update_plan(app_state.clone(), web::Path::from(non_existent_plan_id), web::Json(plan_details.clone())).await;
        assert_eq!(resp_update.respond_to(&http_req).status(), StatusCode::NOT_FOUND);

        let resp_delete = delete_plan(app_state.clone(), web::Path::from(non_existent_plan_id)).await;
        assert_eq!(resp_delete.respond_to(&http_req).status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_add_get_plan_item() {
        let app_state = web::Data::new(setup_test_app_state());
        let http_req = default_req();
        let plan_id = add_test_plan(&app_state, "Plan For Items", &http_req).await;

        let item_req = PlanItemRequest {
            entity_type: "accommodation".to_string(),
            entity_id: 123,
            visit_date: Some("2024-03-11".to_string()),
            notes: Some("Check in early".to_string()),
        };

        let resp_add_item = add_plan_item(app_state.clone(), web::Path::from(plan_id), web::Json(item_req.clone())).await;
        let http_resp_add_item = resp_add_item.respond_to(&http_req);
        assert_eq!(http_resp_add_item.status(), StatusCode::CREATED);
        let body_bytes_add_item = match to_bytes(http_resp_add_item.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for add_plan_item"),
        };
        let added_item: PlanItem = serde_json::from_slice(&body_bytes_add_item).expect("Failed to deserialize added item");
        assert!(added_item.id.is_some());
        assert_eq!(added_item.plan_id, plan_id);
        assert_eq!(added_item.entity_type, "accommodation");
        assert_eq!(added_item.entity_id, 123);

        let item_id = added_item.id.unwrap();

        let resp_get_plan = get_plan(app_state.clone(), web::Path::from(plan_id)).await;
        let http_resp_get_plan = resp_get_plan.respond_to(&http_req);
        let body_bytes_get_plan = match to_bytes(http_resp_get_plan.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for get_plan in add_get_plan_item test"),
        };
        let fetched_plan: TravelPlan = serde_json::from_slice(&body_bytes_get_plan).expect("Failed to deserialize plan with item");
        assert_eq!(fetched_plan.items.as_ref().map_or(0, |i| i.len()), 1);
        let fetched_item = &fetched_plan.items.unwrap()[0];
        assert_eq!(fetched_item.id, Some(item_id));
        assert_eq!(fetched_item.entity_type, "accommodation");
    }

    #[actix_web::test]
    async fn test_update_plan_item() {
        let app_state = web::Data::new(setup_test_app_state());
        let http_req = default_req();
        let plan_id = add_test_plan(&app_state, "Plan for Item Update", &http_req).await;

        let initial_item_req = PlanItemRequest {
            entity_type: "place".to_string(),
            entity_id: 1,
            visit_date: Some("2024-01-01".to_string()),
            notes: Some("Initial note".to_string()),
        };
        let resp_add = add_plan_item(app_state.clone(), web::Path::from(plan_id), web::Json(initial_item_req.clone())).await;
        let add_item_body_bytes = match to_bytes(resp_add.respond_to(&http_req).into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for add_plan_item in update_plan_item test"),
        };
        let added_item: PlanItem = serde_json::from_slice(&add_item_body_bytes).expect("Failed to deserialize added item in update test");
        let item_id = added_item.id.unwrap();

        let updated_item_req = PlanItemRequest {
            entity_type: "place".to_string(),
            entity_id: 2,
            visit_date: Some("2024-01-02".to_string()),
            notes: Some("Updated note".to_string()),
        };
        let resp_update_item = update_plan_item(app_state.clone(), web::Path::from((plan_id, item_id)), web::Json(updated_item_req.clone())).await;
        let http_resp_update_item = resp_update_item.respond_to(&http_req);
        assert_eq!(http_resp_update_item.status(), StatusCode::OK);
        let update_item_body_bytes = match to_bytes(http_resp_update_item.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for update_plan_item"),
        };
        let updated_item_resp: PlanItem = serde_json::from_slice(&update_item_body_bytes).expect("Failed to deserialize updated item");
        assert_eq!(updated_item_resp.id, Some(item_id));
        assert_eq!(updated_item_resp.entity_id, 2);
        assert_eq!(updated_item_resp.notes, Some("Updated note".to_string()));

        let resp_get_plan = get_plan(app_state.clone(), web::Path::from(plan_id)).await;
        let http_resp_get_plan = resp_get_plan.respond_to(&http_req);
        let get_plan_body_bytes = match to_bytes(http_resp_get_plan.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for get_plan after update_plan_item"),
        };
        let fetched_plan: TravelPlan = serde_json::from_slice(&get_plan_body_bytes).expect("Failed to deserialize plan after item update");
        let item_in_plan = fetched_plan.items.unwrap().into_iter().find(|i| i.id == Some(item_id)).unwrap();
        assert_eq!(item_in_plan.entity_id, 2);
        assert_eq!(item_in_plan.notes, Some("Updated note".to_string()));
    }

    #[actix_web::test]
    async fn test_delete_plan_item() {
        let app_state = web::Data::new(setup_test_app_state());
        let http_req = default_req();
        let plan_id = add_test_plan(&app_state, "Plan for Item Deletion", &http_req).await;

        let item_req1 = PlanItemRequest { entity_type: "activity".to_string(), entity_id: 10, visit_date: None, notes: None };
        let resp_add1 = add_plan_item(app_state.clone(), web::Path::from(plan_id), web::Json(item_req1.clone())).await;
        let add1_body_bytes = match to_bytes(resp_add1.respond_to(&http_req).into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for add_plan_item 1 in delete_plan_item test"),
        };
        let item1: PlanItem = serde_json::from_slice(&add1_body_bytes).expect("Failed to deserialize item 1 in delete test");
        let item_id1 = item1.id.unwrap();

        let item_req2 = PlanItemRequest { entity_type: "restaurant".to_string(), entity_id: 20, visit_date: None, notes: None };
        let resp_add2 = add_plan_item(app_state.clone(), web::Path::from(plan_id), web::Json(item_req2.clone())).await;
        let _ = resp_add2.respond_to(&http_req); // Consume responder

        let resp_delete_item = delete_plan_item(app_state.clone(), web::Path::from((plan_id, item_id1))).await;
        let http_resp_delete_item = resp_delete_item.respond_to(&http_req);
        assert_eq!(http_resp_delete_item.status(), StatusCode::NO_CONTENT);

        let resp_get_plan = get_plan(app_state.clone(), web::Path::from(plan_id)).await;
        let http_resp_get_plan = resp_get_plan.respond_to(&http_req);
        let get_plan_body_bytes = match to_bytes(http_resp_get_plan.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => panic!("Failed to read body for get_plan after delete_plan_item"),
        };
        let fetched_plan: TravelPlan = serde_json::from_slice(&get_plan_body_bytes).expect("Failed to deserialize plan after item delete");
        assert_eq!(fetched_plan.items.as_ref().map_or(0, |i| i.len()), 1);
        assert!(fetched_plan.items.unwrap().iter().all(|i| i.id != Some(item_id1)));

        let non_existent_item_id = 999i64;
        let resp_delete_non_existent = delete_plan_item(app_state.clone(), web::Path::from((plan_id, non_existent_item_id))).await;
        assert_eq!(resp_delete_non_existent.respond_to(&http_req).status(), StatusCode::NOT_FOUND);

         let resp_delete_from_non_existent_plan = delete_plan_item(app_state.clone(), web::Path::from((999i64, item_id1))).await;
         assert_eq!(resp_delete_from_non_existent_plan.respond_to(&http_req).status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_plan_item_not_found_scenarios() {
        let app_state = web::Data::new(setup_test_app_state());
        let http_req = default_req();
        let plan_id = add_test_plan(&app_state, "Plan for Not Found Items", &http_req).await;
        let non_existent_item_id = 888i64;
        let non_existent_plan_id = 777i64;

        let item_details = PlanItemRequest { entity_type: "ghost".into(), entity_id: 0, visit_date: None, notes: None };

        let resp_update = update_plan_item(app_state.clone(), web::Path::from((plan_id, non_existent_item_id)), web::Json(item_details.clone())).await;
        assert_eq!(resp_update.respond_to(&http_req).status(), StatusCode::NOT_FOUND);

        let resp_update_np = update_plan_item(app_state.clone(), web::Path::from((non_existent_plan_id, non_existent_item_id)), web::Json(item_details.clone())).await;
        assert_eq!(resp_update_np.respond_to(&http_req).status(), StatusCode::NOT_FOUND);

        let resp_delete = delete_plan_item(app_state.clone(), web::Path::from((plan_id, non_existent_item_id))).await;
        assert_eq!(resp_delete.respond_to(&http_req).status(), StatusCode::NOT_FOUND);

        let resp_delete_np = delete_plan_item(app_state.clone(), web::Path::from((non_existent_plan_id, non_existent_item_id))).await;
        assert_eq!(resp_delete_np.respond_to(&http_req).status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_add_item_to_non_existent_plan() {
        let app_state = web::Data::new(setup_test_app_state());
        let http_req = default_req();
        let non_existent_plan_id = 999i64;
        let item_req = PlanItemRequest {
            entity_type: "test".to_string(),
            entity_id: 1,
            visit_date: None,
            notes: None,
        };
        let resp = add_plan_item(app_state.clone(), web::Path::from(non_existent_plan_id), web::Json(item_req.clone())).await;
        let http_resp = resp.respond_to(&http_req);
        assert_eq!(http_resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
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
