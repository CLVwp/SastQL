-- Volume queries over the simulated shop DB. Clean on BOTH dialects under
-- all default rules: explicit columns, every JOIN has ON, every CASE has
-- ELSE, pagination always ORDER BY + LIMIT, no wildcards, UNION ALL only.

-- Q1: customers by lifetime spend (CTE + join).
WITH spend AS (
    SELECT user_id, SUM(total) AS lifetime_total
    FROM orders
    WHERE status <> 'cancelled'
    GROUP BY user_id
)
SELECT u.username, s.lifetime_total
FROM users u
JOIN spend s ON s.user_id = u.id
ORDER BY s.lifetime_total DESC;

-- Q2: price ranking inside each category (window function).
SELECT p.name, p.unit_price,
       ROW_NUMBER() OVER (PARTITION BY p.category_id ORDER BY p.unit_price DESC) AS price_rank
FROM products p
WHERE p.is_active = TRUE
ORDER BY p.name;

-- Q3: category sizes.
SELECT c.name, COUNT(p.id) AS product_count, SUM(p.stock) AS units_in_stock
FROM categories c
LEFT JOIN products p ON p.category_id = c.id
GROUP BY c.name
HAVING COUNT(p.id) > 0
ORDER BY product_count DESC, c.name;

-- Q4: new vs legacy users (UNION ALL).
SELECT 'new' AS source, id, username FROM users WHERE created_at >= '2024-01-01'
UNION ALL
SELECT 'legacy' AS source, id, username FROM users WHERE created_at < '2024-01-01'
ORDER BY id;

-- Q5: third page of recent orders.
SELECT id, status, total, placed_at FROM orders WHERE user_id = 7 ORDER BY placed_at DESC LIMIT 20 OFFSET 40;

-- Q6: SKU prefix search (trailing wildcard only).
SELECT id, sku, name FROM products WHERE sku LIKE 'SKU-10%' ORDER BY sku;

-- Q7: subcategories.
SELECT id, name FROM products WHERE category_id IN (SELECT id FROM categories WHERE parent_id IS NOT NULL) ORDER BY id;

-- Q8: orders never paid (NOT EXISTS anti-join).
SELECT o.id, o.total FROM orders o
WHERE NOT EXISTS (SELECT 1 FROM payments p WHERE p.order_id = o.id)
ORDER BY o.id;

-- Q9: fallback timestamps.
SELECT id, COALESCE(shipped_at, placed_at) AS event_time FROM orders ORDER BY event_time;

-- Q10: order size buckets (CASE with ELSE).
SELECT id, CASE WHEN total >= 1000 THEN 'big' WHEN total >= 100 THEN 'mid' ELSE 'small' END AS bucket
FROM orders ORDER BY id;

-- Q11: each customer's biggest orders (DENSE_RANK).
SELECT u.username, o.id, o.total,
       DENSE_RANK() OVER (PARTITION BY u.id ORDER BY o.total DESC) AS order_rank
FROM orders o
JOIN users u ON u.id = o.user_id
ORDER BY u.username, order_rank;

-- Q12: monthly revenue.
SELECT substr(placed_at, 1, 7) AS month, SUM(total) AS revenue, COUNT(id) AS order_count
FROM orders
WHERE status <> 'cancelled'
GROUP BY substr(placed_at, 1, 7)
ORDER BY month;

-- Q13: products nobody reviewed (LEFT JOIN anti-join).
SELECT p.id, p.sku FROM products p
LEFT JOIN reviews r ON r.product_id = p.id
WHERE r.id IS NULL
ORDER BY p.id;

-- Q14: activity summary per user (scalar subqueries).
SELECT u.username,
       (SELECT COUNT(*) FROM orders o WHERE o.user_id = u.id) AS order_count,
       (SELECT MAX(amount) FROM payments p JOIN orders o ON o.id = p.order_id WHERE o.user_id = u.id) AS max_payment
FROM users u
ORDER BY u.username;

-- Q15: category tree (recursive CTE).
WITH RECURSIVE tree AS (
    SELECT id, name, parent_id, 0 AS depth FROM categories WHERE parent_id IS NULL
    UNION ALL
    SELECT c.id, c.name, c.parent_id, t.depth + 1 FROM categories c JOIN tree t ON t.id = c.parent_id
)
SELECT id, name, depth FROM tree ORDER BY depth, name;

-- Q16: review stats per product (two CTEs).
WITH best AS (
    SELECT product_id, MAX(rating) AS top_rating FROM reviews GROUP BY product_id
),
last_seen AS (
    SELECT product_id, MAX(created_at) AS last_review_at FROM reviews GROUP BY product_id
)
SELECT p.id, p.sku, b.top_rating, l.last_review_at
FROM products p
JOIN best b ON b.product_id = p.id
JOIN last_seen l ON l.product_id = p.id
ORDER BY b.top_rating DESC, p.sku;

