/**
 * Live event stream — virtualised scrolling list with auto-follow.
 */

import { useEffect, useRef, useState } from "react";
import { Virtuoso } from "react-virtuoso";
import { useLiveStore } from "@/lib/hooks/useLiveEvents";
import { LiveIndicator } from "@/components/live/LiveIndicator";
import { summarizePayload } from "@/lib/normalize/format";
import { formatRelative } from "@/lib/normalize/format";
import type { CanonicalEvent } from "@/lib/normalize/types";

const MAX_VISIBLE = 200;

export function LiveStream() {
  const events = useLiveStore((s) => s.events);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [following, setFollowing] = useState(true);

  // When new events arrive and we're "following", scroll to top (newest first).
  useEffect(() => {
    if (following && containerRef.current) {
      containerRef.current.scrollTo({ top: 0, behavior: "smooth" });
    }
  }, [events.length, following]);

  const visible = events.slice(0, MAX_VISIBLE);

  return (
    <div className="relative">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2 text-xs text-white/60">
          <LiveIndicator size={6} active={events.length > 0} />
          {events.length > 0 ? `${events.length} events` : "no events yet"}
        </div>
        {!following && (
          <button
            onClick={() => setFollowing(true)}
            className="text-xs text-red-300 hover:text-red-200 transition-colors"
          >
            Jump to latest ↓
          </button>
        )}
      </div>

      <div
        ref={containerRef}
        onScroll={(e) => {
          const el = e.currentTarget;
          if (el.scrollTop > 20 && following) setFollowing(false);
        }}
        className="h-72 overflow-y-auto rounded-lg"
      >
        {visible.length === 0 ? (
          <p className="text-sm text-white/40 py-12 text-center">
            Waiting for the first event…
          </p>
        ) : (
          <ul className="space-y-1 px-1">
            {visible.map((e) => (
              <li key={`${e.session_id}-${e.seq}`}>
                <EventLine event={e} />
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

function EventLine({ event }: { event: CanonicalEvent }) {
  return (
    <div className="flex items-start gap-2 text-xs py-1.5 px-2 rounded hover:bg-white/5 transition-colors">
      <span className="text-white/30 tabular-nums shrink-0 w-16">
        {formatRelative(event.occurred_at, Date.now() / 1000)}
      </span>
      <span className="text-red-300/80 shrink-0">{event.kind}</span>
      <span className="text-white/70 truncate flex-1">
        {summarizePayload(event.kind, event.payload)}
      </span>
    </div>
  );
}