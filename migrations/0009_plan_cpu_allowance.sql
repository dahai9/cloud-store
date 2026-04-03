ALTER TABLE nat_plans
ADD COLUMN cpu_allowance_pct INT NOT NULL DEFAULT 100;

UPDATE nat_plans
SET
    cpu_allowance_pct = CASE
        WHEN cpu_cores < 1 THEN 100
        ELSE cpu_cores * 100
    END;