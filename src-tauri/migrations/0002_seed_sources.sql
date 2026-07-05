-- 0002_seed_sources.sql — register the three agents Spire observes.

INSERT OR IGNORE INTO agent_source (id, label, icon, color) VALUES
    ('claude',   'Claude Code', 'terminal', '#ef4444'),
    ('opencode', 'OpenCode',    'code',     '#f5f5f5'),
    ('hermes',   'Hermes',      'zap',      '#a3a3a3');