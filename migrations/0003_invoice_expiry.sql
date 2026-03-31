PRAGMA foreign_keys = OFF;

ALTER TABLE invoices RENAME TO invoices_old;

CREATE TABLE invoices (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    order_id TEXT REFERENCES orders (id),
    amount NUMERIC(12, 2) NOT NULL CHECK (amount > 0),
    currency TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN (
            'open',
            'paid',
            'failed',
            'refunded',
            'expired'
        )
    ),
    external_payment_ref TEXT,
    due_at TEXT NOT NULL,
    paid_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (external_payment_ref)
);

INSERT INTO
    invoices (
        id,
        user_id,
        order_id,
        amount,
        currency,
        status,
        external_payment_ref,
        due_at,
        paid_at,
        created_at
    )
SELECT
    id,
    user_id,
    order_id,
    amount,
    currency,
    CASE
        WHEN status = 'open'
        AND datetime(due_at) <= datetime('now') THEN 'expired'
        ELSE status
    END,
    external_payment_ref,
    due_at,
    paid_at,
    created_at
FROM invoices_old;

DROP TABLE invoices_old;

CREATE INDEX idx_invoices_user_status ON invoices (user_id, status);

PRAGMA foreign_keys = ON;