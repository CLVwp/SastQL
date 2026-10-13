-- Deliberate query bugs. Default-rule counts pinned in tests/integration.rs:
--   case-without-else x2 (2 CASE) | join-without-on x2 (comma FROM + bare JOIN)
--   natural-join x1 | select-star x2 (NATURAL + *, col) | select-star-plus x1
--   insert-without-columns x1 | offset-without-limit x1 | order-by-random x1
--   leading-wildcard-like x2 | weak-hash x2 | no-where-on-mutation x1
--   null-comparison x1 | not-in-null-trap x1 | string-plus x1 (SQLite only)
-- Opt-in demos (silent in default runs): UNION without ALL, ORDER BY 2,
-- LIMIT without ORDER BY.

SELECT CASE WHEN total > 1000 THEN 'big' END FROM orders;

SELECT CASE WHEN stock = 0 THEN 'out' END FROM products;

SELECT u.id, a.city FROM users u, addresses a WHERE a.user_id = u.id;

SELECT o.id FROM orders o JOIN payments p;

SELECT * FROM users NATURAL JOIN orders;

INSERT INTO audit_log VALUES (999, NULL, 'hack', NULL, '2024-01-01');

SELECT id FROM orders ORDER BY id OFFSET 50;

SELECT id FROM products ORDER BY RANDOM() LIMIT 10;

SELECT id FROM users WHERE email LIKE '%@gmail.com';

SELECT id, sku FROM products WHERE sku LIKE '_pro%';

SELECT md5(email) FROM users;

SELECT SHA1(token) FROM sessions;

SELECT *, revoked FROM sessions;

UPDATE sessions SET revoked = TRUE;

SELECT id FROM users WHERE deleted_at = NULL;

SELECT id FROM users WHERE id NOT IN (SELECT user_id FROM sessions);

SELECT 'order-' + id FROM orders;

-- Opt-in demos.
SELECT id FROM orders_2023 UNION SELECT id FROM orders_2024;
SELECT id, total FROM orders ORDER BY 2;
SELECT id FROM products LIMIT 5;
