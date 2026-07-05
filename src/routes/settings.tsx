/**
 * Settings page (Task 14).
 *
 * Sections: Sources (per-agent enable), Hermes Auth, Display, About.
 */

import { useEffect, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { motion, AnimatePresence } from "motion/react";
import { Check, KeyRound, Eye, Info } from "lucide-react";
import { GlassCard, GlassPill, GlassButton } from "@/components/glass";
import { api } from "@/lib/api/client";
import { keys } from "@/lib/query/keys";

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
});

const SOURCES = ["claude", "opencode", "hermes"] as const;

function SettingsPage() {
  const qc = useQueryClient();
  const settingsQuery = useQuery({
    queryKey: keys.settings,
    queryFn: () => api.getSettings(),
    staleTime: 30_000,
  });

  const [password, setPassword] = useState("");
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function savePassword() {
    if (password.length < 4) {
      setError("Password must be at least 4 characters.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await api.setHermesPassword(password);
      setPassword("");
      setSaved(true);
      qc.invalidateQueries({ queryKey: keys.settings });
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="p-8 space-y-6 max-w-3xl">
      <header>
        <h1 className="text-2xl font-medium tracking-tight text-white">
          Settings
        </h1>
        <p className="mt-1 text-sm text-white/50">
          Configure Spire Bridge for your environment.
        </p>
      </header>

      {/* Sources */}
      <Section icon={<Eye className="size-3.5" />} title="Sources">
        <p className="text-sm text-white/50 mb-4">
          Enable or disable each agent. Disabled sources are hidden from
          the dashboard.
        </p>
        <div className="space-y-2">
          {SOURCES.map((s) => {
            const enabled = settingsQuery.data?.sources[s] ?? false;
            return (
              <div
                key={s}
                className="flex items-center justify-between p-3 glass rounded-xl"
              >
                <div>
                  <div className="text-sm font-medium text-white">{s}</div>
                  <div className="text-xs text-white/50">
                    {s === "claude" && "Anthropic Claude Code CLI sessions"}
                    {s === "opencode" && "OpenCode TUI / server sessions"}
                    {s === "hermes" && "Hermes Agent gateway sessions"}
                  </div>
                </div>
                <TogglePill
                  label={enabled ? "Enabled" : "Disabled"}
                  on={enabled}
                  onClick={() => {
                    /* TODO: persist via api when backend supports it */
                  }}
                />
              </div>
            );
          })}
        </div>
      </Section>

      {/* Hermes auth */}
      <Section icon={<KeyRound className="size-3.5" />} title="Hermes auth">
        <p className="text-sm text-white/50 mb-4">
          Stored in the OS keychain (encrypted at rest). Never written to
          disk in plain text.
        </p>
        <div className="flex items-center gap-2 mb-3">
          <GlassPill tone={settingsQuery.data?.hermes_password_set ? "success" : "neutral"}>
            {settingsQuery.data?.hermes_password_set ? "Password set" : "Not configured"}
          </GlassPill>
        </div>
        <div className="flex items-center gap-2">
          <input
            type="password"
            placeholder="New password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") savePassword();
            }}
            className="flex-1 glass rounded-lg px-3 py-1.5 text-sm bg-white/[0.04] border-white/10 focus:border-red-400/40 focus:outline-none transition-colors"
          />
          <GlassButton variant="primary" onClick={savePassword} disabled={saving}>
            {saving ? "Saving…" : "Save"}
          </GlassButton>
        </div>
        <AnimatePresence>
          {saved && (
            <motion.div
              initial={{ opacity: 0, y: -4 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              className="mt-2 flex items-center gap-1.5 text-xs text-emerald-300"
            >
              <Check className="size-3" /> Saved to keychain.
            </motion.div>
          )}
          {error && (
            <motion.div
              initial={{ opacity: 0, y: -4 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              className="mt-2 text-xs text-red-300"
            >
              {error}
            </motion.div>
          )}
        </AnimatePresence>
      </Section>

      {/* Display */}
      <Section icon={<Eye className="size-3.5" />} title="Display">
        <p className="text-sm text-white/50">
          Theme follows system. Custom themes are Phase 2.
        </p>
      </Section>

      {/* About */}
      <Section icon={<Info className="size-3.5" />} title="About">
        <p className="text-sm text-white/50">
          Spire Bridge v0.1.0 — MIT licensed. No telemetry. No third-party
          network calls.
        </p>
      </Section>
    </div>
  );
}

function Section({
  icon,
  title,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <GlassCard className="p-5">
      <div className="flex items-center gap-2 text-white/70 mb-4">
        {icon}
        <h2 className="text-sm font-medium">{title}</h2>
      </div>
      {children}
    </GlassCard>
  );
}

function TogglePill({
  label,
  on,
  onClick,
}: {
  label: string;
  on: boolean;
  onClick: () => void;
}) {
  return (
    <button onClick={onClick}>
      <GlassPill tone={on ? "success" : "neutral"}>{label}</GlassPill>
    </button>
  );
}