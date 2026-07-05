-- 0002_seed_sources.sql — seed the three known agent sources.
--
-- `id` values are stable contract strings used in IPC filter args,
-- dashboard facet URLs, and renderer lookups. Never rename them.

INSERT INTO agent_source (id, label, icon, color) VALUES
  ('claude_code', 'Claude Code', 'terminal',  '#ef4444'),
  ('opencode',    'OpenCode',    'code',      '#f5f5f5'),
  ('hermes',      'Hermes',      'zap',       '#a3a3a3');
