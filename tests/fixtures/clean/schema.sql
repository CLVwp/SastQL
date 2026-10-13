-- Clean on both dialects: explicit columns, guarded mutations.
SELECT id, email FROM users WHERE id = 1;

UPDATE users SET email = 'new@example.com' WHERE id = 1;

-- v0.2 negative controls: explicit conditions everywhere.
SELECT u.id FROM users u JOIN orders o ON o.user_id = u.id;
SELECT id, CASE WHEN total > 100 THEN 'big' ELSE 'small' END AS bucket FROM orders;
INSERT INTO users (id, email) VALUES (1, 'a@b.co');
SELECT id FROM users WHERE id = 1 ORDER BY id LIMIT 1;
