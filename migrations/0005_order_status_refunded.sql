-- Add 'refunded' to OrderStatus
PRAGMA defer_foreign_keys = ON;
PRAGMA legacy_alter_table = ON;

ALTER TABLE orders RENAME TO old_orders;

CREATE TABLE orders (
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

INSERT INTO orders SELECT * FROM old_orders;

DROP TABLE old_orders;

PRAGMA legacy_alter_table = OFF;
