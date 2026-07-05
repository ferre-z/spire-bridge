/**
 * Shared Motion variants — DRY for common transitions.
 * All respect `prefers-reduced-motion` via Motion's `useReducedMotion`.
 */

import type { Variants } from "motion/react";

export const fadeInUp: Variants = {
  initial: { opacity: 0, y: 8 },
  animate: { opacity: 1, y: 0, transition: { duration: 0.18, ease: "easeOut" } },
  exit: { opacity: 0, y: -8 },
};

export const fadeIn: Variants = {
  initial: { opacity: 0 },
  animate: { opacity: 1, transition: { duration: 0.15 } },
  exit: { opacity: 0 },
};

export const staggerList = (delay = 0.03, max = 0.2): Variants => ({
  initial: {},
  animate: {
    transition: {
      staggerChildren: Math.min(delay, max / 5),
    },
  },
});

export const scaleIn: Variants = {
  initial: { opacity: 0, scale: 0.96 },
  animate: { opacity: 1, scale: 1, transition: { duration: 0.15, ease: "easeOut" } },
  exit: { opacity: 0, scale: 0.96 },
};