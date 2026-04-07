-- Balance, Tickets, and Auto-Renew improvements

-- Users 表扩展：增加余额字段
ALTER TABLE users ADD COLUMN balance NUMERIC(12, 2) NOT NULL DEFAULT 0.00;

-- 新增 Balance Transactions (余额流水表)
CREATE TABLE balance_transactions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    amount NUMERIC(12, 2) NOT NULL, -- 正数表示增加(充值/退款)，负数表示扣除(续费购买)
    type TEXT NOT NULL CHECK (type IN ('recharge', 'refund', 'auto_renew', 'admin_adjustment')),
    description TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Instances 表扩展：增加自动续费开关
ALTER TABLE instances ADD COLUMN auto_renew INTEGER NOT NULL DEFAULT 0 CHECK (auto_renew IN (0, 1));

-- 为流水表创建索引
CREATE INDEX idx_balance_transactions_user_id ON balance_transactions(user_id);
