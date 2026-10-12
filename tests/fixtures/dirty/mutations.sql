-- Expected violations: 1x select-star, 1x no-where-on-mutation
SELECT * FROM users;

DELETE FROM sessions;
