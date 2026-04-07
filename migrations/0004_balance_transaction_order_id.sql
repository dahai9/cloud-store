-- Add order_id to balance_transactions to link purchases to orders
ALTER TABLE balance_transactions ADD COLUMN order_id TEXT REFERENCES orders(id);
