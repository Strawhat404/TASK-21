use parking_lot::Mutex;
use rusqlite::{params, Connection, Result as SqlResult};
use std::sync::Arc;

pub type DbPool = Arc<Mutex<Connection>>;

pub fn init_db(path: &str) -> DbPool {
    let conn = Connection::open(path).expect("Failed to open SQLite database");
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .expect("Failed to set pragmas");
    create_tables(&conn);
    seed_sample_data(&conn);
    Arc::new(Mutex::new(conn))
}

fn create_tables(conn: &Connection) {
    conn.execute_batch(
        "
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

        -- Sensitive-action confirmation tokens (server-side second-step enforcement)
        CREATE TABLE IF NOT EXISTS sensitive_confirmations (
            token TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id),
            action TEXT NOT NULL,
            target_id TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            consumed INTEGER NOT NULL DEFAULT 0
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
        "
    )
    .expect("Failed to create tables");
}

/// Seed two sample projects on first run so the portal isn't empty.
fn seed_sample_data(conn: &Connection) {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
        .unwrap_or(0);
    if count > 0 {
        return; // already seeded
    }

    // Create a sample Project Manager user with a known password hash
    // Password: "SeedPass1" (argon2 hash generated at build time is not feasible,
    // so we use a pre-computed hash; this account is for demo display only)
    let pm_id = "seed-pm-00000001";
    let pm_hash = crate::auth::hash_password("SeedPass1").unwrap_or_default();
    conn.execute(
        "INSERT OR IGNORE INTO users (id, email, display_name, password_hash, role) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![pm_id, "manager@example.org", "Community Fund Manager", pm_hash, "project_manager"],
    ).ok();

    // ── Project 1: Education ──
    let p1 = "seed-proj-education";
    conn.execute(
        "INSERT INTO projects (id, title, description, cause, zip_code, status, goal_cents, manager_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            p1,
            "Neighborhood STEM Lab for Kids",
            "Help us build a hands-on science and technology lab at the Riverside Community Center. \
             Funds cover equipment, curriculum materials, and instructor training so every child \
             in ZIP 90210 has access to free after-school STEM workshops.",
            "education",
            "90210",
            "active",
            250_000i64, // $2,500.00
            pm_id,
        ],
    ).ok();
    conn.execute(
        "INSERT INTO budget_lines (id, project_id, name, allocated_cents) VALUES (?1, ?2, ?3, ?4)",
        params!["seed-bl-e1", p1, "Lab Equipment", 120_000i64],
    ).ok();
    conn.execute(
        "INSERT INTO budget_lines (id, project_id, name, allocated_cents) VALUES (?1, ?2, ?3, ?4)",
        params!["seed-bl-e2", p1, "Curriculum & Books", 60_000i64],
    ).ok();
    conn.execute(
        "INSERT INTO budget_lines (id, project_id, name, allocated_cents) VALUES (?1, ?2, ?3, ?4)",
        params!["seed-bl-e3", p1, "Instructor Training", 70_000i64],
    ).ok();

    // A sample donation so the progress bar is visible
    conn.execute(
        "INSERT INTO donations (id, pledge_number, project_id, donor_id, amount_cents, payment_method) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["seed-don-e1", "PLG-000001", p1, pm_id, 75_000i64, "cash"],
    ).ok();

    // ── Project 2: Environment ──
    let p2 = "seed-proj-greenpark";
    conn.execute(
        "INSERT INTO projects (id, title, description, cause, zip_code, status, goal_cents, manager_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            p2,
            "Green Park Restoration Drive",
            "Restore the Oak Street Park with native plantings, new walking paths, and \
             accessible playground equipment. This project partners with local volunteers \
             and the city parks department to revitalize 3 acres of neglected green space.",
            "environment",
            "60614",
            "active",
            500_000i64, // $5,000.00
            pm_id,
        ],
    ).ok();
    conn.execute(
        "INSERT INTO budget_lines (id, project_id, name, allocated_cents) VALUES (?1, ?2, ?3, ?4)",
        params!["seed-bl-g1", p2, "Native Plants & Soil", 150_000i64],
    ).ok();
    conn.execute(
        "INSERT INTO budget_lines (id, project_id, name, allocated_cents) VALUES (?1, ?2, ?3, ?4)",
        params!["seed-bl-g2", p2, "Walking Paths", 200_000i64],
    ).ok();
    conn.execute(
        "INSERT INTO budget_lines (id, project_id, name, allocated_cents) VALUES (?1, ?2, ?3, ?4)",
        params!["seed-bl-g3", p2, "Playground Equipment", 150_000i64],
    ).ok();

    conn.execute(
        "INSERT INTO donations (id, pledge_number, project_id, donor_id, amount_cents, payment_method) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["seed-don-g1", "PLG-000002", p2, pm_id, 120_000i64, "check"],
    ).ok();

    // Log the seeding
    conn.execute(
        "INSERT INTO ops_log (id, actor_id, actor_name, action, detail) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["seed-log-01", "system", "System", "seed", "Inserted sample projects for demo"],
    ).ok();
}

// ── Nonce management ──

pub fn store_nonce(db: &DbPool, nonce: &str, expires_at: &str) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT OR IGNORE INTO nonces (nonce, expires_at) VALUES (?1, ?2)",
        params![nonce, expires_at],
    )?;
    Ok(())
}

pub fn consume_nonce(db: &DbPool, nonce: &str) -> SqlResult<bool> {
    let conn = db.lock();
    // Clean expired
    conn.execute(
        "DELETE FROM nonces WHERE expires_at < datetime('now')",
        [],
    )?;
    let deleted = conn.execute(
        "DELETE FROM nonces WHERE nonce = ?1 AND expires_at >= datetime('now')",
        params![nonce],
    )?;
    Ok(deleted > 0)
}

// ── Sensitive action confirmation ──

/// Create a short-lived confirmation token for a sensitive action.
/// Returns the token string.
pub fn create_confirmation_token(db: &DbPool, user_id: &str, action: &str, target_id: &str) -> SqlResult<String> {
    let conn = db.lock();
    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::minutes(5))
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    conn.execute(
        "INSERT INTO sensitive_confirmations (token, user_id, action, target_id, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![token, user_id, action, target_id, expires_at],
    )?;
    Ok(token)
}

/// Consume a confirmation token. Returns true if the token was valid, unexpired, and unconsumed.
pub fn consume_confirmation_token(db: &DbPool, token: &str, user_id: &str, action: &str, target_id: &str) -> bool {
    let conn = db.lock();
    // Clean expired tokens
    let _ = conn.execute("DELETE FROM sensitive_confirmations WHERE expires_at < datetime('now')", []);
    let rows = conn.execute(
        "UPDATE sensitive_confirmations SET consumed = 1 \
         WHERE token = ?1 AND user_id = ?2 AND action = ?3 AND target_id = ?4 \
         AND consumed = 0 AND expires_at >= datetime('now')",
        params![token, user_id, action, target_id],
    ).unwrap_or(0);
    rows > 0
}

// ── Ops log ──

pub fn append_ops_log(db: &DbPool, actor_id: &str, actor_name: &str, action: &str, detail: &str) {
    let conn = db.lock();
    let id = uuid::Uuid::new_v4().to_string();
    let _ = conn.execute(
        "INSERT INTO ops_log (id, actor_id, actor_name, action, detail) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, actor_id, actor_name, action, detail],
    );
}

