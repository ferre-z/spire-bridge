/**
 * Activity heatmap — 7 rows × 24 cols grid, cell opacity = event count.
 */

const dayLabels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

// Deterministic fixture so the grid renders identically across renders.
function generate(): number[][] {
  return Array.from({ length: 7 }).map((_, d) =>
    Array.from({ length: 24 }).map((_, h) => {
      const peak = h >= 9 && h <= 18 ? 1.0 : 0.3;
      const weekend = d >= 5 ? 0.4 : 1.0;
      return peak * weekend * (0.4 + Math.abs(Math.sin(d * 7 + h * 1.3)) * 0.6);
    }),
  );
}

export function ActivityHeatmap() {
  const grid = generate();
  const max = Math.max(...grid.flat(), 0.001);

  return (
    <div className="space-y-1">
      {dayLabels.map((day, d) => (
        <div key={day} className="flex items-center gap-1">
          <span className="w-8 text-[10px] text-white/40 uppercase tracking-wider">
            {day}
          </span>
          <div className="flex-1 grid grid-cols-24 gap-0.5" style={{ gridTemplateColumns: "repeat(24, minmax(0, 1fr))" }}>
            {grid[d].map((intensity, h) => (
              <div
                key={h}
                className="aspect-square rounded-sm transition-all hover:ring-1 hover:ring-red-400/40"
                style={{
                  backgroundColor: `rgba(239, 68, 68, ${(intensity / max) * 0.85})`,
                }}
                title={`${day} ${String(h).padStart(2, "0")}:00 — ${Math.round(intensity * 10)} events`}
              />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}