-- Update balance_transactions.type to include 'purchase'
-- SQLite does not support ALTER TABLE to change CHECK constraints, so we recreate the table.

PRAGMA foreign_keys = OFF;

CREATE TABLE balance_transactions_new (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    amount NUMERIC(12, 2) NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('recharge', 'refund', 'auto_renew', 'admin_adjustment', 'purchase')),
    description TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO balance_transactions_new (id, user_id, amount, type, description, created_at)
SELECT id, user_id, amount, type, description, created_at FROM balance_transactions;

DROP TABLE balance_transactions;

ALTER TABLE balance_transactions_new RENAME TO balance_transactions;

CREATE INDEX idx_balance_transactions_user_id ON balance_transactions(user_id);

PRAGMA foreign_keys = ON;