pub fn get_ops_log(db: &DbPool, limit: i64, offset: i64) -> Vec<common::OpsLogEntry> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare("SELECT id, actor_id, actor_name, action, detail, created_at FROM ops_log ORDER BY created_at DESC LIMIT ?1 OFFSET ?2")
        .unwrap();
    stmt.query_map(params![limit, offset], |row| {
        Ok(common::OpsLogEntry {
            id: row.get(0)?,
            actor_id: row.get(1)?,
            actor_name: row.get(2)?,
            action: row.get(3)?,
            detail: row.get(4)?,
            created_at: row.get(5)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

// ── User queries ──

pub fn create_user(
    db: &DbPool,
    id: &str,
    email: &str,
    display_name: &str,
    password_hash: &str,
    role: &str,
) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO users (id, email, display_name, password_hash, role) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, email, display_name, password_hash, role],
    )?;
    Ok(())
}

pub fn get_user_by_email(db: &DbPool, email: &str) -> Option<(String, String, String, String, String, String, String)> {
    let conn = db.lock();
    conn.query_row(
        "SELECT id, email, display_name, password_hash, role, dnd_start, dnd_end FROM users WHERE email = ?1",
        params![email],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
    )
    .ok()
}

pub fn get_user_by_id(db: &DbPool, id: &str) -> Option<common::UserProfile> {
    let conn = db.lock();
    conn.query_row(
        "SELECT id, email, display_name, role, dnd_start, dnd_end, timezone, created_at FROM users WHERE id = ?1",
        params![id],
        |row| {
            let role_str: String = row.get(3)?;
            Ok(common::UserProfile {
                id: row.get(0)?,
                email: row.get(1)?,
                display_name: row.get(2)?,
                role: common::Role::from_str(&role_str).unwrap_or(common::Role::Supporter),
                dnd_start: row.get(4)?,
                dnd_end: row.get(5)?,
                timezone: row.get(6)?,
                created_at: row.get(7)?,
            })
        },
    )
    .ok()
}

pub fn get_user_password_hash(db: &DbPool, user_id: &str) -> Option<String> {
    let conn = db.lock();
    conn.query_row(
        "SELECT password_hash FROM users WHERE id = ?1",
        params![user_id],
        |row| row.get(0),
    )
    .ok()
}

pub fn update_dnd(db: &DbPool, user_id: &str, start: &str, end: &str, timezone: Option<&str>) -> SqlResult<()> {
    let conn = db.lock();
    if let Some(tz) = timezone {
        conn.execute(
            "UPDATE users SET dnd_start = ?1, dnd_end = ?2, timezone = ?3 WHERE id = ?4",
            params![start, end, tz, user_id],
        )?;
    } else {
        conn.execute(
            "UPDATE users SET dnd_start = ?1, dnd_end = ?2 WHERE id = ?3",
            params![start, end, user_id],
        )?;
    }
    Ok(())
}

pub fn update_user_role(db: &DbPool, user_id: &str, role: &str) -> SqlResult<usize> {
    let conn = db.lock();
    conn.execute(
        "UPDATE users SET role = ?1 WHERE id = ?2",
        params![role, user_id],
    )
}

pub fn count_administrators(db: &DbPool) -> i64 {
    let conn = db.lock();
    conn.query_row(
        "SELECT COUNT(*) FROM users WHERE role = 'administrator'",
        [],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

pub fn get_budget_line_project_id(db: &DbPool, budget_line_id: &str) -> Option<String> {
    let conn = db.lock();
    conn.query_row(
        "SELECT project_id FROM budget_lines WHERE id = ?1",
        params![budget_line_id],
        |row| row.get(0),
    )
    .ok()
}

// ── Project queries ──

pub fn create_project(
    db: &DbPool,
    id: &str,
    title: &str,
    description: &str,
    cause: &str,
    zip_code: &str,
    goal_cents: i64,
    manager_id: &str,
    budget_lines: &[(String, String, i64)], // (id, name, allocated_cents)
) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO projects (id, title, description, cause, zip_code, goal_cents, manager_id, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active')",
        params![id, title, description, cause, zip_code, goal_cents, manager_id],
    )?;
    for (bl_id, name, cents) in budget_lines {
        conn.execute(
            "INSERT INTO budget_lines (id, project_id, name, allocated_cents) VALUES (?1, ?2, ?3, ?4)",
            params![bl_id, id, name, cents],
        )?;
    }
    Ok(())
}

pub fn list_projects(db: &DbPool, filter: &common::ProjectFilter, limit: i64, offset: i64) -> (Vec<common::ProjectSummary>, i64) {
    let conn = db.lock();
    let mut where_clauses = vec!["1=1".to_string()];
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref cause) = filter.cause {
        param_values.push(Box::new(cause.clone()));
        where_clauses.push(format!("p.cause = ?{}", param_values.len()));
    }
    if let Some(ref status) = filter.status {
        param_values.push(Box::new(status.clone()));
        where_clauses.push(format!("p.status = ?{}", param_values.len()));
    }
    if let Some(ref zip) = filter.zip_code {
        param_values.push(Box::new(zip.clone()));
        where_clauses.push(format!("p.zip_code = ?{}", param_values.len()));
    }
    if let Some(ref search) = filter.search {
        // Escape LIKE metacharacters before wrapping in wildcards
        let escaped = search.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        param_values.push(Box::new(format!("%{}%", escaped)));
        where_clauses.push(format!("(p.title LIKE ?{} ESCAPE '\\' OR p.description LIKE ?{} ESCAPE '\\')", param_values.len(), param_values.len()));
    }

    let where_sql = where_clauses.join(" AND ");

    let count_sql = format!(
        "SELECT COUNT(*) FROM projects p WHERE {}",
        where_sql
    );
    let total: i64 = {
        let mut stmt = conn.prepare(&count_sql).unwrap();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        stmt.query_row(param_refs.as_slice(), |row| row.get(0)).unwrap_or(0)
    };

    let query_sql = format!(
        "SELECT p.id, p.title, p.cause, p.zip_code, p.status, p.goal_cents, \
         COALESCE((SELECT SUM(d.amount_cents) FROM donations d WHERE d.project_id = p.id AND (d.is_reversal = 0 OR d.reversal_approved = 1)), 0), \
         COALESCE((SELECT SUM(e.amount_cents) FROM expenses e WHERE e.project_id = p.id AND e.disclosure_status = 'approved'), 0), \
         u.display_name, p.created_at \
         FROM projects p JOIN users u ON p.manager_id = u.id \
         WHERE {} ORDER BY p.created_at DESC LIMIT ?{} OFFSET ?{}",
        where_sql,
        param_values.len() + 1,
        param_values.len() + 2
    );
    param_values.push(Box::new(limit));
    param_values.push(Box::new(offset));

    let mut stmt = conn.prepare(&query_sql).unwrap();
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
    let items = stmt
        .query_map(param_refs.as_slice(), |row| {
            let status_str: String = row.get(4)?;
            Ok(common::ProjectSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                cause: row.get(2)?,
                zip_code: row.get(3)?,
                status: common::ProjectStatus::from_str(&status_str).unwrap_or(common::ProjectStatus::Draft),
                goal_cents: row.get(5)?,
                raised_cents: row.get(6)?,
                spent_cents: row.get(7)?,
                manager_name: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    (items, total)
}

pub fn get_project_detail(db: &DbPool, project_id: &str) -> Option<common::ProjectDetail> {
    let conn = db.lock();
    let project = conn.query_row(
        "SELECT p.id, p.title, p.description, p.cause, p.zip_code, p.status, p.goal_cents, \
         p.manager_id, u.display_name, p.created_at, \
         COALESCE((SELECT SUM(d.amount_cents) FROM donations d WHERE d.project_id = p.id AND (d.is_reversal = 0 OR d.reversal_approved = 1)), 0), \
         COALESCE((SELECT SUM(e.amount_cents) FROM expenses e WHERE e.project_id = p.id AND e.disclosure_status = 'approved'), 0) \
         FROM projects p JOIN users u ON p.manager_id = u.id WHERE p.id = ?1",
        params![project_id],
        |row| {
            let status_str: String = row.get(5)?;
            Ok(common::ProjectDetail {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                cause: row.get(3)?,
                zip_code: row.get(4)?,
                status: common::ProjectStatus::from_str(&status_str).unwrap_or(common::ProjectStatus::Draft),
                goal_cents: row.get(6)?,
                manager_id: row.get(7)?,
                manager_name: row.get(8)?,
                created_at: row.get(9)?,
                raised_cents: row.get(10)?,
                spent_cents: row.get(11)?,
                budget_lines: Vec::new(),
                updates: Vec::new(),
            })
        },
    ).ok()?;

    let mut budget_stmt = conn
        .prepare(
            "SELECT bl.id, bl.project_id, bl.name, bl.allocated_cents, \
             COALESCE((SELECT SUM(e.amount_cents) FROM expenses e WHERE e.budget_line_id = bl.id AND e.disclosure_status = 'approved'), 0) \
             FROM budget_lines bl WHERE bl.project_id = ?1"
        )
        .unwrap();
    let budget_lines: Vec<common::BudgetLine> = budget_stmt
        .query_map(params![project_id], |row| {
            Ok(common::BudgetLine {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                allocated_cents: row.get(3)?,
                spent_cents: row.get(4)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let mut update_stmt = conn
        .prepare(
            "SELECT su.id, su.project_id, su.title, su.body, u.display_name, \
             (SELECT COUNT(*) FROM update_likes ul WHERE ul.update_id = su.id), su.created_at \
             FROM spending_updates su JOIN users u ON su.author_id = u.id \
             WHERE su.project_id = ?1 ORDER BY su.created_at DESC"
        )
        .unwrap();
    let updates: Vec<common::SpendingUpdate> = update_stmt
        .query_map(params![project_id], |row| {
            Ok(common::SpendingUpdate {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                body: row.get(3)?,
                author_name: row.get(4)?,
                like_count: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    Some(common::ProjectDetail {
        budget_lines,
        updates,
        ..project
    })
}

pub fn get_project_manager_id(db: &DbPool, project_id: &str) -> Option<String> {
    let conn = db.lock();
    conn.query_row(
        "SELECT manager_id FROM projects WHERE id = ?1",
        params![project_id],
        |row| row.get(0),
    )
    .ok()
}

pub fn get_ticket_project_id(db: &DbPool, ticket_id: &str) -> Option<String> {
    let conn = db.lock();
    conn.query_row(
        "SELECT project_id FROM tickets WHERE id = ?1",
        params![ticket_id],
        |row| row.get(0),
    )
    .ok()
}

pub fn get_fulfillment_project_id(db: &DbPool, fulfillment_id: &str) -> Option<String> {
    let conn = db.lock();
    conn.query_row(
        "SELECT project_id FROM fulfillments WHERE id = ?1",
        params![fulfillment_id],
        |row| row.get(0),
    )
    .ok()
}

pub fn get_expense_project_id(db: &DbPool, expense_id: &str) -> Option<String> {
    let conn = db.lock();
    conn.query_row(
        "SELECT project_id FROM expenses WHERE id = ?1",
        params![expense_id],
        |row| row.get(0),
    )
    .ok()
}

pub fn update_project_status(db: &DbPool, project_id: &str, status: &str) -> SqlResult<usize> {
    let conn = db.lock();
    conn.execute(
        "UPDATE projects SET status = ?1 WHERE id = ?2",
        params![status, project_id],
    )
}

// ── Donation queries ──

pub fn create_donation(
    db: &DbPool,
    id: &str,
    pledge_number: &str,
    project_id: &str,
    donor_id: &str,
    amount_cents: i64,
    payment_method: &str,
    is_reversal: bool,
    reversal_of: Option<&str>,
    budget_line_id: Option<&str>,
) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO donations (id, pledge_number, project_id, donor_id, amount_cents, payment_method, is_reversal, reversal_of, budget_line_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![id, pledge_number, project_id, donor_id, amount_cents, payment_method, is_reversal as i64, reversal_of, budget_line_id],
    )?;
    Ok(())
}

fn map_donation_row(row: &rusqlite::Row) -> rusqlite::Result<common::DonationRecord> {
    Ok(common::DonationRecord {
        id: row.get(0)?,
        pledge_number: row.get(1)?,
        project_id: row.get(2)?,
        project_title: row.get(3)?,
        donor_id: row.get(4)?,
        amount_cents: row.get(5)?,
        payment_method: row.get(6)?,
        is_reversal: row.get::<_, i64>(7)? != 0,
        reversal_of: row.get(8)?,
        reversal_approved: row.get::<_, Option<i64>>(9)?.map(|v| v != 0),
        budget_line_id: row.get(10)?,
        budget_line_name: row.get(11)?,
        created_at: row.get(12)?,
    })
}

const DONATION_SELECT: &str = "SELECT d.id, d.pledge_number, d.project_id, p.title, d.donor_id, d.amount_cents, \
     d.payment_method, d.is_reversal, d.reversal_of, d.reversal_approved, \
     d.budget_line_id, bl.name, d.created_at \
     FROM donations d \
     JOIN projects p ON d.project_id = p.id \
     LEFT JOIN budget_lines bl ON d.budget_line_id = bl.id";

pub fn get_donation(db: &DbPool, donation_id: &str) -> Option<common::DonationRecord> {
    let conn = db.lock();
    conn.query_row(
        &format!("{} WHERE d.id = ?1", DONATION_SELECT),
        params![donation_id],
        map_donation_row,
    )
    .ok()
}

pub fn list_user_donations(db: &DbPool, user_id: &str) -> Vec<common::DonationRecord> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(&format!("{} WHERE d.donor_id = ?1 ORDER BY d.created_at DESC", DONATION_SELECT))
        .unwrap();
    stmt.query_map(params![user_id], map_donation_row)
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}

pub fn approve_reversal(db: &DbPool, donation_id: &str, approved: bool) -> SqlResult<usize> {
    let conn = db.lock();
    conn.execute(
        "UPDATE donations SET reversal_approved = ?1 WHERE id = ?2 AND is_reversal = 1",
        params![approved as i64, donation_id],
    )
}

pub fn list_pending_reversals(db: &DbPool) -> Vec<common::DonationRecord> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(&format!("{} WHERE d.is_reversal = 1 AND d.reversal_approved IS NULL ORDER BY d.created_at DESC", DONATION_SELECT))
        .unwrap();
    stmt.query_map([], map_donation_row)
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}

// ── Expense queries ──

pub fn create_expense(
    db: &DbPool,
    id: &str,
    project_id: &str,
    budget_line_id: &str,
    amount_cents: i64,
    description: &str,
    receipt_data: Option<&str>,
) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO expenses (id, project_id, budget_line_id, amount_cents, description, receipt_data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, project_id, budget_line_id, amount_cents, description, receipt_data],
    )?;
    Ok(())
}

pub fn list_expenses(db: &DbPool, project_id: &str, encryption_key: &[u8; 32]) -> Vec<common::ExpenseRecord> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.project_id, e.budget_line_id, bl.name, e.amount_cents, \
             e.description, e.receipt_data, e.disclosure_status, e.reviewer_note, e.created_at \
             FROM expenses e JOIN budget_lines bl ON e.budget_line_id = bl.id \
             WHERE e.project_id = ?1 ORDER BY e.created_at DESC"
        )
        .unwrap();
    let results: Vec<common::ExpenseRecord> = stmt.query_map(params![project_id], |row| {
        let status_str: String = row.get(7)?;
        Ok(common::ExpenseRecord {
            id: row.get(0)?,
            project_id: row.get(1)?,
            budget_line_id: row.get(2)?,
            budget_line_name: row.get(3)?,
            amount_cents: row.get(4)?,
            description: row.get(5)?,
            receipt_url: row.get(6)?,
            disclosure_status: common::DisclosureStatus::from_str(&status_str)
                .unwrap_or(common::DisclosureStatus::Pending),
            reviewer_note: row.get(8)?,
            created_at: row.get(9)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect();
    // Decrypt reviewer notes
    results.into_iter().map(|mut e| {
        e.reviewer_note = decrypt_reviewer_note(e.reviewer_note, encryption_key);
        e
    }).collect()
}

pub fn list_pending_expenses(db: &DbPool, encryption_key: &[u8; 32]) -> Vec<common::ExpenseRecord> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.project_id, e.budget_line_id, bl.name, e.amount_cents, \
             e.description, e.receipt_data, e.disclosure_status, e.reviewer_note, e.created_at \
             FROM expenses e JOIN budget_lines bl ON e.budget_line_id = bl.id \
             WHERE e.disclosure_status = 'pending' ORDER BY e.created_at ASC"
        )
        .unwrap();
    let results: Vec<common::ExpenseRecord> = stmt.query_map([], |row| {
        let status_str: String = row.get(7)?;
        Ok(common::ExpenseRecord {
            id: row.get(0)?,
            project_id: row.get(1)?,
            budget_line_id: row.get(2)?,
            budget_line_name: row.get(3)?,
            amount_cents: row.get(4)?,
            description: row.get(5)?,
            receipt_url: row.get(6)?,
            disclosure_status: common::DisclosureStatus::from_str(&status_str)
                .unwrap_or(common::DisclosureStatus::Pending),
            reviewer_note: row.get(8)?,
            created_at: row.get(9)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect();
    results.into_iter().map(|mut e| {
        e.reviewer_note = decrypt_reviewer_note(e.reviewer_note, encryption_key);
        e
    }).collect()
}

pub fn review_expense(db: &DbPool, expense_id: &str, approved: bool, reviewer_id: &str, note: Option<&str>, encryption_key: &[u8; 32]) -> SqlResult<usize> {
    let conn = db.lock();
    let status = if approved { "approved" } else { "rejected" };
    // Encrypt reviewer_note at rest
    let encrypted_note: Option<String> = note.and_then(|n| {
        if n.is_empty() { None } else { crate::crypto::encrypt(n, encryption_key).ok() }
    });
    conn.execute(
        "UPDATE expenses SET disclosure_status = ?1, reviewer_id = ?2, reviewer_note = ?3 WHERE id = ?4",
        params![status, reviewer_id, encrypted_note, expense_id],
    )
}

/// Decrypt a reviewer_note value that was encrypted at rest.
fn decrypt_reviewer_note(encrypted: Option<String>, key: &[u8; 32]) -> Option<String> {
    encrypted.and_then(|enc| {
        if enc.is_empty() { return None; }
        crate::crypto::decrypt(&enc, key).ok()
    })
}

// ── Comment queries ──

pub fn create_comment(db: &DbPool, id: &str, project_id: &str, author_id: &str, body: &str, moderation_status: &str) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO comments (id, project_id, author_id, body, moderation_status) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, project_id, author_id, body, moderation_status],
    )?;
    Ok(())
}

pub fn list_comments(db: &DbPool, project_id: &str) -> Vec<common::Comment> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.project_id, c.author_id, u.display_name, c.body, c.moderation_status, c.created_at \
             FROM comments c JOIN users u ON c.author_id = u.id \
             WHERE c.project_id = ?1 AND c.moderation_status = 'approved' ORDER BY c.created_at ASC"
        )
        .unwrap();
    stmt.query_map(params![project_id], |row| {
        Ok(common::Comment {
            id: row.get(0)?,
            project_id: row.get(1)?,
            author_id: row.get(2)?,
            author_name: row.get(3)?,
            body: row.get(4)?,
            moderation_status: row.get(5)?,
            created_at: row.get(6)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn list_pending_comments(db: &DbPool) -> Vec<common::Comment> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.project_id, c.author_id, u.display_name, c.body, c.moderation_status, c.created_at \
             FROM comments c JOIN users u ON c.author_id = u.id \
             WHERE c.moderation_status = 'pending_review' ORDER BY c.created_at ASC"
        )
        .unwrap();
    stmt.query_map([], |row| {
        Ok(common::Comment {
            id: row.get(0)?,
            project_id: row.get(1)?,
            author_id: row.get(2)?,
            author_name: row.get(3)?,
            body: row.get(4)?,
            moderation_status: row.get(5)?,
            created_at: row.get(6)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn get_comment_project_id(db: &DbPool, comment_id: &str) -> Option<String> {
    let conn = db.lock();
    conn.query_row(
        "SELECT project_id FROM comments WHERE id = ?1",
        params![comment_id],
        |row| row.get(0),
    )
    .ok()
}

pub fn moderate_comment(db: &DbPool, comment_id: &str, approved: bool) -> SqlResult<usize> {
    let conn = db.lock();
    let status = if approved { "approved" } else { "rejected" };
    conn.execute(
        "UPDATE comments SET moderation_status = ?1 WHERE id = ?2",
        params![status, comment_id],
    )
}

pub fn delete_comment(db: &DbPool, comment_id: &str) -> SqlResult<usize> {
    let conn = db.lock();
    conn.execute("DELETE FROM comments WHERE id = ?1", params![comment_id])
}

// ── Favorite queries ──

pub fn toggle_favorite(db: &DbPool, user_id: &str, project_id: &str) -> SqlResult<bool> {
    let conn = db.lock();
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM favorites WHERE user_id = ?1 AND project_id = ?2",
            params![user_id, project_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if exists {
        conn.execute(
            "DELETE FROM favorites WHERE user_id = ?1 AND project_id = ?2",
            params![user_id, project_id],
        )?;
        Ok(false)
    } else {
        conn.execute(
            "INSERT INTO favorites (user_id, project_id) VALUES (?1, ?2)",
            params![user_id, project_id],
        )?;
        Ok(true)
    }
}

pub fn list_favorites(db: &DbPool, user_id: &str) -> Vec<String> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare("SELECT project_id FROM favorites WHERE user_id = ?1")
        .unwrap();
    stmt.query_map(params![user_id], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}

pub fn list_favorite_projects(db: &DbPool, user_id: &str) -> Vec<common::ProjectSummary> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.title, p.cause, p.zip_code, p.status, p.goal_cents, \
             COALESCE((SELECT SUM(d.amount_cents) FROM donations d WHERE d.project_id = p.id AND (d.is_reversal = 0 OR d.reversal_approved = 1)), 0), \
             COALESCE((SELECT SUM(e.amount_cents) FROM expenses e WHERE e.project_id = p.id AND e.disclosure_status = 'approved'), 0), \
             u.display_name, p.created_at \
             FROM favorites f \
             JOIN projects p ON f.project_id = p.id \
             JOIN users u ON p.manager_id = u.id \
             WHERE f.user_id = ?1 \
             ORDER BY p.created_at DESC"
        )
        .unwrap();
    stmt.query_map(params![user_id], |row| {
        let status_str: String = row.get(4)?;
        Ok(common::ProjectSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            cause: row.get(2)?,
            zip_code: row.get(3)?,
            status: common::ProjectStatus::from_str(&status_str).unwrap_or(common::ProjectStatus::Draft),
            goal_cents: row.get(5)?,
            raised_cents: row.get(6)?,
            spent_cents: row.get(7)?,
            manager_name: row.get(8)?,
            created_at: row.get(9)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

// ── Like queries ──

pub fn toggle_like(db: &DbPool, user_id: &str, update_id: &str) -> SqlResult<bool> {
    let conn = db.lock();
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM update_likes WHERE user_id = ?1 AND update_id = ?2",
            params![user_id, update_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if exists {
        conn.execute(
            "DELETE FROM update_likes WHERE user_id = ?1 AND update_id = ?2",
            params![user_id, update_id],
        )?;
        Ok(false)
    } else {
        conn.execute(
            "INSERT INTO update_likes (user_id, update_id) VALUES (?1, ?2)",
            params![user_id, update_id],
        )?;
        Ok(true)
    }
}

// ── Spending update queries ──

pub fn create_spending_update(db: &DbPool, id: &str, project_id: &str, author_id: &str, title: &str, body: &str) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO spending_updates (id, project_id, author_id, title, body) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, project_id, author_id, title, body],
    )?;
    Ok(())
}

// ── Ticket queries ──

pub fn create_ticket(db: &DbPool, id: &str, project_id: &str, submitter_id: &str, subject: &str, body: &str) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO tickets (id, project_id, submitter_id, subject, body) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, project_id, submitter_id, subject, body],
    )?;
    Ok(())
}

pub fn list_tickets(db: &DbPool, project_id: &str) -> Vec<common::Ticket> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.project_id, t.submitter_id, u.display_name, t.subject, t.body, t.status, t.response, t.created_at \
             FROM tickets t JOIN users u ON t.submitter_id = u.id \
             WHERE t.project_id = ?1 ORDER BY t.created_at DESC"
        )
        .unwrap();
    stmt.query_map(params![project_id], |row| {
        Ok(common::Ticket {
            id: row.get(0)?,
            project_id: row.get(1)?,
            submitter_id: row.get(2)?,
            submitter_name: row.get(3)?,
            subject: row.get(4)?,
            body: row.get(5)?,
            status: row.get(6)?,
            response: row.get(7)?,
            created_at: row.get(8)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn respond_ticket(db: &DbPool, ticket_id: &str, response: &str) -> SqlResult<usize> {
    let conn = db.lock();
    conn.execute(
        "UPDATE tickets SET response = ?1, status = 'resolved' WHERE id = ?2",
        params![response, ticket_id],
    )
}

// ── Notification queries ──

/// Create a notification, respecting the user's DND hours.
/// If the current UTC time is within the user's DND window, the notification is
/// persisted with `is_deferred = 1` instead of being dropped, preserving the audit trail.
pub fn create_notification(db: &DbPool, id: &str, user_id: &str, title: &str, body: &str) -> SqlResult<()> {
    let conn = db.lock();
    // Check DND — compare against the user's local time using their timezone
    let mut deferred = false;
    let dnd: Option<(String, String, String)> = conn.query_row(
        "SELECT dnd_start, dnd_end, timezone FROM users WHERE id = ?1",
        params![user_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).ok();
    if let Some((start_str, end_str, tz_str)) = dnd {
        let now_local = user_local_time_hhmm(&tz_str);
        if is_within_dnd(&now_local, &start_str, &end_str) {
            deferred = true;
        }
    }
    conn.execute(
        "INSERT INTO notifications (id, user_id, title, body, is_deferred) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, user_id, title, body, deferred as i64],
    )?;
    Ok(())
}

/// Convert the current UTC time to the user's local HH:MM using a fixed UTC offset.
/// Accepts IANA-style names "UTC", or explicit offsets like "+05:30", "-08:00".
/// Falls back to UTC on unrecognised input.
fn user_local_time_hhmm(tz: &str) -> String {
    let utc_now = chrono::Utc::now();
    let offset = parse_utc_offset(tz);
    let local = utc_now.with_timezone(&offset);
    local.format("%H:%M").to_string()
}

/// Parse a timezone string into a chrono::FixedOffset.
/// Supports "UTC", "+HH:MM", "-HH:MM" formats.  Unknown strings fall back to UTC.
fn parse_utc_offset(tz: &str) -> chrono::FixedOffset {
    let tz = tz.trim();
    if tz.eq_ignore_ascii_case("UTC") || tz.eq_ignore_ascii_case("GMT") || tz.is_empty() {
        return chrono::FixedOffset::east_opt(0).unwrap();
    }
    // Try to parse "+HH:MM" / "-HH:MM"
    if let Some(rest) = tz.strip_prefix('+').or_else(|| tz.strip_prefix('-')) {
        let sign: i32 = if tz.starts_with('-') { -1 } else { 1 };
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() == 2 {
            if let (Ok(h), Ok(m)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                let total_secs = sign * (h * 3600 + m * 60);
                if let Some(offset) = chrono::FixedOffset::east_opt(total_secs) {
                    return offset;
                }
            }
        }
    }
    // Fallback to UTC for unrecognised timezone strings
    chrono::FixedOffset::east_opt(0).unwrap()
}

/// Check if `now` (HH:MM) falls within a DND window.
/// Handles overnight windows like 21:00-07:00.
fn is_within_dnd(now: &str, start: &str, end: &str) -> bool {
    if start <= end {
        // Same-day window, e.g. 09:00-17:00
        now >= start && now < end
    } else {
        // Overnight window, e.g. 21:00-07:00
        now >= start || now < end
    }
}

pub fn list_notifications(db: &DbPool, user_id: &str) -> Vec<common::Notification> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, user_id, title, body, is_read, is_deferred, created_at FROM notifications WHERE user_id = ?1 ORDER BY created_at DESC LIMIT 100"
        )
        .unwrap();
    stmt.query_map(params![user_id], |row| {
        Ok(common::Notification {
            id: row.get(0)?,
            user_id: row.get(1)?,
            title: row.get(2)?,
            body: row.get(3)?,
            is_read: row.get::<_, i64>(4)? != 0,
            is_deferred: row.get::<_, i64>(5)? != 0,
            created_at: row.get(6)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn mark_notification_read(db: &DbPool, notification_id: &str, user_id: &str) -> SqlResult<usize> {
    let conn = db.lock();
    conn.execute(
        "UPDATE notifications SET is_read = 1 WHERE id = ?1 AND user_id = ?2",
        params![notification_id, user_id],
    )
}

pub fn mark_all_notifications_read(db: &DbPool, user_id: &str) -> SqlResult<usize> {
    let conn = db.lock();
    conn.execute(
        "UPDATE notifications SET is_read = 1 WHERE user_id = ?1",
        params![user_id],
    )
}

// ── Subscription queries ──

pub fn set_subscription(db: &DbPool, user_id: &str, project_id: &str, enabled: bool) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO subscriptions (user_id, project_id, enabled) VALUES (?1, ?2, ?3) \
         ON CONFLICT(user_id, project_id) DO UPDATE SET enabled = ?3",
        params![user_id, project_id, enabled as i64],
    )?;
    Ok(())
}

pub fn get_project_subscribers(db: &DbPool, project_id: &str) -> Vec<String> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare("SELECT user_id FROM subscriptions WHERE project_id = ?1 AND enabled = 1")
        .unwrap();
    stmt.query_map(params![project_id], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}

// ── Dashboard stats ──

pub fn get_dashboard_stats(
    db: &DbPool,
    from: Option<&str>,
    to: Option<&str>,
    cause: Option<&str>,
    status: Option<&str>,
) -> common::DashboardStats {
    let conn = db.lock();

    // Build parameterized filters
    let mut clauses = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    // Accounting filter
    clauses.push("(d.is_reversal = 0 OR d.reversal_approved = 1)".to_string());

    if let Some(f) = from {
        params.push(Box::new(f.to_string()));
        clauses.push(format!("d.created_at >= ?{}", params.len()));
    }
    if let Some(t) = to {
        params.push(Box::new(t.to_string()));
        clauses.push(format!("d.created_at <= ?{}", params.len()));
    }

    // Multidimensional project filters via JOIN
    let mut project_join = String::new();
    if cause.is_some() || status.is_some() {
        project_join = " JOIN projects p ON d.project_id = p.id".to_string();
        if let Some(c) = cause {
            params.push(Box::new(c.to_string()));
            clauses.push(format!("p.cause = ?{}", params.len()));
        }
        if let Some(s) = status {
            params.push(Box::new(s.to_string()));
            clauses.push(format!("p.status = ?{}", params.len()));
        }
    }

    let where_sql = clauses.join(" AND ");
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|b| b.as_ref() as &dyn rusqlite::types::ToSql).collect();

    let gmv_sql = format!(
        "SELECT COALESCE(SUM(d.amount_cents), 0) FROM donations d{} WHERE {}",
        project_join, where_sql
    );
    let gmv: i64 = {
        let mut stmt = conn.prepare(&gmv_sql).unwrap();
        stmt.query_row(param_refs.as_slice(), |row| row.get(0)).unwrap_or(0)
    };

    // For counts, exclude reversals entirely
    let count_where = where_sql.replace(
        "(d.is_reversal = 0 OR d.reversal_approved = 1)",
        "d.is_reversal = 0",
    );

    let total_sql = format!(
        "SELECT COUNT(*) FROM donations d{} WHERE {}", project_join, count_where
    );
    let total_donations: i64 = {
        let mut stmt = conn.prepare(&total_sql).unwrap();
        stmt.query_row(param_refs.as_slice(), |row| row.get(0)).unwrap_or(0)
    };

    let unique_sql = format!(
        "SELECT COUNT(DISTINCT d.donor_id) FROM donations d{} WHERE {}", project_join, count_where
    );
    let unique_donors: i64 = {
        let mut stmt = conn.prepare(&unique_sql).unwrap();
        stmt.query_row(param_refs.as_slice(), |row| row.get(0)).unwrap_or(0)
    };

    let average_donation_cents = if total_donations > 0 { gmv / total_donations } else { 0 };

    let repeat_sql = format!(
        "SELECT COUNT(*) FROM (SELECT d.donor_id FROM donations d{} \
         WHERE {} GROUP BY d.donor_id HAVING COUNT(*) > 1)",
        project_join, count_where
    );
    let repeat_donors: i64 = {
        let mut stmt = conn.prepare(&repeat_sql).unwrap();
        stmt.query_row(param_refs.as_slice(), |row| row.get(0)).unwrap_or(0)
    };

    let repeat_donor_rate = if unique_donors > 0 { repeat_donors as f64 / unique_donors as f64 } else { 0.0 };
    let total_users: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0)).unwrap_or(1);
    let conversion_rate = if total_users > 0 { unique_donors as f64 / total_users as f64 } else { 0.0 };

    common::DashboardStats {
        gmv_cents: gmv,
        total_donations,
        unique_donors,
        average_donation_cents,
        repeat_donor_rate,
        conversion_rate,
    }
}

/// Build a parameterized date clause with ?-placeholders for from/to.
/// Returns (clause_string, params_vec). Placeholder numbering starts at 1.
fn build_date_params(from: Option<&str>, to: Option<&str>) -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
    let mut clauses = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(f) = from {
        params.push(Box::new(f.to_string()));
        clauses.push(format!("AND d.created_at >= ?{}", params.len()));
    }
    if let Some(t) = to {
        params.push(Box::new(t.to_string()));
        clauses.push(format!("AND d.created_at <= ?{}", params.len()));
    }
    (clauses.join(" "), params)
}

// ── CSV Export helpers ──

pub fn export_donations_csv(
    db: &DbPool,
    from: Option<&str>,
    to: Option<&str>,
    cause: Option<&str>,
    status: Option<&str>,
) -> String {
    let conn = db.lock();

    // Accounting filter: only include finalized records (non-reversals + approved reversals)
    let mut clauses = vec!["(d.is_reversal = 0 OR d.reversal_approved = 1)".to_string()];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(f) = from {
        params.push(Box::new(f.to_string()));
        clauses.push(format!("d.created_at >= ?{}", params.len()));
    }
    if let Some(t) = to {
        params.push(Box::new(t.to_string()));
        clauses.push(format!("d.created_at <= ?{}", params.len()));
    }
    if let Some(c) = cause {
        params.push(Box::new(c.to_string()));
        clauses.push(format!("p.cause = ?{}", params.len()));
    }
    if let Some(s) = status {
        params.push(Box::new(s.to_string()));
        clauses.push(format!("p.status = ?{}", params.len()));
    }
    let where_sql = clauses.join(" AND ");

    let query = format!(
        "SELECT d.pledge_number, p.title, \
         SUBSTR(u.display_name, 1, 1) || '***' AS masked_name, \
         d.amount_cents, bl.name, d.created_at \
         FROM donations d \
         JOIN projects p ON d.project_id = p.id \
         JOIN users u ON d.donor_id = u.id \
         LEFT JOIN budget_lines bl ON d.budget_line_id = bl.id \
         WHERE {} ORDER BY d.created_at DESC",
        where_sql
    );

    let mut stmt = conn.prepare(&query).unwrap();
    let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref() as &dyn rusqlite::types::ToSql).collect();
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(["Pledge#", "Project", "Donor (masked)", "Amount (cents)", "Budget Line", "Date"])
        .unwrap();
    let rows = stmt
        .query_map(refs.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .unwrap();
    for r in rows.flatten() {
        wtr.write_record(&[r.0, r.1, r.2, r.3.to_string(), r.4.unwrap_or_default(), r.5]).unwrap();
    }
    String::from_utf8(wtr.into_inner().unwrap()).unwrap_or_default()
}

// ── Receipt queries ──

pub fn create_receipt(
    db: &DbPool,
    id: &str,
    expense_id: &str,
    file_name: &str,
    file_type: &str,
    file_size: i64,
    file_data: &[u8],
    sha256_fingerprint: &str,
) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO receipts (id, expense_id, file_name, file_type, file_size, file_data, sha256_fingerprint) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, expense_id, file_name, file_type, file_size, file_data, sha256_fingerprint],
    )?;
    Ok(())
}

pub fn receipt_fingerprint_exists(db: &DbPool, fingerprint: &str) -> bool {
    let conn = db.lock();
    conn.query_row(
        "SELECT COUNT(*) FROM receipts WHERE sha256_fingerprint = ?1",
        params![fingerprint],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

pub fn review_receipt(db: &DbPool, receipt_id: &str, verified: bool, reviewer_id: &str, rejection_reason: Option<&str>) -> SqlResult<usize> {
    let conn = db.lock();
    let status = if verified { "verified" } else { "rejected" };
    conn.execute(
        "UPDATE receipts SET status = ?1, reviewer_id = ?2, rejection_reason = ?3 WHERE id = ?4",
        params![status, reviewer_id, rejection_reason, receipt_id],
    )
}

pub fn list_receipts_for_expense(db: &DbPool, expense_id: &str) -> Vec<common::ReceiptRecord> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, expense_id, file_name, file_type, file_size, sha256_fingerprint, status, rejection_reason, reviewer_id, created_at \
             FROM receipts WHERE expense_id = ?1 ORDER BY created_at DESC"
        )
        .unwrap();
    stmt.query_map(params![expense_id], |row| {
        let status_str: String = row.get(6)?;
        Ok(common::ReceiptRecord {
            id: row.get(0)?,
            expense_id: row.get(1)?,
            file_name: row.get(2)?,
            file_type: row.get(3)?,
            file_size: row.get(4)?,
            sha256_fingerprint: row.get(5)?,
            status: common::ReceiptStatus::from_str(&status_str).unwrap_or(common::ReceiptStatus::Uploaded),
            rejection_reason: row.get(7)?,
            reviewer_id: row.get(8)?,
            created_at: row.get(9)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn list_uploaded_receipts(db: &DbPool) -> Vec<common::ReceiptRecord> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, expense_id, file_name, file_type, file_size, sha256_fingerprint, status, rejection_reason, reviewer_id, created_at \
             FROM receipts WHERE status = 'uploaded' ORDER BY created_at ASC"
        )
        .unwrap();
    stmt.query_map([], |row| {
        let status_str: String = row.get(6)?;
        Ok(common::ReceiptRecord {
            id: row.get(0)?,
            expense_id: row.get(1)?,
            file_name: row.get(2)?,
            file_type: row.get(3)?,
            file_size: row.get(4)?,
            sha256_fingerprint: row.get(5)?,
            status: common::ReceiptStatus::from_str(&status_str).unwrap_or(common::ReceiptStatus::Uploaded),
            rejection_reason: row.get(7)?,
            reviewer_id: row.get(8)?,
            created_at: row.get(9)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

// ── Moderation config queries ──

pub fn get_moderation_config(db: &DbPool) -> common::ModerationConfig {
    let conn = db.lock();
    conn.query_row(
        "SELECT comments_enabled, require_pre_moderation, sensitive_words FROM moderation_config WHERE id = 1",
        [],
        |row| {
            let words_str: String = row.get(2)?;
            let words: Vec<String> = if words_str.is_empty() {
                Vec::new()
            } else {
                words_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
            };
            Ok(common::ModerationConfig {
                comments_enabled: row.get::<_, i64>(0)? != 0,
                require_pre_moderation: row.get::<_, i64>(1)? != 0,
                sensitive_words: words,
            })
        },
    )
    .unwrap_or(common::ModerationConfig {
        comments_enabled: true,
        require_pre_moderation: false,
        sensitive_words: Vec::new(),
    })
}

pub fn update_moderation_config(db: &DbPool, config: &common::ModerationConfig) -> SqlResult<()> {
    let conn = db.lock();
    let words_str = config.sensitive_words.join(",");
    conn.execute(
        "UPDATE moderation_config SET comments_enabled = ?1, require_pre_moderation = ?2, sensitive_words = ?3 WHERE id = 1",
        params![config.comments_enabled as i64, config.require_pre_moderation as i64, words_str],
    )?;
    Ok(())
}

// ── Fulfillment queries ──

pub fn create_fulfillment(db: &DbPool, id: &str, project_id: &str) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO fulfillments (id, project_id) VALUES (?1, ?2)",
        params![id, project_id],
    )?;
    Ok(())
}

pub fn get_fulfillment(db: &DbPool, id: &str) -> Option<common::FulfillmentRecord> {
    let conn = db.lock();
    conn.query_row(
        "SELECT id, project_id, arrival_at, start_at, end_at, is_complete, service_record_hash, created_at \
         FROM fulfillments WHERE id = ?1",
        params![id],
        |row| {
            Ok(common::FulfillmentRecord {
                id: row.get(0)?,
                project_id: row.get(1)?,
                arrival_at: row.get(2)?,
                start_at: row.get(3)?,
                end_at: row.get(4)?,
                arrival_code: None,
                start_code: None,
                end_code: None,
                is_complete: row.get::<_, i64>(5)? != 0,
                service_record_hash: row.get(6)?,
                created_at: row.get(7)?,
            })
        },
    )
    .ok()
}

pub fn list_fulfillments(db: &DbPool, project_id: &str) -> Vec<common::FulfillmentRecord> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, arrival_at, start_at, end_at, is_complete, service_record_hash, created_at \
             FROM fulfillments WHERE project_id = ?1 ORDER BY created_at DESC"
        )
        .unwrap();
    stmt.query_map(params![project_id], |row| {
        Ok(common::FulfillmentRecord {
            id: row.get(0)?,
            project_id: row.get(1)?,
            arrival_at: row.get(2)?,
            start_at: row.get(3)?,
            end_at: row.get(4)?,
            arrival_code: None,
            start_code: None,
            end_code: None,
            is_complete: row.get::<_, i64>(5)? != 0,
            service_record_hash: row.get(6)?,
            created_at: row.get(7)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn store_checkpoint_code(db: &DbPool, id: &str, fulfillment_id: &str, checkpoint: &str, code: &str, expires_at: &str) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO checkpoint_codes (id, fulfillment_id, checkpoint, code, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, fulfillment_id, checkpoint, code, expires_at],
    )?;
    Ok(())
}

pub fn consume_checkpoint_code(db: &DbPool, fulfillment_id: &str, checkpoint: &str, code: &str) -> SqlResult<bool> {
    let conn = db.lock();
    let deleted = conn.execute(
        "DELETE FROM checkpoint_codes WHERE fulfillment_id = ?1 AND checkpoint = ?2 AND code = ?3 AND consumed = 0 AND expires_at >= datetime('now')",
        params![fulfillment_id, checkpoint, code],
    )?;
    Ok(deleted > 0)
}

pub fn record_checkpoint(db: &DbPool, fulfillment_id: &str, checkpoint: &str, timestamp: &str) -> SqlResult<()> {
    let conn = db.lock();
    let col = match checkpoint {
        "arrival" => "arrival_at",
        "start" => "start_at",
        "end" => "end_at",
        _ => return Err(rusqlite::Error::InvalidParameterName("invalid checkpoint".into())),
    };
    conn.execute(
        &format!("UPDATE fulfillments SET {} = ?1 WHERE id = ?2", col),
        params![timestamp, fulfillment_id],
    )?;
    Ok(())
}

pub fn complete_fulfillment(db: &DbPool, fulfillment_id: &str, hash: &str) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "UPDATE fulfillments SET is_complete = 1, service_record_hash = ?1 WHERE id = ?2",
        params![hash, fulfillment_id],
    )?;
    Ok(())
}

// ── Analytics event queries ──

pub fn insert_event(
    db: &DbPool,
    id: &str,
    event_kind: &str,
    target_type: &str,
    target_id: &str,
    session_id: &str,
    user_id: Option<&str>,
    dwell_ms: Option<i64>,
    is_duplicate: bool,
    is_suspicious: bool,
) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO analytics_events (id, event_kind, target_type, target_id, session_id, user_id, dwell_ms, is_duplicate, is_suspicious) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![id, event_kind, target_type, target_id, session_id, user_id, dwell_ms, is_duplicate as i64, is_suspicious as i64],
    )?;
    Ok(())
}

/// Check if a duplicate event exists within 3 seconds.
pub fn is_duplicate_event(db: &DbPool, event_kind: &str, target_id: &str, session_id: &str) -> bool {
    let conn = db.lock();
    conn.query_row(
        "SELECT COUNT(*) FROM analytics_events \
         WHERE event_kind = ?1 AND target_id = ?2 AND session_id = ?3 \
         AND created_at >= datetime('now', '-3 seconds')",
        params![event_kind, target_id, session_id],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

/// Check if there's a suspicious burst (>20 events in 10 seconds from same session).
pub fn is_suspicious_burst(db: &DbPool, session_id: &str) -> bool {
    let conn = db.lock();
    conn.query_row(
        "SELECT COUNT(*) FROM analytics_events \
         WHERE session_id = ?1 AND created_at >= datetime('now', '-10 seconds')",
        params![session_id],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 20
}

pub fn get_data_quality_metrics(db: &DbPool) -> common::DataQualityMetrics {
    let conn = db.lock();
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM analytics_events", [], |row| row.get(0)).unwrap_or(0);
    let duplicates: i64 = conn.query_row("SELECT COUNT(*) FROM analytics_events WHERE is_duplicate = 1", [], |row| row.get(0)).unwrap_or(0);
    let suspicious: i64 = conn.query_row("SELECT COUNT(*) FROM analytics_events WHERE is_suspicious = 1", [], |row| row.get(0)).unwrap_or(0);

    let mut stmt = conn.prepare("SELECT event_kind, COUNT(*) FROM analytics_events GROUP BY event_kind ORDER BY COUNT(*) DESC").unwrap();
    let events_by_kind: Vec<(String, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    common::DataQualityMetrics {
        total_events: total,
        duplicate_events: duplicates,
        suspicious_events: suspicious,
        duplicate_rate: if total > 0 { duplicates as f64 / total as f64 } else { 0.0 },
        suspicious_rate: if total > 0 { suspicious as f64 / total as f64 } else { 0.0 },
        events_by_kind,
    }
}

pub fn list_suspicious_events(db: &DbPool, limit: i64) -> Vec<common::AnalyticsEvent> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, event_kind, target_type, target_id, session_id, user_id, dwell_ms, is_duplicate, is_suspicious, created_at \
             FROM analytics_events WHERE is_suspicious = 1 ORDER BY created_at DESC LIMIT ?1"
        )
        .unwrap();
    stmt.query_map(params![limit], |row| {
        Ok(common::AnalyticsEvent {
            id: row.get(0)?,
            event_kind: row.get(1)?,
            target_type: row.get(2)?,
            target_id: row.get(3)?,
            session_id: row.get(4)?,
            user_id: row.get(5)?,
            dwell_ms: row.get(6)?,
            is_duplicate: row.get::<_, i64>(7)? != 0,
            is_suspicious: row.get::<_, i64>(8)? != 0,
            created_at: row.get(9)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

// ── Webhook queries ──

pub fn create_webhook(db: &DbPool, id: &str, name: &str, url: &str, secret: &str, event_types: &str) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO webhooks (id, name, url, secret, event_types) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, name, url, secret, event_types],
    )?;
    Ok(())
}

pub fn list_webhooks(db: &DbPool) -> Vec<common::WebhookConfig> {
    let conn = db.lock();
    let mut stmt = conn.prepare("SELECT id, name, url, secret, event_types, enabled, created_at FROM webhooks ORDER BY created_at DESC").unwrap();
    stmt.query_map([], |row| {
        let types_json: String = row.get(4)?;
        let event_types: Vec<String> = serde_json::from_str(&types_json).unwrap_or_default();
        Ok(common::WebhookConfig {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            secret: row.get(3)?,
            event_types,
            enabled: row.get::<_, i64>(5)? != 0,
            created_at: row.get(6)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn get_webhooks_for_event(db: &DbPool, event_type: &str) -> Vec<common::WebhookConfig> {
    list_webhooks(db)
        .into_iter()
        .filter(|w| w.enabled && w.event_types.iter().any(|t| t == event_type || t == "*"))
        .collect()
}

pub fn delete_webhook(db: &DbPool, webhook_id: &str) -> SqlResult<usize> {
    let conn = db.lock();
    conn.execute("DELETE FROM webhooks WHERE id = ?1", params![webhook_id])
}

pub fn log_webhook_delivery(
    db: &DbPool,
    id: &str,
    webhook_id: &str,
    event_type: &str,
    payload_summary: &str,
    attempt: i32,
    status_code: Option<i32>,
    success: bool,
    error_message: Option<&str>,
) -> SqlResult<()> {
    let conn = db.lock();
    conn.execute(
        "INSERT INTO webhook_delivery_log (id, webhook_id, event_type, payload_summary, attempt, status_code, success, error_message) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, webhook_id, event_type, payload_summary, attempt, status_code, success as i64, error_message],
    )?;
    Ok(())
}

pub fn list_webhook_deliveries(db: &DbPool, webhook_id: &str, limit: i64) -> Vec<common::WebhookDeliveryLog> {
    let conn = db.lock();
    let mut stmt = conn
        .prepare(
            "SELECT id, webhook_id, event_type, payload_summary, attempt, status_code, success, error_message, created_at \
             FROM webhook_delivery_log WHERE webhook_id = ?1 ORDER BY created_at DESC LIMIT ?2"
        )
        .unwrap();
    stmt.query_map(params![webhook_id, limit], |row| {
        Ok(common::WebhookDeliveryLog {
            id: row.get(0)?,
            webhook_id: row.get(1)?,
            event_type: row.get(2)?,
            payload_summary: row.get(3)?,
            attempt: row.get(4)?,
            status_code: row.get(5)?,
            success: row.get::<_, i64>(6)? != 0,
            error_message: row.get(7)?,
            created_at: row.get(8)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}
