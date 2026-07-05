/**
 * Number / date / token formatters used across the UI.
 *
 * All formatters are pure and deterministic — the same input always
 * yields the same string. They never throw, never call out to a
 * clock (`Date.now()`) so they're safe to use during render and in
 * tests with frozen time.
 *
 * Convention: every formatter takes a plain number/Date and returns
 * a plain string. No `Intl.NumberFormat` caches here because the
 * number of distinct inputs the UI ever sees is small (a handful of
 * cost values + a handful of token counts), so a cache would just
 * add bugs.
 */

/** Round to N decimals without trailing zeros (e.g. 1.20 → "1.2"). */
export function roundTo(value: number, decimals: number): number {
  if (!Number.isFinite(value)) return 0;
  const factor = 10 ** decimals;
  return Math.round(value * factor) / factor;
}

/**
 * Format a USD cost. Sub-cent amounts collapse to "<$0.01"; ≥ $1k
 * uses compact notation ("$1.23k"); ≥ $1M uses "M".
 */
export function formatCost(usd: number): string {
  if (!Number.isFinite(usd)) return "$0";
  if (usd === 0) return "$0";
  if (usd > 0 && usd < 0.01) return "<$0.01";
  if (usd >= 1_000_000) return `$${roundTo(usd / 1_000_000, 2)}M`;
  if (usd >= 1_000) return `$${roundTo(usd / 1_000, 2)}k`;
  if (usd >= 1) return `$${roundTo(usd, 2)}`;
  return `$${roundTo(usd, 3)}`;
}

/**
 * Format a token count. ≥ 1k uses compact "k"; ≥ 1M uses "M".
 * Reads naturally: `formatTokens(25400)` → "25.4k".
 */
export function formatTokens(n: number): string {
  if (!Number.isFinite(n)) return "0";
  const abs = Math.abs(n);
  if (abs >= 1_000_000) return `${roundTo(n / 1_000_000, 1)}M`;
  if (abs >= 1_000) return `${roundTo(n / 1_000, 1)}k`;
  return `${Math.round(n)}`;
}

/**
 * Format a duration in seconds as a human-readable string.
 *
 *  - < 60s        → "42s"
 *  - < 60m        → "3m 12s"
 *  - < 24h        → "1h 5m"
 *  - otherwise    → "2d 3h"
 *
 * Negative or non-finite inputs collapse to "—".
 */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return "—";
  const s = Math.floor(seconds);
  if (s < 60) return `${s}s`;
  if (s < 60 * 60) {
    const m = Math.floor(s / 60);
    const rem = s % 60;
    return rem > 0 ? `${m}m ${rem}s` : `${m}m`;
  }
  if (s < 60 * 60 * 24) {
    const h = Math.floor(s / 3600);
    const rem = Math.floor((s % 3600) / 60);
    return rem > 0 ? `${h}h ${rem}m` : `${h}h`;
  }
  const d = Math.floor(s / 86400);
  const rem = Math.floor((s % 86400) / 3600);
  return rem > 0 ? `${d}d ${rem}h` : `${d}d`;
}

/**
 * Format a Unix-epoch-second timestamp as a short relative
 * description: "just now" (<60s), "5m ago", "2h ago", "3d ago",
 * otherwise a short absolute date "Mar 4".
 *
 * `now` is injectable so tests can pin time without mocking globals.
 */
export function formatRelative(
  epochSeconds: number,
  now: number = Date.now() / 1000,
): string {
  if (!Number.isFinite(epochSeconds)) return "—";
  const delta = Math.max(0, now - epochSeconds);
  if (delta < 5) return "just now";
  if (delta < 60) return `${Math.floor(delta)}s ago`;
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
  if (delta < 86400) return `${Math.floor(delta / 3600)}h ago`;
  if (delta < 86400 * 7) return `${Math.floor(delta / 86400)}d ago`;
  // Older: short absolute date. Avoid pulling in `date-fns` here
  // because this file ships in the renderer's hot path.
  const d = new Date(epochSeconds * 1000);
  const months = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
  ];
  const m = months[d.getUTCMonth()] ?? "—";
  return `${m} ${d.getUTCDate()}`;
}

/**
 * Format a Unix-epoch-second timestamp as HH:MM:SS in UTC.
 * Used in the timeline for unambiguous event ordering across timezones.
 */
export function formatTimeUTC(epochSeconds: number): string {
  if (!Number.isFinite(epochSeconds)) return "—";
  const d = new Date(epochSeconds * 1000);
  const hh = String(d.getUTCHours()).padStart(2, "0");
  const mm = String(d.getUTCMinutes()).padStart(2, "0");
  const ss = String(d.getUTCSeconds()).padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
}

/**
 * Compact a project's path so long absolute paths fit in a 240px
 * sidebar row: "/home/u/projects/spire-bridge" → "~/…/spire-bridge".
 */
export function compactPath(path: string, maxLen = 40): string {
  if (path.length <= maxLen) return path;
  const head = path.slice(0, 2);
  const tail = path.slice(-(maxLen - 4));
  return `${head}/…/${tail}`;
}

/**
 * Extract a one-line preview from a tool-call payload.
 *
 * Returns "" when nothing useful can be extracted — callers should
 * show a generic "ran a tool" pill instead.
 */
export function summarizePayload(
  kind: string,
  payload: Record<string, unknown>,
): string {
  if (kind === "user_prompt") {
    const text = typeof payload.text === "string" ? payload.text : null;
    return text ? truncate(text, 140) : "";
  }
  if (kind === "assistant_text") {
    const text = typeof payload.text === "string" ? payload.text : null;
    return text ? truncate(text, 140) : "";
  }
  if (kind === "tool_call") {
    const name = typeof payload.name === "string" ? payload.name : "tool";
    const cmd =
      typeof payload.command === "string"
        ? payload.command
        : typeof payload.path === "string"
          ? payload.path
          : typeof payload.query === "string"
            ? payload.query
            : "";
    return cmd ? `${name}: ${truncate(cmd, 120)}` : name;
  }
  if (kind === "tool_result") {
    const ok = payload.ok === true;
    return ok ? "ok" : "error";
  }
  if (kind === "api_error" || kind === "api_refusal") {
    const msg = typeof payload.message === "string" ? payload.message : null;
    return msg ? truncate(msg, 120) : kind.replace("_", " ");
  }
  return kind.replace(/_/g, " ");
}

function truncate(s: string, max: number): string {
  if (s.length <= max) return s;
  return `${s.slice(0, max - 1)}…`;
}
