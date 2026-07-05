/**
 * Single timeline event row. Variant chosen by `event.kind`.
 */

import { motion } from "motion/react";
import { User, Bot, Wrench, AlertTriangle, Server } from "lucide-react";
import { formatRelative } from "@/lib/normalize/format";
import { summarizePayload } from "@/lib/normalize/format";
import type {
  CanonicalEvent,
  CanonicalSession,
} from "@/lib/normalize/types";

export function EventRow({
  event,
  session: _session,
}: {
  event: CanonicalEvent;
  session: CanonicalSession;
}) {
  const kind = event.kind;

  if (kind === "user_prompt") return <UserRow event={event} />;
  if (kind === "assistant_text") return <AssistantRow event={event} />;
  if (kind === "tool_call" || kind === "tool_result")
    return <ToolRow event={event} />;
  if (kind === "api_error" || kind === "api_refusal")
    return <ErrorRow event={event} />;
  return <MetaRow event={event} />;
}

function Row({
  icon,
  children,
  tone = "neutral",
}: {
  icon: React.ReactNode;
  children: React.ReactNode;
  tone?: "neutral" | "accent" | "warning" | "error" | "info";
}) {
  const border =
    tone === "error"
      ? "border-l-2 border-red-500"
      : tone === "warning"
        ? "border-l-2 border-yellow-500"
        : "";
  return (
    <motion.div
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.15 }}
      className={`flex items-start gap-3 px-3 py-2 border-b border-white/5 ${border}`}
    >
      <span className="shrink-0 mt-0.5 text-white/40">{icon}</span>
      <div className="flex-1 min-w-0">{children}</div>
    </motion.div>
  );
}

function UserRow({ event }: { event: CanonicalEvent }) {
  return (
    <Row icon={<User className="size-3.5" />}>
      <div className="flex items-center gap-2 text-xs text-white/40">
        <span>{formatRelative(event.occurred_at, Date.now() / 1000)}</span>
        <span className="text-white/30">user</span>
      </div>
      <p className="mt-1 text-sm text-white/90 break-words">
        {String((event.payload as { text?: string })?.text ?? "…")}
      </p>
    </Row>
  );
}

function AssistantRow({ event }: { event: CanonicalEvent }) {
  return (
    <Row icon={<Bot className="size-3.5" />} tone="info">
      <div className="flex items-center gap-2 text-xs text-white/40">
        <span>{formatRelative(event.occurred_at, Date.now() / 1000)}</span>
        <span className="text-blue-300/80">assistant</span>
      </div>
      <p className="mt-1 text-sm text-white/80 break-words whitespace-pre-wrap">
        {String((event.payload as { text?: string })?.text ?? "…")}
      </p>
    </Row>
  );
}

function ToolRow({ event }: { event: CanonicalEvent }) {
  const name = (event.payload as { name?: string })?.name ?? event.tool_name ?? "tool";
  const summary = summarizePayload(event.kind, event.payload);
  return (
    <Row icon={<Wrench className="size-3.5" />} tone="neutral">
      <div className="flex items-center gap-2 text-xs text-white/40">
        <span>{formatRelative(event.occurred_at, Date.now() / 1000)}</span>
        <span className="text-white/70 font-mono">{name}</span>
        {event.duration_ms != null && (
          <span>{event.duration_ms}ms</span>
        )}
      </div>
      <p className="mt-1 text-xs text-white/60 font-mono truncate">{summary}</p>
    </Row>
  );
}

function ErrorRow({ event }: { event: CanonicalEvent }) {
  return (
    <Row icon={<AlertTriangle className="size-3.5" />} tone="error">
      <div className="flex items-center gap-2 text-xs text-white/40">
        <span>{formatRelative(event.occurred_at, Date.now() / 1000)}</span>
        <span className="text-red-300">{event.kind}</span>
      </div>
      <p className="mt-1 text-sm text-red-200/90 break-words">
        {String((event.payload as { message?: string })?.message ?? "error")}
      </p>
    </Row>
  );
}

function MetaRow({ event }: { event: CanonicalEvent }) {
  return (
    <Row icon={<Server className="size-3.5" />}>
      <div className="flex items-center gap-2 text-xs text-white/40">
        <span>{formatRelative(event.occurred_at, Date.now() / 1000)}</span>
        <span>{event.kind}</span>
      </div>
    </Row>
  );
}