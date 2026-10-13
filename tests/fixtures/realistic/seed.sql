-- Seed data for the simulated shop DB. Every INSERT names its columns.
-- Literal formats exercised: escaped quotes, negative numbers, decimals,
-- scientific notation, hex blobs, TRUE/FALSE, NULL, dates-as-strings,
-- unicode, multi-row VALUES. 0 violations on both dialects.

INSERT INTO users (id, username, email, full_name, is_active, created_at) VALUES
    (1, 'amartin', 'alice.martin@example.com', 'Alice Martin', TRUE, '2022-03-14 09:12:00'),
    (2, 'bdurand', 'bob.durand@example.com', 'Bob Durand', TRUE, '2022-06-02 14:45:00'),
    (3, 'cbern', 'chloe.bernard@example.com', 'Chloé Bernard', TRUE, '2022-11-30 08:03:00'),
    (4, 'dl0pez', 'diego.lopez@example.com', 'Diego López', FALSE, '2023-01-17 19:26:00'),
    (5, 'eokafor', 'emeka.okafor@example.com', 'Emeka Okafor', TRUE, '2023-05-09 11:58:00'),
    (6, 'fyamada', 'yuki.yamada@example.com', 'Yuki Yamada', TRUE, '2023-09-21 16:34:00'),
    (7, 'g Rossi ', 'giulia.rossi@example.com', 'Giulia Rossi', TRUE, '2024-01-05 10:20:00'),
    (8, 'hmller', 'hans.muller@example.com', 'Hans Müller', TRUE, '2024-02-28 13:07:00');

INSERT INTO addresses (id, user_id, street, city, postal_code, country, kind) VALUES
    (1, 1, '12 rue des Lilas', 'Lyon', '69003', 'FR', 'home'),
    (2, 1, '5 avenue Jean Jaurès', 'Lyon', '69007', 'FR', 'work'),
    (3, 2, '88 Oberstraße', 'München', '80331', 'DE', 'home'),
    (4, 3, '23 Via Roma', 'Torino', '10121', 'IT', 'home'),
    (5, 5, '404 Cedar Ave', 'Austin', '78701', 'US', 'work'),
    (6, 6, '2-1-1 Shibaura', 'Tokyo', '105-0023', 'JP', 'home');

INSERT INTO categories (id, name, parent_id) VALUES
    (1, 'Electronics', NULL),
    (2, 'Computers', 1),
    (3, 'Phones', 1),
    (4, 'Audio', 1),
    (5, 'Books', NULL),
    (6, 'Fiction', 5),
    (7, 'Home', NULL),
    (8, 'Kitchen', 7);

INSERT INTO products (id, sku, name, category_id, unit_price, stock, is_active, created_at) VALUES
    (1, 'SKU-1001', 'Laptop Pro 14', 2, 1299.00, 12, TRUE, '2023-01-10 10:00:00'),
    (2, 'SKU-1002', 'Laptop Air 13', 2, 999.99, 30, TRUE, '2023-01-10 10:05:00'),
    (3, 'SKU-1003', 'Phone X', 3, 849.50, 45, TRUE, '2023-02-11 09:30:00'),
    (4, 'SKU-1004', 'Phone Mini', 3, 599.00, 8, TRUE, '2023-02-11 09:35:00'),
    (5, 'SKU-1005', 'Headphones ANC', 4, 249.90, 60, TRUE, '2023-03-01 14:00:00'),
    (6, 'SKU-1006', 'Desk Speaker', 4, 149.00, 0, FALSE, '2023-03-01 14:10:00'),
    (7, 'SKU-1007', 'The Silent Sea (novel)', 6, 14.90, 120, TRUE, '2023-04-15 08:45:00'),
    (8, 'SKU-1008', 'Blender 9000', 8, 89.95, 25, TRUE, '2023-05-20 12:00:00'),
    (9, 'SKU-1009', 'Espresso Machine', 8, 449.00, 14, TRUE, '2023-05-20 12:10:00'),
    (10, 'SKU-1010', '4K Monitor 27"', 2, 379.99, 22, TRUE, '2023-06-30 16:20:00'),
    (11, 'SKU-1011', 'USB-C Dock', 2, 129.00, 55, TRUE, '2023-06-30 16:25:00'),
    (12, 'SKU-1012', 'Discontinued Tablet', 2, 199.00, 3, FALSE, '2022-12-01 11:11:00');

