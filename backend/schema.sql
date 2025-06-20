CREATE TABLE IF NOT EXISTS places (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    location TEXT
);

CREATE TABLE IF NOT EXISTS accommodations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    location TEXT
);

CREATE TABLE IF NOT EXISTS restaurants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    location TEXT
);

CREATE TABLE IF NOT EXISTS travel_plans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    start_date TEXT, -- Using TEXT for simplicity, can be ISO8601 date string
    end_date TEXT
);

CREATE TABLE IF NOT EXISTS plan_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_id INTEGER NOT NULL,
    entity_type TEXT NOT NULL, -- 'place', 'accommodation', 'restaurant'
    entity_id INTEGER NOT NULL,
    visit_date TEXT, -- Specific date for visiting this item
    notes TEXT,
    FOREIGN KEY (plan_id) REFERENCES travel_plans(id) ON DELETE CASCADE
);