-- Q17: archive old sessions.
UPDATE sessions SET revoked = TRUE WHERE expires_at < '2024-05-15 00:00:00';

-- Q18: purge expired rows.
DELETE FROM sessions WHERE expires_at < '2024-04-01 00:00:00';

-- Q19: archive audit rows (INSERT ... SELECT with columns).
INSERT INTO audit_log (actor_id, action, payload, occurred_at)
SELECT actor_id, 'archived', payload, occurred_at FROM audit_log WHERE occurred_at < '2023-01-01';

-- Q20: order-over-order change (LAG).
SELECT id, placed_at, total, total - LAG(total, 1, total) OVER (ORDER BY placed_at) AS delta
FROM orders WHERE user_id = 3
ORDER BY placed_at;

-- Q21: 7-day moving revenue (window frame).
SELECT placed_at, SUM(total) OVER (ORDER BY placed_at ROWS BETWEEN 6 PRECEDING AND CURRENT ROW) AS week_revenue
FROM orders
WHERE status <> 'cancelled'
ORDER BY placed_at;

-- Q22: biggest spender per user (derived table).
SELECT user_id, MAX(spent) AS biggest_basket FROM (
    SELECT user_id, SUM(total) AS spent FROM orders WHERE status <> 'cancelled' GROUP BY user_id
) AS baskets
GROUP BY user_id
ORDER BY user_id;

-- Q23: awaiting shipment.
SELECT id, total FROM orders WHERE total BETWEEN 100 AND 500 AND shipped_at IS NULL ORDER BY id;

-- Q24: full order lines (three-table join).
SELECT u.username, o.id AS order_id, p.name AS product
FROM order_items oi
JOIN orders o ON o.id = oi.order_id
JOIN users u ON u.id = o.user_id
JOIN products p ON p.id = oi.product_id
WHERE oi.quantity >= 2
ORDER BY u.username, order_id;

-- Q25: revenue by category.
SELECT c.name AS category, SUM(oi.line_total) AS revenue
FROM order_items oi
JOIN products p ON p.id = oi.product_id
JOIN categories c ON c.id = p.category_id
JOIN orders o ON o.id = oi.order_id
WHERE o.status <> 'cancelled'
GROUP BY c.name
ORDER BY revenue DESC;

-- Q26: best sellers.
SELECT p.name, SUM(oi.quantity) AS units_sold
FROM order_items oi
JOIN products p ON p.id = oi.product_id
GROUP BY p.name
ORDER BY units_sold DESC
LIMIT 10;

-- Q27: repeat customers.
WITH orders_per_user AS (
    SELECT user_id, COUNT(id) AS order_count FROM orders GROUP BY user_id
)
SELECT u.username, opu.order_count
FROM users u
JOIN orders_per_user opu ON opu.user_id = u.id
WHERE opu.order_count > 1
ORDER BY opu.order_count DESC, u.username;

-- Q28: live sessions.
SELECT s.id, s.token, s.started_at FROM sessions s WHERE s.revoked = FALSE AND s.expires_at > '2024-05-15 00:00:00' ORDER BY s.started_at;

-- Q29: payment mix.
SELECT method, COUNT(id) AS payment_count, SUM(amount) AS collected
FROM payments
GROUP BY method
ORDER BY collected DESC;

-- Q30: addresses per user.
SELECT u.username, COUNT(a.id) AS address_count
FROM users u
LEFT JOIN addresses a ON a.user_id = u.id
GROUP BY u.username
ORDER BY address_count DESC, u.username;

-- Q31: ratings distribution this year.
SELECT r.rating, COUNT(r.id) AS review_count
FROM reviews r
JOIN users u ON u.id = r.user_id
WHERE r.created_at >= '2024-01-01'
GROUP BY r.rating
ORDER BY r.rating;

-- Q32: subcategory map (self-join).
SELECT parent.name AS parent_category, child.name AS child_category
FROM categories child
JOIN categories parent ON parent.id = child.parent_id
ORDER BY parent.name, child.name;

-- Q33: audit summary.
SELECT action, COUNT(id) AS hits FROM audit_log GROUP BY action ORDER BY hits DESC;

-- Q34: shipping speed tiers (CASE with ELSE + COALESCE).
SELECT id, COALESCE(shipped_at, placed_at) AS event_time,
       CASE WHEN total > 500 THEN 'express' ELSE 'standard' END AS shipping
FROM orders
ORDER BY event_time;

-- Q35: orders in flight.
SELECT id, placed_at FROM orders WHERE status IN ('paid', 'shipped') ORDER BY placed_at;

-- Q36: review join with reviewer (explicit columns).
SELECT p.name AS product, r.rating, u.username
FROM reviews r
JOIN products p ON p.id = r.product_id
JOIN users u ON u.id = r.user_id
WHERE r.rating < 3
ORDER BY p.name, r.rating;
