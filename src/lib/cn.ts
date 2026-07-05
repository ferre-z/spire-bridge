import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/**
 * Compose class names with `clsx`, then de-duplicate conflicting
 * Tailwind utilities with `tailwind-merge`.
 *
 * Usage:
 *   cn("p-2", isActive && "bg-red-500", "p-4")   → "bg-red-500 p-4"
 */
export const cn = (...inputs: ClassValue[]): string => twMerge(clsx(inputs));
