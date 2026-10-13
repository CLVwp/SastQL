-- Simulated production schema: e-commerce/inventory domain.
-- Clean on BOTH dialects under all default rules: every table has a PK,
-- money is NUMERIC, timestamps are TEXT, explicit columns everywhere.
-- Seeded and queried by seed.sql / queries.sql; bugs_*.sql break it.

CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    full_name TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TEXT NOT NULL
);

CREATE TABLE addresses (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id),
    street TEXT,
    city TEXT,
    postal_code TEXT,
    country TEXT,
    kind TEXT CHECK (kind IN ('home', 'work', 'other'))
);

CREATE TABLE categories (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    parent_id INTEGER REFERENCES categories (id)
);

CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    sku TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    category_id INTEGER REFERENCES categories (id),
    unit_price NUMERIC(10, 2) NOT NULL,
    stock INTEGER NOT NULL DEFAULT 0 CHECK (stock >= 0),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TEXT NOT NULL
);

CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'paid', 'shipped', 'delivered', 'cancelled')),
    total NUMERIC(12, 2) NOT NULL,
    currency TEXT DEFAULT 'EUR',
    placed_at TEXT NOT NULL,
    shipped_at TEXT
);

CREATE TABLE order_items (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES orders (id),
    product_id INTEGER NOT NULL REFERENCES products (id),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price NUMERIC(10, 2) NOT NULL,
    line_total NUMERIC(12, 2) NOT NULL
);

CREATE TABLE payments (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES orders (id),
    method TEXT CHECK (method IN ('card', 'paypal', 'transfer')),
    amount NUMERIC(12, 2) CHECK (amount > 0),
    paid_at TEXT
);

CREATE TABLE reviews (
    id INTEGER PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products (id),
    user_id INTEGER NOT NULL REFERENCES users (id),
    rating INTEGER NOT NULL CHECK (rating BETWEEN 1 AND 5),
    title TEXT,
    body TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id),
    token TEXT NOT NULL UNIQUE,
    started_at TEXT,
    expires_at TEXT,
    revoked BOOLEAN DEFAULT FALSE
);

CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY,
    actor_id INTEGER REFERENCES users (id),
    action TEXT NOT NULL,
    payload TEXT,
    occurred_at TEXT NOT NULL
);

CREATE INDEX idx_orders_user ON orders (user_id);
CREATE INDEX idx_order_items_order ON order_items (order_id);
CREATE INDEX idx_products_category ON products (category_id);
CREATE INDEX idx_sessions_user ON sessions (user_id);
CREATE INDEX idx_reviews_product ON reviews (product_id);

CREATE VIEW v_active_products AS
    SELECT p.id, p.sku, p.name, p.unit_price, p.stock
    FROM products p
    WHERE p.is_active = TRUE;

CREATE VIEW v_order_totals AS
    SELECT o.id, o.user_id, o.total, COUNT(oi.id) AS item_count, SUM(oi.line_total) AS items_total
    FROM orders o
    LEFT JOIN order_items oi ON oi.order_id = o.id
    GROUP BY o.id, o.user_id, o.total;