INSERT INTO orders (id, user_id, status, total, currency, placed_at, shipped_at) VALUES
    (1, 1, 'delivered', 1448.90, 'EUR', '2023-07-01 10:15:00', '2023-07-03 08:00:00'),
    (2, 1, 'delivered', 14.90, 'EUR', '2023-07-15 22:40:00', '2023-07-18 07:30:00'),
    (3, 2, 'cancelled', 999.99, 'EUR', '2023-08-02 12:00:00', NULL),
    (4, 3, 'delivered', 1104.49, 'EUR', '2023-08-19 17:05:00', '2023-08-22 09:10:00'),
    (5, 5, 'paid', 249.90, 'USD', '2023-09-01 09:00:00', NULL),
    (6, 5, 'delivered', 388.99, 'USD', '2023-09-14 15:45:00', '2023-09-17 11:20:00'),
    (7, 6, 'delivered', 149.00, 'JPY', '2023-10-05 20:30:00', '2023-10-08 06:00:00'),
    (8, 7, 'shipped', 463.95, 'EUR', '2024-01-06 11:11:00', '2024-01-08 09:45:00'),
    (9, 8, 'pending', 1299.00, 'EUR', '2024-02-29 08:55:00', NULL),
    (10, 2, 'delivered', 89.95, 'EUR', '2024-03-10 13:33:00', '2024-03-12 10:00:00'),
    (11, 3, 'delivered', 599.00, 'EUR', '2024-03-21 18:20:00', '2024-03-24 07:15:00'),
    (12, 6, 'paid', 999.99, 'JPY', '2024-04-02 10:10:00', NULL),
    (13, 1, 'delivered', 449.00, 'EUR', '2024-04-18 09:05:00', '2024-04-20 14:00:00'),
    (14, 5, 'pending', 14.90, 'USD', '2024-05-01 21:00:00', NULL),
    (15, 8, 'delivered', 1104.49, 'EUR', '2024-05-12 16:40:00', '2024-05-15 08:30:00');

INSERT INTO order_items (id, order_id, product_id, quantity, unit_price, line_total) VALUES
    (1, 1, 1, 1, 1299.00, 1299.00),
    (2, 1, 5, 1, 249.90, 249.90),
    (3, 2, 7, 1, 14.90, 14.90),
    (4, 3, 2, 1, 999.99, 999.99),
    (5, 4, 3, 1, 849.50, 849.50),
    (6, 4, 11, 2, 129.00, 258.00),
    (7, 5, 5, 1, 249.90, 249.90),
    (8, 6, 10, 1, 379.99, 379.99),
    (9, 6, 5, 1, 249.90, 249.90),
    (10, 6, 5, 1, 249.90, 249.90),
    (11, 7, 6, 1, 149.00, 149.00),
    (12, 8, 9, 1, 449.00, 449.00),
    (13, 8, 11, 1, 129.00, 129.00),
    (14, 9, 1, 1, 1299.00, 1299.00),
    (15, 10, 8, 1, 89.95, 89.95),
    (16, 11, 4, 1, 599.00, 599.00),
    (17, 12, 2, 1, 999.99, 999.99),
    (18, 13, 9, 1, 449.00, 449.00),
    (19, 14, 7, 1, 14.90, 14.90),
    (20, 15, 1, 1, 1299.00, 1299.00),
    (21, 15, 11, 2, 129.00, 258.00);

INSERT INTO payments (id, order_id, method, amount, paid_at) VALUES
    (1, 1, 'card', 1448.90, '2023-07-01 10:16:00'),
    (2, 2, 'card', 14.90, '2023-07-15 22:41:00'),
    (4, 4, 'paypal', 1104.49, '2023-08-19 17:06:00'),
    (5, 5, 'card', 249.90, '2023-09-01 09:01:00'),
    (6, 6, 'card', 388.99, '2023-09-14 15:46:00'),
    (7, 7, 'transfer', 149.00, '2023-10-05 20:35:00'),
    (8, 8, 'paypal', 463.95, '2024-01-06 11:12:00'),
    (10, 10, 'card', 89.95, '2024-03-10 13:34:00'),
    (11, 11, 'card', 599.00, '2024-03-21 18:21:00'),
    (13, 13, 'transfer', 449.00, '2024-04-18 09:06:00'),
    (15, 15, 'card', 1104.49, '2024-05-12 16:41:00');

