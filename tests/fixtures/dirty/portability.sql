-- sqlite, default rules: 2 violations (not-in-null-trap, dialect-portability NOW()).
-- The bare SELECT also fires opt-in missing-limit.
SELECT id FROM users WHERE id NOT IN (SELECT manager_id FROM users);

UPDATE users SET seen_at = NOW() WHERE id = 1;
