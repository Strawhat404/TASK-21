-- Fund Transparency Database Schema
-- SQLite 3
-- NOTE: This file is a reference mirror of the authoritative schema defined
--       in backend/src/db.rs create_tables(). Keep both in sync.

PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'supporter',
    dnd_start TEXT NOT NULL DEFAULT '21:00',
    dnd_end TEXT NOT NULL DEFAULT '07:00',
    timezone TEXT NOT NULL DEFAULT 'UTC',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    cause TEXT NOT NULL,
    zip_code TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    goal_cents INTEGER NOT NULL,
    manager_id TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS budget_lines (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    name TEXT NOT NULL,
    allocated_cents INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS donations (
    id TEXT PRIMARY KEY,
    pledge_number TEXT UNIQUE NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id),
    donor_id TEXT NOT NULL REFERENCES users(id),
    amount_cents INTEGER NOT NULL,
    payment_method TEXT NOT NULL DEFAULT 'cash',
    is_reversal INTEGER NOT NULL DEFAULT 0,
    reversal_of TEXT REFERENCES donations(id),
    reversal_approved INTEGER,
    budget_line_id TEXT REFERENCES budget_lines(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS expenses (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    budget_line_id TEXT NOT NULL REFERENCES budget_lines(id),
    amount_cents INTEGER NOT NULL,
    description TEXT NOT NULL,
    receipt_data TEXT,
    disclosure_status TEXT NOT NULL DEFAULT 'pending',
    reviewer_id TEXT REFERENCES users(id),
    reviewer_note TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS spending_updates (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    author_id TEXT NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS update_likes (
    user_id TEXT NOT NULL REFERENCES users(id),
    update_id TEXT NOT NULL REFERENCES spending_updates(id),
    PRIMARY KEY (user_id, update_id)
);

CREATE TABLE IF NOT EXISTS comments (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    author_id TEXT NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    moderation_status TEXT NOT NULL DEFAULT 'approved',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS favorites (
    user_id TEXT NOT NULL REFERENCES users(id),
    project_id TEXT NOT NULL REFERENCES projects(id),
    PRIMARY KEY (user_id, project_id)
);

CREATE TABLE IF NOT EXISTS subscriptions (
    user_id TEXT NOT NULL REFERENCES users(id),
    project_id TEXT NOT NULL REFERENCES projects(id),
    enabled INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (user_id, project_id)
);

CREATE TABLE IF NOT EXISTS tickets (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    submitter_id TEXT NOT NULL REFERENCES users(id),
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    response TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    is_read INTEGER NOT NULL DEFAULT 0,
    is_deferred INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ops_log (
    id TEXT PRIMARY KEY,
    actor_id TEXT NOT NULL,
    actor_name TEXT NOT NULL,
    action TEXT NOT NULL,
    detail TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS nonces (
    nonce TEXT PRIMARY KEY,
    expires_at TEXT NOT NULL
);

-- Receipt / voucher uploads
CREATE TABLE IF NOT EXISTS receipts (
    id TEXT PRIMARY KEY,
    expense_id TEXT NOT NULL REFERENCES expenses(id),
    file_name TEXT NOT NULL,
    file_type TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    file_data BLOB,
    sha256_fingerprint TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'uploaded',
    rejection_reason TEXT,
    reviewer_id TEXT REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Content moderation config (singleton row)
CREATE TABLE IF NOT EXISTS moderation_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    comments_enabled INTEGER NOT NULL DEFAULT 1,
    require_pre_moderation INTEGER NOT NULL DEFAULT 0,
    sensitive_words TEXT NOT NULL DEFAULT ''
);
INSERT OR IGNORE INTO moderation_config (id, comments_enabled, require_pre_moderation, sensitive_words)
    VALUES (1, 1, 0, '');

-- Fulfillment verification
CREATE TABLE IF NOT EXISTS fulfillments (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    arrival_at TEXT,
    start_at TEXT,
    end_at TEXT,
    is_complete INTEGER NOT NULL DEFAULT 0,
    service_record_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Fulfillment checkpoint codes (OTP/QR, 10-min expiry)
CREATE TABLE IF NOT EXISTS checkpoint_codes (
    id TEXT PRIMARY KEY,
    fulfillment_id TEXT NOT NULL REFERENCES fulfillments(id),
    checkpoint TEXT NOT NULL,
    code TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    consumed INTEGER NOT NULL DEFAULT 0
);

-- Analytics events
CREATE TABLE IF NOT EXISTS analytics_events (
    id TEXT PRIMARY KEY,
    event_kind TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    user_id TEXT,
    dwell_ms INTEGER,
    is_duplicate INTEGER NOT NULL DEFAULT 0,
    is_suspicious INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Webhooks
CREATE TABLE IF NOT EXISTS webhooks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    secret TEXT NOT NULL,
    event_types TEXT NOT NULL DEFAULT '[]',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Webhook delivery log
CREATE TABLE IF NOT EXISTS webhook_delivery_log (
    id TEXT PRIMARY KEY,
    webhook_id TEXT NOT NULL REFERENCES webhooks(id),
    event_type TEXT NOT NULL,
    payload_summary TEXT NOT NULL,
    attempt INTEGER NOT NULL DEFAULT 1,
    status_code INTEGER,
    success INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_receipts_expense ON receipts(expense_id);
CREATE INDEX IF NOT EXISTS idx_receipts_fingerprint ON receipts(sha256_fingerprint);
CREATE INDEX IF NOT EXISTS idx_fulfillments_project ON fulfillments(project_id);
CREATE INDEX IF NOT EXISTS idx_events_session ON analytics_events(session_id);
CREATE INDEX IF NOT EXISTS idx_events_created ON analytics_events(created_at);
CREATE INDEX IF NOT EXISTS idx_events_kind ON analytics_events(event_kind);
CREATE INDEX IF NOT EXISTS idx_webhook_log_webhook ON webhook_delivery_log(webhook_id);
CREATE INDEX IF NOT EXISTS idx_projects_cause ON projects(cause);
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);
CREATE INDEX IF NOT EXISTS idx_projects_zip ON projects(zip_code);
CREATE INDEX IF NOT EXISTS idx_donations_project ON donations(project_id);
CREATE INDEX IF NOT EXISTS idx_donations_donor ON donations(donor_id);
CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications(user_id);
CREATE INDEX IF NOT EXISTS idx_comments_project ON comments(project_id);
CREATE INDEX IF NOT EXISTS idx_ops_log_created ON ops_log(created_at);

-- Sensitive-action confirmation tokens (server-side second-step enforcement)
CREATE TABLE IF NOT EXISTS sensitive_confirmations (
    token TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    action TEXT NOT NULL,
    target_id TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    consumed INTEGER NOT NULL DEFAULT 0
);

-- Immutability enforcement: block UPDATE and DELETE on ops_log
CREATE TRIGGER IF NOT EXISTS ops_log_no_update
    BEFORE UPDATE ON ops_log
    BEGIN
        SELECT RAISE(ABORT, 'ops_log is immutable: updates are not allowed');
    END;
CREATE TRIGGER IF NOT EXISTS ops_log_no_delete
    BEFORE DELETE ON ops_log
    BEGIN
        SELECT RAISE(ABORT, 'ops_log is immutable: deletions are not allowed');
    END;
