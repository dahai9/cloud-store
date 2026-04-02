ALTER TABLE nat_plans ADD COLUMN max_inventory INT;

ALTER TABLE nat_plans
ADD COLUMN sold_inventory INT NOT NULL DEFAULT 0 CHECK (sold_inventory >= 0);

ALTER TABLE users
ADD COLUMN disabled INTEGER NOT NULL DEFAULT 0 CHECK (disabled IN (0, 1));

CREATE INDEX IF NOT EXISTS idx_nat_plans_active ON nat_plans (active);

CREATE INDEX IF NOT EXISTS idx_nat_plans_inventory ON nat_plans (
    code,
    active,
    sold_inventory,
    max_inventory
);

CREATE INDEX IF NOT EXISTS idx_users_disabled ON users (disabled);