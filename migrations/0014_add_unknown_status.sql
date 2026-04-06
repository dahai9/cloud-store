-- Add 'unknown' to instances status check constraint
PRAGMA foreign_keys = OFF;

CREATE TABLE instances_new (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    node_id TEXT NOT NULL REFERENCES nodes (id),
    order_id TEXT NOT NULL REFERENCES orders (id),
    plan_id TEXT NOT NULL REFERENCES nat_plans (id),
    provider_instance_id TEXT,
    status TEXT NOT NULL CHECK (
        status IN (
            'pending',
            'starting',
            'running',
            'stopped',
            'suspended',
            'deleted',
            'unknown'
        )
    ),
    os_template TEXT NOT NULL,
    root_password TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO instances_new (id, user_id, node_id, order_id, plan_id, provider_instance_id, status, os_template, root_password, created_at, updated_at)
SELECT id, user_id, node_id, order_id, plan_id, provider_instance_id, status, os_template, root_password, created_at, updated_at FROM instances;

DROP TABLE instances;
ALTER TABLE instances_new RENAME TO instances;

CREATE INDEX idx_instances_user_id ON instances (user_id);
CREATE INDEX idx_instances_node_id ON instances (node_id);
CREATE INDEX idx_instances_status ON instances (status);

PRAGMA foreign_keys = ON;
