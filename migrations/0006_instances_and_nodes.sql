-- Add new resource specific capacities and API connection details
ALTER TABLE nodes ADD COLUMN cpu_cores_total INT NOT NULL DEFAULT 0;
ALTER TABLE nodes ADD COLUMN memory_mb_total INT NOT NULL DEFAULT 0;
ALTER TABLE nodes ADD COLUMN storage_gb_total INT NOT NULL DEFAULT 0;

ALTER TABLE nodes ADD COLUMN cpu_cores_used INT NOT NULL DEFAULT 0;
ALTER TABLE nodes ADD COLUMN memory_mb_used INT NOT NULL DEFAULT 0;
ALTER TABLE nodes ADD COLUMN storage_gb_used INT NOT NULL DEFAULT 0;

ALTER TABLE nodes ADD COLUMN api_endpoint TEXT;
ALTER TABLE nodes ADD COLUMN api_token TEXT;

-- Drop old capacity columns (supported in SQLite 3.35.0+)
ALTER TABLE nodes DROP COLUMN total_capacity;
ALTER TABLE nodes DROP COLUMN used_capacity;

-- Create instances table
CREATE TABLE instances (
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
            'deleted'
        )
    ),
    os_template TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_instances_user_id ON instances (user_id);
CREATE INDEX idx_instances_node_id ON instances (node_id);
CREATE INDEX idx_instances_status ON instances (status);
