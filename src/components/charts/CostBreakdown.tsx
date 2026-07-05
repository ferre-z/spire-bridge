/**
 * Cost breakdown — stacked bar chart per source per day.
 */

import {
  BarChart,
  Bar,
  ResponsiveContainer,
  XAxis,
  YAxis,
  Tooltip,
  Legend,
} from "recharts";
import type { CanonicalSession } from "@/lib/normalize/types";

const SOURCES = ["claude", "opencode", "hermes"] as const;
const SOURCE_COLORS: Record<string, string> = {
  claude: "#ef4444",
  opencode: "#f5f5f5",
  hermes: "#a3a3a3",
};

function dayBucket(epochSeconds: number): string {
  const d = new Date(epochSeconds * 1000);
  return `${d.getMonth() + 1}/${d.getDate()}`;
}

export function CostBreakdown({ sessions }: { sessions: CanonicalSession[] }) {
  // Aggregate by day × source.
  const byDay = new Map<string, Record<string, number>>();
  for (const s of sessions) {
    const day = dayBucket(s.started_at);
    if (!byDay.has(day)) byDay.set(day, {});
    const row = byDay.get(day)!;
    row[s.source_id] = (row[s.source_id] ?? 0) + s.cost_usd;
  }

  const data = Array.from(byDay.entries())
    .map(([day, sources]) => ({
      day,
      ...Object.fromEntries(
        SOURCES.map((src) => [src, sources[src] ?? 0]),
      ),
    }))
    .sort((a, b) => a.day.localeCompare(b.day))
    .slice(-30); // last 30 days max

  if (data.length === 0) {
    return (
      <p className="text-sm text-white/40 py-4 text-center">
        No cost data in this window.
      </p>
    );
  }

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart data={data} margin={{ top: 4, right: 4, left: 0, bottom: 0 }}>
        <XAxis
          dataKey="day"
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
          width={32}
          tickFormatter={(v) => `$${v.toFixed(0)}`}
        />
        <Tooltip
          contentStyle={{
            background: "rgba(10,10,10,0.92)",
            border: "1px solid rgba(255,255,255,0.08)",
            borderRadius: 8,
            fontSize: 12,
            color: "#f5f5f5",
          }}
          formatter={(v: number) => [`$${v.toFixed(4)}`]}
        />
        <Legend
          wrapperStyle={{ fontSize: 11, color: "#a3a3a3" }}
          iconType="circle"
        />
        {SOURCES.map((src) => (
          <Bar
            key={src}
            dataKey={src}
            stackId="cost"
            fill={SOURCE_COLORS[src]}
            radius={[2, 2, 0, 0]}
          />
        ))}
      </BarChart>
    </ResponsiveContainer>
  );
}