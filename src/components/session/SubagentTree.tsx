/**
 * Recursive subagent tree (Task 12).
 *
 * Renders parent → child sessions as a nested tree with animated
 * expand/collapse. Max depth is capped (4) to avoid runaway recursion
 * — any deeper child is treated as a leaf. Cost is rolled up
 * bottom-up so the total at a parent always equals the sum of
 * itself + all descendants.
 *
 * Why a cap? Phase 1 sessions are flat — subagents only nest two or
 * three levels deep. But we render defensively so a misbehaving
 * parent loop can't paint infinite rows.
 */

import { useMemo, useState } from "react";
import { motion, AnimatePresence } from "motion/react";
import {
  ChevronDown,
  ChevronRight,
  GitBranch,
  CheckCircle2,
  XCircle,
  Clock,
} from "lucide-react";
import { Link } from "@tanstack/react-router";
import { GlassPill } from "@/components/glass/GlassPill";
import { formatCost, formatDuration } from "@/lib/normalize/format";
import type { CanonicalSession } from "@/lib/normalize/types";

export interface SubagentTreeNode {
  session: CanonicalSession;
  children: SubagentTreeNode[];
  /** cost_usd + sum(children.cost) — computed at build time. */
  rolledCost: number;
}

const MAX_DEPTH = 4;

/**
 * Build the tree from a flat session list, capped at `maxDepth`.
 * Sessions without a parent_session_id become roots.
 */
export function buildSubagentTree(
  sessions: CanonicalSession[],
  maxDepth: number = MAX_DEPTH,
): SubagentTreeNode[] {
  if (sessions.length === 0) return [];

  const ids = new Set(sessions.map((s) => s.id));
  const byParent = new Map<string | null, CanonicalSession[]>();
  for (const s of sessions) {
    // If parent reference points to a session we don't have, treat
    // the session as a root. This way a misbehaving source can't
    // drop records silently — they show up at the top level.
    const rawParent = s.parent_session_id;
    const parent =
      rawParent !== null && ids.has(rawParent) ? rawParent : null;
    const list = byParent.get(parent) ?? [];
    list.push(s);
    byParent.set(parent, list);
  }

  const buildChildren = (
    parentId: string,
    depth: number,
  ): SubagentTreeNode[] => {
    if (depth > maxDepth) return [];
    const direct = byParent.get(parentId) ?? [];
    return direct.map((session) => {
      const children = buildChildren(session.id, depth + 1);
      const childCost = children.reduce((acc, c) => acc + c.rolledCost, 0);
      return {
        session,
        children,
        rolledCost: session.cost_usd + childCost,
      };
    });
  };

  const roots = byParent.get(null) ?? [];
  return roots.map((session) => {
    const children = buildChildren(session.id, 1);
    const childCost = children.reduce((acc, c) => acc + c.rolledCost, 0);
    return {
      session,
      children,
      rolledCost: session.cost_usd + childCost,
    };
  });
}

export function SubagentTree({
  sessions,
  defaultExpanded = true,
}: {
  sessions: CanonicalSession[];
  defaultExpanded?: boolean;
}) {
  const tree = useMemo(() => buildSubagentTree(sessions), [sessions]);

  if (tree.length === 0) {
    return (
      <p className="text-sm text-white/40 py-6 text-center">
        No sessions for this source yet.
      </p>
    );
  }

  return (
    <ul className="space-y-1">
      {tree.map((node) => (
        <TreeNode
          key={node.session.id}
          node={node}
          depth={0}
          defaultExpanded={defaultExpanded}
        />
      ))}
    </ul>
  );
}

function TreeNode({
  node,
  depth,
  defaultExpanded,
}: {
  node: SubagentTreeNode;
  depth: number;
  defaultExpanded: boolean;
}) {
  const [expanded, setExpanded] = useState(defaultExpanded && depth < 2);
  const hasChildren = node.children.length > 0;

  return (
    <li>
      <div
        className="flex items-center gap-2 py-1.5 px-2 rounded-lg hover:bg-white/[0.04] transition-colors group"
        style={{ paddingLeft: `${depth * 18 + 8}px` }}
      >
        {hasChildren ? (
          <button
            type="button"
            aria-label={expanded ? "Collapse" : "Expand"}
            onClick={() => setExpanded((v) => !v)}
            className="text-white/40 hover:text-white/80 transition-colors"
          >
            {expanded ? (
              <ChevronDown className="size-3.5" />
            ) : (
              <ChevronRight className="size-3.5" />
            )}
          </button>
        ) : (
          <span className="w-3.5" aria-hidden />
        )}

        <span className="text-white/40 shrink-0">
          {hasChildren ? (
            <GitBranch className="size-3.5" />
          ) : node.session.ended_at ? (
            <CheckCircle2 className="size-3.5 text-emerald-400/70" />
          ) : (
            <Clock className="size-3.5 text-blue-400/70" />
          )}
        </span>

        <Link
          to="/sessions/$sessionId"
          params={{ sessionId: node.session.id }}
          className="flex-1 min-w-0 truncate text-sm text-white/90 hover:text-white transition-colors"
        >
          {node.session.title ?? node.session.id.slice(0, 16)}
        </Link>

        <span className="text-[11px] text-white/40 tabular-nums shrink-0 hidden sm:inline">
          {formatDuration(
            node.session.ended_at
              ? node.session.ended_at - node.session.started_at
              : null,
          )}
        </span>

        {node.session.end_reason === "error" && (
          <GlassPill tone="error" className="hidden md:inline-flex">
            <XCircle className="size-3" /> err
          </GlassPill>
        )}

        <span className="text-xs text-red-300/90 tabular-nums shrink-0">
          {formatCost(node.rolledCost)}
        </span>
      </div>

      <AnimatePresence initial={false}>
        {expanded && hasChildren && (
          <motion.ul
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.18, ease: "easeOut" }}
            className="overflow-hidden"
          >
            {node.children.map((child) => (
              <TreeNode
                key={child.session.id}
                node={child}
                depth={depth + 1}
                defaultExpanded={defaultExpanded}
              />
            ))}
          </motion.ul>
        )}
      </AnimatePresence>
    </li>
  );
}
