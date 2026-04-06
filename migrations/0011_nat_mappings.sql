CREATE TABLE nat_mappings (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    internal_port INT NOT NULL,
    external_port INT NOT NULL,
    protocol TEXT NOT NULL CHECK (protocol IN ('tcp', 'udp')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_nat_mappings_instance_id ON nat_mappings (instance_id);
CREATE UNIQUE INDEX idx_nat_mappings_external ON nat_mappings (external_port, protocol);
