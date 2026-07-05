/**
 * Unit tests for the SubagentTree builder.
 *
 * We test the pure `buildSubagentTree` function rather than the
 * React component so the tests stay deterministic — no DOM, no
 * timers, no animation state. Visual behaviour is covered by Storybook
 * / QA, not here.
 */

import { describe, it, expect } from "vitest";
import { buildSubagentTree } from "@/components/session/SubagentTree";
import type { CanonicalSession } from "@/lib/normalize/types";

function sess(
  id: string,
  parent_session_id: string | null = null,
  extras: Partial<CanonicalSession> = {},
): CanonicalSession {
  return {
    id,
    source_id: "claude_code",
    native_id: id,
    title: id,
    project_dir: null,
    cwd: null,
    git_branch: null,
    model: null,
    started_at: 0,
    ended_at: 1,
    end_reason: null,
    input_tokens: 0,
    output_tokens: 0,
    cache_read: 0,
    cache_write: 0,
    reasoning_tokens: 0,
    cost_usd: 0,
    message_count: 0,
    tool_call_count: 0,
    parent_session_id,
    source_path: "",
    ...extras,
  };
}

describe("buildSubagentTree", () => {
  it("returns an empty list for no sessions", () => {
    expect(buildSubagentTree([])).toEqual([]);
  });

  it("treats parentless sessions as roots", () => {
    const a = sess("a");
    const b = sess("b");
    const tree = buildSubagentTree([a, b]);
    expect(tree).toHaveLength(2);
    expect(tree.map((t) => t.session.id).sort()).toEqual(["a", "b"]);
    expect(tree.every((t) => t.children.length === 0)).toBe(true);
  });

  it("nests children under their parent and rolls up cost", () => {
    const root = sess("root", null, { cost_usd: 1.0 });
    const child1 = sess("child1", "root", { cost_usd: 0.5 });
    const child2 = sess("child2", "root", { cost_usd: 0.25 });
    const tree = buildSubagentTree([root, child1, child2]);
    expect(tree).toHaveLength(1);
    expect(tree[0]!.session.id).toBe("root");
    expect(tree[0]!.children).toHaveLength(2);
    // 1.0 (self) + 0.5 + 0.25 (children) = 1.75
    expect(tree[0]!.rolledCost).toBeCloseTo(1.75, 5);
  });

  it("rolls up cost across multiple generations", () => {
    const root = sess("root", null, { cost_usd: 1 });
    const a = sess("a", "root", { cost_usd: 1 });
    const b = sess("b", "a", { cost_usd: 1 });
    const tree = buildSubagentTree([root, a, b]);
    expect(tree[0]!.rolledCost).toBe(3);
    expect(tree[0]!.children[0]!.rolledCost).toBe(2);
    expect(tree[0]!.children[0]!.children[0]!.rolledCost).toBe(1);
  });

  it("caps recursion at the provided maxDepth", () => {
    // 6-generation chain
    const ids = ["g0", "g1", "g2", "g3", "g4", "g5"];
    const sessions = ids.map((id, i) =>
      sess(id, i === 0 ? null : ids[i - 1]!),
    );
    const tree = buildSubagentTree(sessions, 2);
    // Root is g0; depth 0 = g0, depth 1 = g1, depth 2 = g2.
    // Anything below should be a leaf.
    expect(tree[0]!.session.id).toBe("g0");
    const g2node = tree[0]!.children[0]!.children[0];
    expect(g2node?.session.id).toBe("g2");
    // g2 has no rendered children even though g3–g5 reference it.
    expect(g2node?.children).toHaveLength(0);
    // And rolledCost stops at the cap as well (cap acts as trim).
    expect(tree[0]!.rolledCost).toBe(0);
  });

  it("ignores children whose parent is missing from the input", () => {
    // Orphan — child references a parent that doesn't exist.
    const orphan = sess("orphan", "ghost", { cost_usd: 0.5 });
    const tree = buildSubagentTree([orphan]);
    // Orphan falls through to root since its parent isn't here.
    expect(tree).toHaveLength(1);
    expect(tree[0]!.session.id).toBe("orphan");
  });
});
