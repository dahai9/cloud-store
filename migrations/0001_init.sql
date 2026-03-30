PRAGMA foreign_keys = ON;

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'admin')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE nat_plans (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    memory_mb INT NOT NULL CHECK (memory_mb > 0),
    storage_gb INT NOT NULL CHECK (storage_gb > 0),
    monthly_price NUMERIC(12, 2) NOT NULL CHECK (monthly_price > 0),
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE nodes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    region TEXT NOT NULL,
    total_capacity INT NOT NULL CHECK (total_capacity > 0),
    used_capacity INT NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE nat_port_leases (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL REFERENCES nodes (id) ON DELETE CASCADE,
    public_ip TEXT NOT NULL,
    start_port INT NOT NULL CHECK (start_port > 0),
    end_port INT NOT NULL CHECK (end_port >= start_port),
    reserved INTEGER NOT NULL DEFAULT 0 CHECK (reserved IN (0, 1)),
    reserved_for_order_id TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (
        node_id,
        public_ip,
        start_port,
        end_port
    )
);

CREATE TABLE orders (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    plan_id TEXT NOT NULL REFERENCES nat_plans (id),
    status TEXT NOT NULL CHECK (
        status IN (
            'pending_payment',
            'paid',
            'provisioning',
            'active',
            'failed',
            'cancelled'
        )
    ),
    total_amount NUMERIC(12, 2) NOT NULL CHECK (total_amount > 0),
    idempotency_key TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (idempotency_key)
);

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
            'refunded'
        )
    ),
    external_payment_ref TEXT,
    due_at TEXT NOT NULL,
    paid_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (external_payment_ref)
);

CREATE TABLE subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    order_id TEXT NOT NULL REFERENCES orders (id),
    status TEXT NOT NULL CHECK (
        status IN (
            'active',
            'grace_period',
            'suspended',
            'cancelled'
        )
    ),
    current_period_start TEXT NOT NULL,
    current_period_end TEXT NOT NULL,
    auto_renew INTEGER NOT NULL DEFAULT 1 CHECK (auto_renew IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE payment_webhook_events (
    id TEXT PRIMARY KEY,
    gateway TEXT NOT NULL,
    event_id TEXT NOT NULL,
    payload TEXT NOT NULL,
    processed_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (gateway, event_id)
);

CREATE TABLE support_tickets (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    category TEXT NOT NULL,
    priority TEXT NOT NULL,
    subject TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE support_messages (
    id TEXT PRIMARY KEY,
    ticket_id TEXT NOT NULL REFERENCES support_tickets (id) ON DELETE CASCADE,
    sender_user_id TEXT REFERENCES users (id),
    message TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE support_attachments (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES support_messages (id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    storage_key TEXT NOT NULL,
    content_type TEXT NOT NULL,
    file_size BIGINT NOT NULL CHECK (file_size >= 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_orders_user_status ON orders (user_id, status);

CREATE INDEX idx_invoices_user_status ON invoices (user_id, status);

CREATE INDEX idx_subscriptions_period_end ON subscriptions (current_period_end);

CREATE INDEX idx_tickets_user_status ON support_tickets (user_id, status);