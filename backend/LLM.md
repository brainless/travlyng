# Backend System Overview for LLM

This document provides a comprehensive overview of the backend system to assist Large Language Models (LLMs) in understanding its architecture, components, and functionalities.

## 1. Language and Framework

*   **Language:** Rust
    *   The backend is implemented in Rust, a systems programming language known for its performance, safety, and concurrency features.
*   **Framework:** Actix-web
    *   Actix-web is a powerful, pragmatic, and extremely fast web framework for Rust. It's used to build the RESTful APIs for this application.

## 2. Project Structure

The backend project is organized as follows:

*   `Cargo.toml`: This is the manifest file for the Rust project (managed by Cargo, Rust's package manager). It contains metadata about the project, such as its name, version, edition, and dependencies (e.g., `actix-web`, `serde`, `rusqlite`).
*   `schema.sql`: Defines the database schema. This file contains SQL `CREATE TABLE` statements that describe the structure of the database tables, their columns, data types, and relationships.
*   `src/`: This directory contains all the Rust source code.
    *   `main.rs`: The entry point of the application. It initializes the database, sets up the Actix-web HTTP server, defines API routes, and configures middleware (like CORS).
    *   `db.rs`: Handles database-related logic, including initializing the database connection (using `rusqlite`) and defining the `AppState` struct that holds the shared database connection pool for Actix-web handlers.
    *   `places.rs`: Contains HTTP handlers and business logic for the "places" entity.
    *   `accommodations.rs`: Contains HTTP handlers and business logic for the "accommodations" entity.
    *   `restaurants.rs`: Contains HTTP handlers and business logic for the "restaurants" entity.
    *   `travel_plans.rs`: Contains HTTP handlers and business logic for "travel_plans" and "plan_items" entities.
    *   `search.rs`: Contains the logic for the search functionality across different entities.

## 3. Database

*   **Type:** SQLite
    *   The application uses SQLite, a C-language library that implements a small, fast, self-contained, high-reliability, full-featured, SQL database engine. The database is stored in a single file.
*   **Schema:** Defined in `schema.sql`.

    *   **`places` Table:** Stores information about places of interest.
        *   `id`: INTEGER PRIMARY KEY AUTOINCREMENT - Unique identifier for the place.
        *   `name`: TEXT NOT NULL - Name of the place.
        *   `description`: TEXT - Detailed description of the place.
        *   `location`: TEXT - Location of the place (e.g., address or coordinates).

    *   **`accommodations` Table:** Stores details about lodging.
        *   `id`: INTEGER PRIMARY KEY AUTOINCREMENT - Unique identifier for the accommodation.
        *   `name`: TEXT NOT NULL - Name of the accommodation.
        *   `description`: TEXT - Detailed description.
        *   `location`: TEXT - Location of the accommodation.

    *   **`restaurants` Table:** Stores information about dining options.
        *   `id`: INTEGER PRIMARY KEY AUTOINCREMENT - Unique identifier for the restaurant.
        *   `name`: TEXT NOT NULL - Name of the restaurant.
        *   `description`: TEXT - Detailed description.
        *   `location`: TEXT - Location of the restaurant.

    *   **`travel_plans` Table:** Stores overall travel plans.
        *   `id`: INTEGER PRIMARY KEY AUTOINCREMENT - Unique identifier for the travel plan.
        *   `name`: TEXT NOT NULL - Name of the travel plan.
        *   `start_date`: TEXT - Start date of the travel plan (ISO8601 format recommended).
        *   `end_date`: TEXT - End date of the travel plan (ISO8601 format recommended).

    *   **`plan_items` Table:** Links entities (places, accommodations, restaurants) to travel plans. This acts as a join table with additional details.
        *   `id`: INTEGER PRIMARY KEY AUTOINCREMENT - Unique identifier for the plan item.
        *   `plan_id`: INTEGER NOT NULL - Foreign key referencing `travel_plans(id)`. Indicates which travel plan this item belongs to.
        *   `entity_type`: TEXT NOT NULL - Type of the linked entity (e.g., 'place', 'accommodation', 'restaurant').
        *   `entity_id`: INTEGER NOT NULL - Foreign key referencing the ID of the specific entity in its respective table.
        *   `visit_date`: TEXT - Specific date for visiting this item within the travel plan.
        *   `notes`: TEXT - Additional notes for this plan item.

## 4. API Endpoints

The API is defined in `src/main.rs` and implemented in the respective entity modules.

*   **Places (`/places`)**
    *   `GET /places`: List all places.
    *   `POST /places`: Add a new place.
    *   `GET /places/{id}`: Get a specific place by ID.
    *   `PUT /places/{id}`: Update a specific place by ID.
    *   `DELETE /places/{id}`: Delete a specific place by ID.

*   **Accommodations (`/accommodations`)**
    *   `GET /accommodations`: List all accommodations.
    *   `POST /accommodations`: Add a new accommodation.
    *   `GET /accommodations/{id}`: Get a specific accommodation by ID.
    *   `PUT /accommodations/{id}`: Update a specific accommodation by ID.
    *   `DELETE /accommodations/{id}`: Delete a specific accommodation by ID.

*   **Restaurants (`/restaurants`)**
    *   `GET /restaurants`: List all restaurants.
    *   `POST /restaurants`: Add a new restaurant.
    *   `GET /restaurants/{id}`: Get a specific restaurant by ID.
    *   `PUT /restaurants/{id}`: Update a specific restaurant by ID.
    *   `DELETE /restaurants/{id}`: Delete a specific restaurant by ID.

*   **Travel Plans (`/plans`)**
    *   `GET /plans`: List all travel plans.
    *   `POST /plans`: Add a new travel plan.
    *   `GET /plans/{id}`: Get a specific travel plan by ID (likely including its items).
    *   `PUT /plans/{id}`: Update a specific travel plan by ID.
    *   `DELETE /plans/{id}`: Delete a specific travel plan by ID.
    *   **Plan Items (nested under `/plans`)**
        *   `POST /plans/{plan_id}/items`: Add an item (place, accommodation, or restaurant) to a specific travel plan.
        *   `PUT /plans/{plan_id}/items/{item_id}`: Update a specific item within a travel plan.
        *   `DELETE /plans/{plan_id}/items/{item_id}`: Delete a specific item from a travel plan.

*   **Search (`/search`)**
    *   `GET /search`: Allows searching across multiple entity types (places, accommodations, restaurants). Query parameters will likely be used to specify search terms.

## 5. Core Logic Flow

1.  **Server Initialization:** `main.rs` initializes the Actix-web `HttpServer`.
2.  **Database Connection:** `db.rs` provides functions to initialize the SQLite database. An `AppState` struct, containing a `Mutex`-wrapped `rusqlite::Connection`, is shared across all handlers.
3.  **Routing:** `main.rs` defines routes using `App::new().service(web::scope(...).route(...))`. Each route is mapped to an asynchronous handler function located in one of the entity-specific modules (e.g., `places::get_places`).
4.  **Request Handling:** When an HTTP request matches a defined route:
    *   The corresponding handler function is invoked.
    *   Handlers can access path parameters, query parameters, and request bodies (typically JSON).
    *   Handlers use the `AppState` to get a database connection.
    *   Business logic (data validation, database operations) is performed.
    *   Responses (typically JSON) are returned with appropriate HTTP status codes.

## 6. Data Serialization

*   **JSON:** The API primarily uses JSON for request and response bodies.
*   **Serde:** The `serde` crate (with the `derive` feature) is used for serializing Rust structs into JSON and deserializing JSON into Rust structs. This is evident from its presence in `Cargo.toml` and common usage patterns in Actix-web applications.

## 7. Tips for LLM Analysis

*   **Database Schema:** For a deep understanding of data structures and relationships, always refer to `backend/schema.sql`.
*   **API Endpoints & Structure:** `backend/src/main.rs` is the best place to see how routes are defined and which handler functions are responsible for them.
*   **Entity-Specific Logic:** For business logic related to a specific entity (e.g., how places are created or queried), look into the corresponding `backend/src/<entity_name>.rs` file (e.g., `backend/src/places.rs`).
*   **Dependencies:** `backend/Cargo.toml` lists all external libraries used, which can give clues about functionalities (e.g., `rusqlite` for DB, `actix-web` for web server, `serde` for JSON).
*   **Error Handling:** Expect standard Rust error handling patterns (e.g., `Result<T, E>`) and Actix-web's error handling mechanisms.
*   **Asynchronous Operations:** Most request handlers will be `async` functions, as Actix-web is an asynchronous framework.

This document should serve as a good starting point for understanding the backend system.
If more specific details are needed, direct code analysis of the mentioned files will be necessary.
