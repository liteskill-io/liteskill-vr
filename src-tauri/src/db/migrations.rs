use rusqlite::Connection;

use super::error::Result;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS project (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    color TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS connection_types (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS items (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    item_type TEXT NOT NULL,
    path TEXT,
    architecture TEXT,
    description TEXT NOT NULL DEFAULT '',
    analysis_status TEXT NOT NULL DEFAULT 'untouched',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS item_tags (
    item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    tag_name TEXT NOT NULL REFERENCES tags(name) ON DELETE CASCADE,
    PRIMARY KEY (item_id, tag_name)
);

CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    item_id TEXT REFERENCES items(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    author TEXT NOT NULL,
    author_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS note_tags (
    note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    tag_name TEXT NOT NULL REFERENCES tags(name) ON DELETE CASCADE,
    PRIMARY KEY (note_id, tag_name)
);

CREATE TABLE IF NOT EXISTS items_of_interest (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'draft',
    location TEXT,
    severity TEXT,
    author TEXT NOT NULL,
    author_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ioi_tags (
    ioi_id TEXT NOT NULL REFERENCES items_of_interest(id) ON DELETE CASCADE,
    tag_name TEXT NOT NULL REFERENCES tags(name) ON DELETE CASCADE,
    PRIMARY KEY (ioi_id, tag_name)
);

CREATE TABLE IF NOT EXISTS connections (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    source_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    target_type TEXT NOT NULL,
    connection_type TEXT NOT NULL REFERENCES connection_types(name) ON DELETE CASCADE,
    description TEXT NOT NULL DEFAULT '',
    author TEXT NOT NULL,
    author_type TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Cascade deletes for polymorphic connection references
CREATE TRIGGER IF NOT EXISTS items_delete_connections AFTER DELETE ON items BEGIN
    DELETE FROM connections WHERE
        (source_id = old.id AND source_type = 'item') OR
        (target_id = old.id AND target_type = 'item');
END;

CREATE TRIGGER IF NOT EXISTS ioi_delete_connections AFTER DELETE ON items_of_interest BEGIN
    DELETE FROM connections WHERE
        (source_id = old.id AND source_type = 'item_of_interest') OR
        (target_id = old.id AND target_type = 'item_of_interest');
END;

-- FTS5 virtual tables for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS fts_items USING fts5(
    id UNINDEXED, name, description, content=items, content_rowid=rowid
);

CREATE VIRTUAL TABLE IF NOT EXISTS fts_notes USING fts5(
    id UNINDEXED, item_id UNINDEXED, title, content, content=notes, content_rowid=rowid
);

CREATE VIRTUAL TABLE IF NOT EXISTS fts_ioi USING fts5(
    id UNINDEXED, item_id UNINDEXED, title, description, location,
    content=items_of_interest, content_rowid=rowid
);

CREATE VIRTUAL TABLE IF NOT EXISTS fts_connections USING fts5(
    id UNINDEXED, description, content=connections, content_rowid=rowid
);

-- Triggers to keep FTS tables in sync
CREATE TRIGGER IF NOT EXISTS items_ai AFTER INSERT ON items BEGIN
    INSERT INTO fts_items(rowid, id, name, description) VALUES (new.rowid, new.id, new.name, new.description);
END;
CREATE TRIGGER IF NOT EXISTS items_ad AFTER DELETE ON items BEGIN
    INSERT INTO fts_items(fts_items, rowid, id, name, description) VALUES ('delete', old.rowid, old.id, old.name, old.description);
END;
CREATE TRIGGER IF NOT EXISTS items_au AFTER UPDATE ON items BEGIN
    INSERT INTO fts_items(fts_items, rowid, id, name, description) VALUES ('delete', old.rowid, old.id, old.name, old.description);
    INSERT INTO fts_items(rowid, id, name, description) VALUES (new.rowid, new.id, new.name, new.description);
END;

CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
    INSERT INTO fts_notes(rowid, id, item_id, title, content) VALUES (new.rowid, new.id, new.item_id, new.title, new.content);
END;
CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
    INSERT INTO fts_notes(fts_notes, rowid, id, item_id, title, content) VALUES ('delete', old.rowid, old.id, old.item_id, old.title, old.content);
END;
CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
    INSERT INTO fts_notes(fts_notes, rowid, id, item_id, title, content) VALUES ('delete', old.rowid, old.id, old.item_id, old.title, old.content);
    INSERT INTO fts_notes(rowid, id, item_id, title, content) VALUES (new.rowid, new.id, new.item_id, new.title, new.content);
END;

CREATE TRIGGER IF NOT EXISTS ioi_ai AFTER INSERT ON items_of_interest BEGIN
    INSERT INTO fts_ioi(rowid, id, item_id, title, description, location) VALUES (new.rowid, new.id, new.item_id, new.title, new.description, new.location);
END;
CREATE TRIGGER IF NOT EXISTS ioi_ad AFTER DELETE ON items_of_interest BEGIN
    INSERT INTO fts_ioi(fts_ioi, rowid, id, item_id, title, description, location) VALUES ('delete', old.rowid, old.id, old.item_id, old.title, old.description, old.location);
END;
CREATE TRIGGER IF NOT EXISTS ioi_au AFTER UPDATE ON items_of_interest BEGIN
    INSERT INTO fts_ioi(fts_ioi, rowid, id, item_id, title, description, location) VALUES ('delete', old.rowid, old.id, old.item_id, old.title, old.description, old.location);
    INSERT INTO fts_ioi(rowid, id, item_id, title, description, location) VALUES (new.rowid, new.id, new.item_id, new.title, new.description, new.location);
END;

CREATE TRIGGER IF NOT EXISTS conn_ai AFTER INSERT ON connections BEGIN
    INSERT INTO fts_connections(rowid, id, description) VALUES (new.rowid, new.id, new.description);
END;
CREATE TRIGGER IF NOT EXISTS conn_ad AFTER DELETE ON connections BEGIN
    INSERT INTO fts_connections(fts_connections, rowid, id, description) VALUES ('delete', old.rowid, old.id, old.description);
END;
CREATE TRIGGER IF NOT EXISTS conn_au AFTER UPDATE ON connections BEGIN
    INSERT INTO fts_connections(fts_connections, rowid, id, description) VALUES ('delete', old.rowid, old.id, old.description);
    INSERT INTO fts_connections(rowid, id, description) VALUES (new.rowid, new.id, new.description);
END;

-- ===================================================================
-- Explanation layer: evidence-backed 'how this works' models.
-- A model envelope (explanations) + claims + open_questions, with
-- fine-grained evidence_links. Coarse links (scope, finding-context)
-- reuse the connections table via 'explanation' source/target types.
-- ===================================================================

CREATE TABLE IF NOT EXISTS explanations (
    id TEXT PRIMARY KEY,
    stable_key TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    explanation_type TEXT NOT NULL DEFAULT 'custom',
    summary TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'draft',
    confidence TEXT NOT NULL DEFAULT 'medium',
    author TEXT NOT NULL,
    author_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS explanation_tags (
    explanation_id TEXT NOT NULL REFERENCES explanations(id) ON DELETE CASCADE,
    tag_name TEXT NOT NULL REFERENCES tags(name) ON DELETE CASCADE,
    PRIMARY KEY (explanation_id, tag_name)
);

CREATE TABLE IF NOT EXISTS claims (
    id TEXT PRIMARY KEY,
    explanation_id TEXT NOT NULL REFERENCES explanations(id) ON DELETE CASCADE,
    stable_key TEXT NOT NULL,
    text TEXT NOT NULL,
    claim_type TEXT NOT NULL DEFAULT 'behavior',
    status TEXT NOT NULL DEFAULT 'hypothesis',
    confidence TEXT NOT NULL DEFAULT 'medium',
    author TEXT NOT NULL,
    author_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (explanation_id, stable_key)
);

CREATE TABLE IF NOT EXISTS open_questions (
    id TEXT PRIMARY KEY,
    explanation_id TEXT NOT NULL REFERENCES explanations(id) ON DELETE CASCADE,
    stable_key TEXT NOT NULL,
    question TEXT NOT NULL,
    priority TEXT NOT NULL DEFAULT 'medium',
    status TEXT NOT NULL DEFAULT 'open',
    answer_claim_id TEXT,
    author TEXT NOT NULL,
    author_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (explanation_id, stable_key)
);

-- Fine-grained evidence: target is an explanation/claim/finding; source is
-- EITHER an existing entity OR a free-text external locator (Ghidra symbol,
-- address, pcap packet, log line).
CREATE TABLE IF NOT EXISTS evidence_links (
    id TEXT PRIMARY KEY,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    source_entity_type TEXT,
    source_entity_id TEXT,
    external_locator TEXT,
    external_kind TEXT,
    evidence_type TEXT NOT NULL DEFAULT 'agent_inference',
    strength TEXT NOT NULL DEFAULT 'moderate',
    excerpt TEXT,
    author TEXT NOT NULL,
    author_type TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_claims_explanation ON claims(explanation_id);
CREATE INDEX IF NOT EXISTS idx_questions_explanation ON open_questions(explanation_id);
CREATE INDEX IF NOT EXISTS idx_evidence_target ON evidence_links(target_type, target_id);

-- Cascade cleanup of polymorphic references on delete.
CREATE TRIGGER IF NOT EXISTS explanations_delete_connections AFTER DELETE ON explanations BEGIN
    DELETE FROM connections WHERE
        (source_id = old.id AND source_type = 'explanation') OR
        (target_id = old.id AND target_type = 'explanation');
END;
CREATE TRIGGER IF NOT EXISTS explanations_delete_evidence AFTER DELETE ON explanations BEGIN
    DELETE FROM evidence_links WHERE target_type = 'explanation' AND target_id = old.id;
END;
CREATE TRIGGER IF NOT EXISTS claims_delete_evidence AFTER DELETE ON claims BEGIN
    DELETE FROM evidence_links WHERE target_type = 'claim' AND target_id = old.id;
END;
CREATE TRIGGER IF NOT EXISTS ioi_delete_evidence AFTER DELETE ON items_of_interest BEGIN
    DELETE FROM evidence_links WHERE target_type = 'finding' AND target_id = old.id;
END;

-- FTS for explanations + claims (kept fresh by triggers; search wiring is
-- deferred but the index stays current so it's there when needed).
CREATE VIRTUAL TABLE IF NOT EXISTS fts_explanations USING fts5(
    id UNINDEXED, title, summary, content=explanations, content_rowid=rowid
);
CREATE VIRTUAL TABLE IF NOT EXISTS fts_claims USING fts5(
    id UNINDEXED, explanation_id UNINDEXED, text, content=claims, content_rowid=rowid
);

CREATE TRIGGER IF NOT EXISTS expl_ai AFTER INSERT ON explanations BEGIN
    INSERT INTO fts_explanations(rowid, id, title, summary) VALUES (new.rowid, new.id, new.title, new.summary);
END;
CREATE TRIGGER IF NOT EXISTS expl_ad AFTER DELETE ON explanations BEGIN
    INSERT INTO fts_explanations(fts_explanations, rowid, id, title, summary) VALUES ('delete', old.rowid, old.id, old.title, old.summary);
END;
CREATE TRIGGER IF NOT EXISTS expl_au AFTER UPDATE ON explanations BEGIN
    INSERT INTO fts_explanations(fts_explanations, rowid, id, title, summary) VALUES ('delete', old.rowid, old.id, old.title, old.summary);
    INSERT INTO fts_explanations(rowid, id, title, summary) VALUES (new.rowid, new.id, new.title, new.summary);
END;

CREATE TRIGGER IF NOT EXISTS claim_ai AFTER INSERT ON claims BEGIN
    INSERT INTO fts_claims(rowid, id, explanation_id, text) VALUES (new.rowid, new.id, new.explanation_id, new.text);
END;
CREATE TRIGGER IF NOT EXISTS claim_ad AFTER DELETE ON claims BEGIN
    INSERT INTO fts_claims(fts_claims, rowid, id, explanation_id, text) VALUES ('delete', old.rowid, old.id, old.explanation_id, old.text);
END;
CREATE TRIGGER IF NOT EXISTS claim_au AFTER UPDATE ON claims BEGIN
    INSERT INTO fts_claims(fts_claims, rowid, id, explanation_id, text) VALUES ('delete', old.rowid, old.id, old.explanation_id, old.text);
    INSERT INTO fts_claims(rowid, id, explanation_id, text) VALUES (new.rowid, new.id, new.explanation_id, new.text);
END;
";

const DEFAULT_TAGS: &[(&str, &str)] = &[
    (
        "memory-corruption",
        "Buffer overflows, heap issues, use-after-free",
    ),
    ("auth-bypass", "Authentication or authorization flaws"),
    ("command-injection", "OS command injection"),
    ("hardcoded-creds", "Hardcoded passwords, keys, tokens"),
    ("info-disclosure", "Information leakage"),
    ("logic-issue", "Business logic or control flow flaws"),
    ("crypto-weakness", "Weak or misused cryptography"),
    ("race-condition", "TOCTOU and concurrency bugs"),
    ("format-string", "Format string vulnerabilities"),
    ("integer-issue", "Integer overflow, underflow, truncation"),
    ("insecure-config", "Dangerous default or misconfiguration"),
    ("debug-interface", "Debug ports, test endpoints, JTAG"),
    (
        "interesting",
        "Worth investigating further (not yet classified)",
    ),
];

const DEFAULT_CONNECTION_TYPES: &[(&str, &str)] = &[
    (
        "calls",
        "Source function/binary calls target function/binary",
    ),
    ("imports", "Source imports a symbol from target"),
    ("links", "Source dynamically links target shared object"),
    ("reads_config", "Source reads target config file at runtime"),
    ("writes_config", "Source writes/modifies target config file"),
    ("spawns", "Source starts target as a process/daemon"),
    ("related", "Loose association worth tracking"),
    (
        "explains",
        "Source explanation explains/covers the target item",
    ),
    (
        "affects",
        "Source finding affects or relates to the target explanation",
    ),
];

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_add_ioi_status",
        "ALTER TABLE items_of_interest ADD COLUMN status TEXT NOT NULL DEFAULT 'draft';",
    ),
    // Seed the explanation-layer connection types into pre-existing databases
    // (new databases get them via DEFAULT_CONNECTION_TYPES in seed_defaults).
    (
        "002_seed_explanation_connection_types",
        "INSERT OR IGNORE INTO connection_types (id, name, description, created_at) VALUES
            (lower(hex(randomblob(16))), 'explains', 'Source explanation explains/covers the target item', strftime('%Y-%m-%dT%H:%M:%SZ','now')),
            (lower(hex(randomblob(16))), 'affects', 'Source finding affects or relates to the target explanation', strftime('%Y-%m-%dT%H:%M:%SZ','now'));",
    ),
];

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(SCHEMA)?;

    // Versioned migrations for schema changes on existing databases
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;

    for (name, sql) in MIGRATIONS {
        let already_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = ?1)",
            rusqlite::params![name],
            |row| row.get(0),
        )?;
        if !already_applied {
            // Some migrations may fail on new databases where the schema
            // already includes the change (e.g., column already exists).
            // That's fine — we record it as applied either way.
            let _ = conn.execute_batch(sql);
            conn.execute(
                "INSERT INTO _migrations (name, applied_at) VALUES (?1, ?2)",
                rusqlite::params![name, chrono::Utc::now().to_rfc3339()],
            )?;
        }
    }

    Ok(())
}

pub fn seed_defaults(conn: &Connection, project_name: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let project_id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT OR IGNORE INTO project (id, name, description, created_at, updated_at) VALUES (?1, ?2, '', ?3, ?3)",
        rusqlite::params![project_id, project_name, now],
    )?;

    for (name, desc) in DEFAULT_TAGS {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT OR IGNORE INTO tags (id, name, description, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, name, desc, now],
        )?;
    }

    for (name, desc) in DEFAULT_CONNECTION_TYPES {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT OR IGNORE INTO connection_types (id, name, description, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, name, desc, now],
        )?;
    }

    Ok(())
}
