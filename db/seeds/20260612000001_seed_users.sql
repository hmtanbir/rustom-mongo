-- Insert seed users
-- Note: password_digest for these users is 'password' hashed with Argon2id
INSERT INTO users (name, email, password_digest, role, status)
SELECT 'Admin User', 'admin@rustom.project', '$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs', 0, 1
WHERE NOT EXISTS (SELECT 1 FROM users);
