-- null-comparison: comparing with NULL yields UNKNOWN; the WHERE never matches.
UPDATE users SET locked = true WHERE last_login = NULL;
