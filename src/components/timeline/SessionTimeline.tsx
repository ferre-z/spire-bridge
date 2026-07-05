/**
 * Virtualised session timeline.
 */

import { Virtuoso } from "react-virtuoso";
import { EventRow } from "./EventRow";
import type { CanonicalEvent, CanonicalSession } from "@/lib/normalize/types";

export function SessionTimeline({
  events,
  session,
}: {
  events: CanonicalEvent[];
  session: CanonicalSession;
}) {
  if (events.length === 0) {
    return (
      <p className="text-sm text-white/40 py-8 text-center">
        No events captured for this session.
      </p>
    );
  }

  return (
    <div className="h-[600px] rounded-lg overflow-hidden glass">
      <Virtuoso
        data={events}
        itemContent={(_, e) => (
          <EventRow event={e} session={session} />
        )}
        followOutput="smooth"
        className="h-full"
      />
    </div>
  );
}