/**
 * Cumulative cost step chart for a session's events.
 */

import {
  LineChart,
  Line,
  ResponsiveContainer,
  XAxis,
  YAxis,
  Tooltip,
} from "recharts";
import type { CanonicalEvent } from "@/lib/normalize/types";

export function SessionCostChart({ events }: { events: CanonicalEvent[] }) {
  let running = 0;
  const data = events.map((e) => {
    running += e.cost_usd;
    return {
      seq: e.seq,
      cumulative: Math.round(running * 10_000) / 10_000,
    };
  });

  if (data.length === 0) {
    return (
      <p className="text-sm text-white/40 py-4 text-center">
        No cost data yet.
      </p>
    );
  }

  return (
    <ResponsiveContainer width="100%" height="100%">
      <LineChart data={data} margin={{ top: 4, right: 4, left: 0, bottom: 0 }}>
        <XAxis
          dataKey="seq"
          stroke="#525252"
          fontSize={10}
          tickLine={false}
          axisLine={false}
        />
        <YAxis
          stroke="#525252"
          fontSize={10}
          tickLine={false}
          axisLine={false}
          width={36}
          tickFormatter={(v) => `$${v.toFixed(2)}`}
        />
        <Tooltip
          contentStyle={{
            background: "rgba(10,10,10,0.92)",
            border: "1px solid rgba(255,255,255,0.08)",
            borderRadius: 8,
            fontSize: 12,
            color: "#f5f5f5",
          }}
          formatter={(v: number) => [`$${v.toFixed(4)}`, "Cumulative"]}
        />
        <Line
          type="stepAfter"
          dataKey="cumulative"
          stroke="#ef4444"
          strokeWidth={1.5}
          dot={false}
        />
      </LineChart>
    </ResponsiveContainer>
  );
}