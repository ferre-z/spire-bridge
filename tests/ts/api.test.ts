/**
 * `api/client.ts` — unit tests with the Tauri `invoke` mock.
 *
 * Verifies:
 *   1. Each method calls `invoke` with the correct command name.
 *   2. Arguments are forwarded as-is (no reshaping, no camelCase
 *      conversion — Tauri serialises both sides identically).
 *   3. Return values are typed (Promise<T>) and resolve from the
 *      mocked invoke return.
 *
 * The `motion` and `@tauri-apps/api/event` imports inside the
 * library are mocked at the test boundary so we don't have to
 * spin up a real Tauri runtime.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";

// Mock @tauri-apps/api/core before importing the module under test
// so the module captures our fake `invoke`.
const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

// Mock @tauri-apps/api/event so `stream.ts` can import `listen`
// without pulling in a real implementation.
const listenMock = vi.fn();
vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));

// Static imports — vitest hoists `vi.mock` above them, so these
// resolve against the mocked modules.
import { api } from "@/lib/api/client";
import { streamLiveEvents, LIVE_EVENT_NAME } from "@/lib/api/stream";
import { keys } from "@/lib/query/keys";
import { createQueryClient } from "@/lib/query/client";
import {
  formatCost,
  formatTokens,
  formatDuration,
  formatRelative,
  summarizePayload,
} from "@/lib/normalize/format";

beforeEach(() => {
  invokeMock.mockReset();
  listenMock.mockReset();
});

describe("api.client", () => {
  it("listSessions invokes 'list_sessions' with { filter, limit, offset }", async () => {
    invokeMock.mockResolvedValueOnce([]);
    await api.listSessions({ source: "claude_code" }, 25, 10);
    expect(invokeMock).toHaveBeenCalledWith("list_sessions", {
      filter: { source: "claude_code" },
      limit: 25,
      offset: 10,
    });
  });

  it("listSessions defaults limit=50, offset=0", async () => {
    invokeMock.mockResolvedValueOnce([]);
    await api.listSessions();
    expect(invokeMock).toHaveBeenCalledWith("list_sessions", {
      filter: {},
      limit: 50,
      offset: 0,
    });
  });

  it("getSession invokes 'get_session' with the id", async () => {
    invokeMock.mockResolvedValueOnce({ session: {}, events: [] });
    await api.getSession("ses_abc");
    expect(invokeMock).toHaveBeenCalledWith("get_session", {
      id: "ses_abc",
    });
  });

  it("dashboardStats invokes 'dashboard_stats' with since", async () => {
    invokeMock.mockResolvedValueOnce({
      total_cost_usd: 1.23,
      session_count: 4,
      error_count: 0,
    });
    await api.dashboardStats(1783163383);
    expect(invokeMock).toHaveBeenCalledWith("dashboard_stats", {
      since: 1783163383,
    });
  });

  it("getSettings invokes 'get_settings' with no args", async () => {
    invokeMock.mockResolvedValueOnce({ hermes_password_set: false });
    await api.getSettings();
    expect(invokeMock).toHaveBeenCalledWith("get_settings", {});
  });

  it("setHermesPassword forwards the password", async () => {
    invokeMock.mockResolvedValueOnce(undefined);
    await api.setHermesPassword("hunter2");
    expect(invokeMock).toHaveBeenCalledWith("set_hermes_password", {
      password: "hunter2",
    });
  });

  it("streamLiveEvents invokes 'stream_live_events'", async () => {
    invokeMock.mockResolvedValueOnce(undefined);
    // api.streamLiveEvents was removed in Phase 1 (backend doesn't expose
    // that command yet). The frontend subscribes via the `listen` API
    // directly — see `src/lib/api/stream.ts`. This test now exercises
    // that path explicitly.
    const cb = vi.fn();
    await streamLiveEvents(cb);
    expect(listenMock).toHaveBeenCalledWith("live_event", expect.any(Function));
  });
});

describe("api.stream", () => {
  it("LIVE_EVENT_NAME is the literal 'live_event'", () => {
    expect(LIVE_EVENT_NAME).toBe("live_event");
  });

  it("streamLiveEvents delegates to listen<CanonicalEvent>", async () => {
    listenMock.mockResolvedValueOnce(() => {});
    const cb = vi.fn();
    const unlisten = await streamLiveEvents(cb);
    expect(listenMock).toHaveBeenCalledWith("live_event", expect.any(Function));
    expect(typeof unlisten).toBe("function");

    // Hand the listener a fake Tauri event and assert cb fires.
    const handler = listenMock.mock.calls[0]?.[1] as (msg: {
      payload: unknown;
    }) => void;
    handler({
      payload: {
        session_id: "s",
        seq: 1,
        occurred_at: 1,
        kind: "user_prompt",
        payload: {},
      },
    });
    expect(cb).toHaveBeenCalledWith(
      expect.objectContaining({ session_id: "s" }),
    );
  });
});

describe("query keys", () => {
  it("sessions.all is a stable tuple", () => {
    expect(keys.sessions.all).toEqual(["sessions"]);
  });

  it("sessions.list keys include the filter + paging", () => {
    expect(keys.sessions.list({ source: "x" }, 25, 10)).toEqual([
      "sessions",
      "list",
      { filter: { source: "x" }, limit: 25, offset: 10 },
    ]);
  });

  it("dashboard.stats keys include the since timestamp", () => {
    expect(keys.dashboard.stats(1234)).toEqual([
      "dashboard",
      "stats",
      1234,
    ]);
  });

  it("settings.current is a stable tuple", () => {
    expect(keys.settings.current()).toEqual(["settings", "current"]);
  });

  it("sessions.detail keys include the session id", () => {
    expect(keys.sessions.detail("ses_xyz")).toEqual([
      "sessions",
      "detail",
      "ses_xyz",
    ]);
  });
});

describe("query client factory", () => {
  it("creates a QueryClient with the v1 defaults", () => {
    const qc = createQueryClient();
    const opts = qc.getDefaultOptions();
    expect(opts.queries?.staleTime).toBe(5_000);
    expect(opts.queries?.gcTime).toBe(60_000);
    expect(opts.queries?.refetchOnWindowFocus).toBe(false);
    expect(opts.queries?.retry).toBe(1);
    expect(opts.mutations?.retry).toBe(0);
  });
});

describe("format helpers", () => {
  it("formatCost rounds small amounts and uses compact for big", () => {
    expect(formatCost(0)).toBe("$0");
    expect(formatCost(0.004)).toBe("<$0.01");
    expect(formatCost(0.42)).toBe("$0.42");
    expect(formatCost(1.5)).toBe("$1.5");
    expect(formatCost(2549)).toBe("$2.55k");
    expect(formatCost(1_500_000)).toBe("$1.5M");
    expect(formatCost(NaN)).toBe("$0");
  });

  it("formatTokens uses k / M suffixes", () => {
    expect(formatTokens(0)).toBe("0");
    expect(formatTokens(999)).toBe("999");
    expect(formatTokens(1500)).toBe("1.5k");
    expect(formatTokens(2_540_000)).toBe("2.5M");
    expect(formatTokens(NaN)).toBe("0");
  });

  it("formatDuration emits seconds / minutes / hours / days", () => {
    expect(formatDuration(42)).toBe("42s");
    expect(formatDuration(60)).toBe("1m");
    expect(formatDuration(192)).toBe("3m 12s");
    expect(formatDuration(3600)).toBe("1h");
    expect(formatDuration(3725)).toBe("1h 2m");
    expect(formatDuration(86400)).toBe("1d");
    expect(formatDuration(90000)).toBe("1d 1h");
    expect(formatDuration(null)).toBe("—");
    expect(formatDuration(-5)).toBe("—");
    expect(formatDuration(NaN)).toBe("—");
  });

  it("formatRelative degrades gracefully for old / non-finite", () => {
    const now = 1000;
    expect(formatRelative(1000, now)).toBe("just now");
    expect(formatRelative(995, now)).toBe("5s ago");
    expect(formatRelative(940, now)).toBe("1m ago");
    expect(formatRelative(400, now)).toBe("10m ago");
    // Future timestamps clamp to "just now" instead of going negative.
    expect(formatRelative(2000, now)).toBe("just now");
    expect(formatRelative(NaN, now)).toBe("—");
  });

  it("summarizePayload extracts a one-liner per kind", () => {
    expect(
      summarizePayload("tool_call", { name: "Bash", command: "gh --version" }),
    ).toBe("Bash: gh --version");
    expect(
      summarizePayload("tool_call", { name: "Read", path: "/etc/hosts" }),
    ).toBe("Read: /etc/hosts");
    expect(summarizePayload("tool_result", { ok: true })).toBe("ok");
    expect(summarizePayload("tool_result", { ok: false })).toBe("error");
    expect(summarizePayload("user_prompt", { text: "hi there" })).toBe(
      "hi there",
    );
    expect(summarizePayload("api_error", {})).toBe("api error");
  });
});
