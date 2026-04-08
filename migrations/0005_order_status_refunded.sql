-- no-transaction

PRAGMA foreign_keys = OFF;

CREATE TABLE new_orders (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    plan_id TEXT NOT NULL REFERENCES nat_plans (id),
    status TEXT NOT NULL CHECK (
        status IN (
            'pending_payment', 'paid', 'provisioning', 'active', 'failed', 'cancelled', 'refunded'
        )
    ),
    total_amount NUMERIC(12, 2) NOT NULL CHECK (total_amount > 0),
    idempotency_key TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (idempotency_key)
);

INSERT INTO new_orders SELECT * FROM orders;

DROP TABLE orders;

ALTER TABLE new_orders RENAME TO orders;

-- Re-create indexes for orders
CREATE INDEX idx_orders_user_status ON orders (user_id, status);

PRAGMA foreign_keys = ON;
