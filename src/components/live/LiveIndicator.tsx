/**
 * Pulsing red dot — indicates the live event bus is connected.
 */

import { motion } from "motion/react";

export function LiveIndicator({
  size = 8,
  active = true,
}: {
  size?: number;
  active?: boolean;
}) {
  return (
    <motion.span
      style={{
        width: size,
        height: size,
        backgroundColor: active ? "#ef4444" : "#525252",
        borderRadius: "50%",
        display: "inline-block",
      }}
      animate={active ? { scale: [1, 1.4, 1], opacity: [1, 0.6, 1] } : {}}
      transition={
        active
          ? { duration: 1.5, repeat: Infinity, ease: "easeInOut" }
          : { duration: 0 }
      }
      aria-label={active ? "live" : "idle"}
    />
  );
}