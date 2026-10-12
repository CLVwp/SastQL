-- Clean on both dialects: explicit columns, guarded mutations.
SELECT id, email FROM users WHERE id = 1;

UPDATE users SET email = 'new@example.com' WHERE id = 1;
