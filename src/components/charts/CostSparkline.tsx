/**
 * Cost sparkline — area chart of the last 24h hourly buckets.
 */

import {
  AreaChart,
  Area,
  ResponsiveContainer,
  XAxis,
  YAxis,
  Tooltip,
} from "recharts";

// Hardcoded fixture: last 24 hourly buckets. Real data comes from
// `dashboard_stats.hourly_buckets` (wired in Phase 2 when the IPC
// surface grows). For now we render a representative curve.
const data = Array.from({ length: 24 }).map((_, i) => ({
  hour: `${String((new Date().getHours() - 23 + i + 24) % 24).padStart(2, "0")}:00`,
  cost: Math.sin((i / 24) * Math.PI * 2) * 1.4 + Math.random() * 0.8 + 2,
}));

export function CostSparkline() {
  return (
    <ResponsiveContainer width="100%" height="100%">
      <AreaChart data={data} margin={{ top: 4, right: 4, left: 0, bottom: 0 }}>
        <defs>
          <linearGradient id="costGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#ef4444" stopOpacity={0.6} />
            <stop offset="100%" stopColor="#ef4444" stopOpacity={0} />
          </linearGradient>
        </defs>
        <XAxis
          dataKey="hour"
          stroke="#525252"
          fontSize={10}
          tickLine={false}
          axisLine={false}
          interval={5}
        />
        <YAxis
          stroke="#525252"
          fontSize={10}
          tickLine={false}
          axisLine={false}
          width={28}
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
          formatter={(v: number) => [`$${v.toFixed(2)}`, "Cost"]}
        />
        <Area
          type="monotone"
          dataKey="cost"
          stroke="#ef4444"
          strokeWidth={1.5}
          fill="url(#costGrad)"
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}