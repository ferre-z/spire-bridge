/**
 * Global keyboard shortcuts.
 *
 * Subscribes to window keydown events and dispatches named actions
 * via a tiny pub/sub so any component (sidebar, command palette,
 * timeline) can register a handler without owning the keyboard.
 *
 * Bindings (plan Task 7.6):
 *
 *   - `⌘K` / `Ctrl+K`     → command palette
 *   - `⌘/` / `Ctrl+/`     → search focus
 *   - `⌘1-9` / `Ctrl+1-9` → navigate routes by index
 *   - `Esc`               → close dialog / dismiss overlay
 *
 * Shortcuts are suppressed when focus is in an editable element
 * (`<input>`, `<textarea>`, `[contenteditable]`) so typing `⌘K`
 * inside a search field doesn't yank the palette open.
 */

import { useEffect } from "react";

export type ShortcutAction =
  | "command-palette"
  | "search"
  | "navigate"
  | "escape";

export interface ShortcutEvent {
  action: ShortcutAction;
  /** Present for `navigate` actions; 1-indexed. */
  index?: number;
}

type Handler = (e: ShortcutEvent) => void;

const handlers = new Set<Handler>();

/**
 * Subscribe to shortcut events. Returns an unsubscribe function
 * suitable for `useEffect` cleanup.
 */
export function onShortcut(handler: Handler): () => void {
  handlers.add(handler);
  return () => {
    handlers.delete(handler);
  };
}

function dispatch(event: ShortcutEvent): void {
  for (const h of handlers) {
    try {
      h(event);
    } catch {
      // Never let one bad handler break the rest.
    }
  }
}

/**
 * True when the current focus target is a text-editing element.
 * We bail out of single-letter shortcuts in that case so the user
 * can type without the UI stealing their keypresses.
 */
function isEditingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  if (target.isContentEditable) return true;
  return false;
}

/**
 * Mount once at the app shell. Owns the window-level keydown
 * listener; safe to remount (effect cleanup removes the prior
 * listener before installing a new one).
 */
export function useShortcuts(): void {
  useEffect(() => {
    const onKeyDown = (ev: KeyboardEvent): void => {
      const mod = ev.metaKey || ev.ctrlKey;
      const editing = isEditingTarget(ev.target);

      // Escape is special: it must work even while editing (close
      // dialog, blur field). Only swallow it if a handler explicitly
      // stops propagation.
      if (ev.key === "Escape") {
        dispatch({ action: "escape" });
        return;
      }

      // Modifier-required shortcuts: never fire on bare keys.
      if (!mod) return;

      // When typing, ignore everything except Escape (handled above).
      if (editing) return;

      // `⌘K` / `Ctrl+K` — command palette.
      if (ev.key.toLowerCase() === "k") {
        ev.preventDefault();
        dispatch({ action: "command-palette" });
        return;
      }

      // `⌘/` — search focus.
      if (ev.key === "/") {
        ev.preventDefault();
        dispatch({ action: "search" });
        return;
      }

      // `⌘1-9` — navigate to the Nth sidebar item.
      if (/^[1-9]$/.test(ev.key)) {
        ev.preventDefault();
        dispatch({ action: "navigate", index: Number(ev.key) });
        return;
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, []);
}

/** Reset hook for tests — clears the handler set between runs. */
export function _resetShortcutsForTest(): void {
  handlers.clear();
}
