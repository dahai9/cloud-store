CREATE UNIQUE INDEX IF NOT EXISTS idx_users_single_admin ON users (role)
WHERE
    role = 'admin';