INSERT INTO reviews (id, product_id, user_id, rating, title, body, created_at) VALUES
    (1, 1, 1, 5, 'Fast and quiet', 'Compiles Rust in a blink.', '2023-07-10 12:00:00'),
    (2, 1, 2, 4, 'Great screen', 'Battery could be better.', '2023-08-01 09:30:00'),
    (3, 3, 3, 3, 'OK camera', 'Does the job.', '2023-08-25 18:45:00'),
    (4, 5, 1, 5, 'Silence at last', 'ANC is sorcery.', '2023-09-02 08:15:00'),
    (5, 5, 6, 2, 'Hurt my ears', 'Clamp force too high.', '2023-10-11 21:05:00'),
    (6, 7, 2, 5, 'Could not put it down', 'Read it twice.', '2023-10-20 23:10:00'),
    (7, 8, 3, 4, 'Solid blender', 'Loud but effective.', '2024-03-11 10:40:00'),
    (8, 9, 1, 5, 'Cafe quality', 'The 9bar is real.', '2024-04-20 07:50:00'),
    (9, 10, 5, 4, 'Crisp panel', 'Stand wobbles a bit.', '2023-09-20 14:25:00'),
    (10, 11, 8, 1, 'Died in a week', 'Dock of shame.', '2024-05-20 19:59:00'),
    (11, 11, 3, 2, 'Flaky ports', 'Random disconnects.', '2024-05-25 11:11:00'),
    (12, 4, 6, 4, 'Pocketable', 'Slower than the big one.', '2024-03-22 17:30:00');

INSERT INTO sessions (id, user_id, token, started_at, expires_at, revoked) VALUES
    (1, 1, 'tok_9f8e7d6c', '2024-05-01 08:00:00', '2024-06-01 08:00:00', FALSE),
    (2, 2, 'tok_a1b2c3d4', '2024-05-02 09:30:00', '2024-06-02 09:30:00', FALSE),
    (3, 3, 'tok_deadbeef', '2024-05-03 10:15:00', '2024-06-03 10:15:00', TRUE),
    (4, 5, 'tok_0f1e2d3c', '2024-05-04 11:45:00', '2024-05-11 11:45:00', FALSE),
    (5, 6, 'tok_55667788', '2024-05-05 13:20:00', '2024-06-05 13:20:00', FALSE),
    (6, 1, 'tok_1234abcd', '2024-05-06 15:00:00', '2024-05-13 15:00:00', TRUE),
    (7, 8, 'tok_facefeed', '2024-05-07 16:40:00', '2024-06-07 16:40:00', FALSE),
    (8, 3, 'tok_c0ffee00', '2024-05-08 18:05:00', '2024-05-15 18:05:00', FALSE);

INSERT INTO audit_log (id, actor_id, action, payload, occurred_at) VALUES
    (1, 1, 'login', NULL, '2024-05-01 08:00:01'),
    (2, 2, 'login', NULL, '2024-05-02 09:31:00'),
    (3, 3, 'password_change', X'7F454C46', '2024-05-02 12:00:00'),
    (4, 1, 'checkout', X'DEADBEEF', '2024-05-03 10:00:00'),
    (5, 5, 'login', NULL, '2024-05-04 11:46:00'),
    (6, 6, 'profile_update', X'00FF10', '2024-05-05 13:25:00'),
    (7, 7, 'login', NULL, '2024-05-06 09:00:00'),
    (8, 1, 'refund', X'CAFEBABE', '2024-05-06 10:30:00'),
    (9, 8, 'export', X'53514C697465', '2024-05-07 17:15:00'),
    (10, 3, 'login', NULL, '2024-05-08 18:06:00'),
    (11, 2, '2fa_enabled', X'0001', '2024-05-09 07:45:00'),
    (12, NULL, 'nightly_job', X'AA55AA55', '2024-05-09 03:00:00');

-- Single-row inserts exercising rare literal formats.
INSERT INTO users (id, username, email, full_name, is_active, created_at)
    VALUES (9, 'O''Brien', 'obrien@example.com', 'Sean O''Brien', TRUE, '2024-05-20 08:00:00');

INSERT INTO products (id, sku, name, category_id, unit_price, stock, is_active, created_at)
    VALUES (13, 'SKU-1013', 'Bulk Cable 1.5e3 m', 2, 1.5e2, 1000, TRUE, '2024-05-21 09:00:00');

INSERT INTO audit_log (id, actor_id, action, payload, occurred_at)
    VALUES (13, NULL, 'import', X'FFFFFFFF', '2024-05-21 03:00:00');

INSERT INTO addresses (id, user_id, street, city, postal_code, country, kind)
    VALUES (7, 9, '42 Rue Neuve', 'Bruxelles', '1000', 'BE', 'home');

INSERT INTO orders (id, user_id, status, total, currency, placed_at, shipped_at)
    VALUES (16, 9, 'pending', 1234.56, 'EUR', '2024-05-22 10:00:00', NULL);

-- Negative amount (refund).
INSERT INTO payments (id, order_id, method, amount, paid_at)
    VALUES (16, 16, 'card', -42.00, '2024-05-22 10:05:00');
