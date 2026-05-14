"use client";

import { useState, useEffect, useCallback } from "react";
import { useSession } from "@/lib/auth/client";
import { useRouter } from "next/navigation";
import type { UserPreferences } from "@/lib/types";
import { DEFAULT_PREFERENCES, AVAILABLE_MODELS, MODEL_CONTEXT_LIMITS } from "@/lib/types";
import {
  Bot, Cpu, Sliders, Sun, Moon, Monitor,
  Thermometer, Hash, Shield, Eye, Code,
  ChevronLeft, Save, RotateCcw,
} from "lucide-react";

export default function SettingsPage() {
  const { data: session, isPending } = useSession();
  const router = useRouter();
  const [prefs, setPrefs] = useState<UserPreferences>(DEFAULT_PREFERENCES);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  // Load preferences
  useEffect(() => {
    fetch("/api/settings")
      .then(r => r.json())
      .then(d => { if (d.preferences) setPrefs({ ...DEFAULT_PREFERENCES, ...d.preferences }); })
      .catch((err) => console.warn("[Settings] failed to load preferences:", err))
      .finally(() => setLoading(false));
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      await fetch("/api/settings", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ preferences: prefs }),
      });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      console.error("[Settings] failed to save preferences:", err);
      setSaved(false);
      setSaveError("Failed to save. Check your connection and try again.");
      setTimeout(() => setSaveError(null), 5000);
    } finally { setSaving(false); }
  }, [prefs]);

  const handleReset = useCallback(() => {
    setPrefs(DEFAULT_PREFERENCES);
  }, []);

  // When model changes, auto-adjust context limit
  const handleModelChange = useCallback((model: string) => {
    const limit = MODEL_CONTEXT_LIMITS[model] || 128000;
    setPrefs(p => ({ ...p, model, contextLimit: limit }));
  }, []);

  useEffect(() => {
    if (!isPending && !session) {
      router.push("/login");
    }
  }, [isPending, session, router]);

  if (isPending || loading) {
    return <div className="flex h-screen items-center justify-center bg-bg"><div className="animate-pulse text-fg-faint text-sm">Loading…</div></div>;
  }

  if (!session) {
    return <div className="flex h-screen items-center justify-center bg-bg"><div className="animate-pulse text-fg-faint text-sm">Redirecting…</div></div>;
  }

  const agentModes = [
    { id: "agent" as const, label: "Agent", desc: "Full autonomy with permission prompts", icon: Bot },
    { id: "plan" as const, label: "Plan", desc: "Plan-first — research before acting", icon: Cpu },
    { id: "yolo" as const, label: "YOLO", desc: "Auto-approve all actions, no prompts", icon: Shield },
  ];

  return (
    <div className="flex min-h-screen flex-col bg-bg text-fg">
      {/* Header */}
      <header className="flex items-center gap-3 border-b border-border px-4 py-3">
        <button onClick={() => { if (window.history.length > 1) router.back(); else router.push("/"); }} className="text-fg-faint hover:text-fg transition-colors">
          <ChevronLeft className="h-5 w-5" />
        </button>
        <Sliders className="h-4 w-4 text-amber" />
        <h1 className="text-sm font-bold text-amber-light">Settings</h1>
        <span className="ml-auto text-xs text-fg-faint font-mono">{session.user?.email}</span>
      </header>

      <div className="flex-1 overflow-y-auto px-4 py-6 max-w-lg mx-auto w-full space-y-8">
        {/* Agent Mode */}
        <section>
          <h2 className="mb-3 text-xs font-semibold uppercase tracking-widest text-fg-faint">Agent Mode</h2>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
            {agentModes.map(m => {
              const Icon = m.icon;
              const active = prefs.agentMode === m.id;
              return (
                <button
                  key={m.id}
                  onClick={() => setPrefs(p => ({ ...p, agentMode: m.id }))}
                  className={`flex flex-col items-center gap-1.5 rounded border p-3 text-left transition-colors ${
                    active
                      ? "border-amber/40 bg-amber/5 text-amber-light"
                      : "border-border bg-card text-fg-dim hover:border-amber/20"
                  }`}
                >
                  <Icon className={`h-5 w-5 ${active ? "text-amber" : "text-fg-faint"}`} />
                  <span className="text-xs font-semibold">{m.label}</span>
                  <span className="text-[10px] leading-tight text-center opacity-70">{m.desc}</span>
                </button>
              );
            })}
          </div>
        </section>

        {/* Model */}
        <section>
          <h2 className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-widest text-fg-faint">
            <Cpu className="h-3.5 w-3.5" /> Model
          </h2>
          <div className="space-y-2">
            {AVAILABLE_MODELS.map(m => (
              <button
                key={m.id}
                onClick={() => handleModelChange(m.id)}
                className={`flex w-full items-center gap-3 rounded border px-3 py-2.5 text-left transition-colors ${
                  prefs.model === m.id
                    ? "border-amber/30 bg-amber/5"
                    : "border-border bg-card hover:border-amber/20"
                }`}
              >
                <span className={`h-2 w-2 shrink-0 rounded-full ${prefs.model === m.id ? "bg-amber" : "bg-fg-faint"}`} />
                <div className="min-w-0 flex-1">
                  <p className="text-sm text-fg">{m.name}</p>
                  <p className="text-xs text-fg-faint">{m.desc}</p>
                </div>
                {prefs.model === m.id && (
                  <span className="text-[10px] text-amber font-mono">active</span>
                )}
              </button>
            ))}
          </div>
        </section>

        {/* Context Limit */}
        <section>
          <h2 className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-widest text-fg-faint">
            <Hash className="h-3.5 w-3.5" /> Context Limit
          </h2>
          <div className="space-y-2">
            <input
              type="range"
              min={16000}
              max={1_000_000}
              step={16000}
              value={prefs.contextLimit}
              onChange={e => setPrefs(p => ({ ...p, contextLimit: parseInt(e.target.value) }))}
              className="w-full accent-amber"
            />
            <div className="flex justify-between text-xs text-fg-faint">
              <span>16K</span>
              <span className="font-mono text-amber">{prefs.contextLimit.toLocaleString()} tokens</span>
              <span>1M</span>
            </div>
          </div>
        </section>

        {/* Temperature */}
        <section>
          <h2 className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-widest text-fg-faint">
            <Thermometer className="h-3.5 w-3.5" /> Temperature
          </h2>
          <div className="space-y-2">
            <input
              type="range"
              min={0}
              max={2}
              step={0.1}
              value={prefs.temperature}
              onChange={e => setPrefs(p => ({ ...p, temperature: parseFloat(e.target.value) }))}
              className="w-full accent-amber"
            />
            <div className="flex justify-between text-xs text-fg-faint">
              <span>Precise (0)</span>
              <span className="font-mono text-amber">{prefs.temperature.toFixed(1)}</span>
              <span>Creative (2.0)</span>
            </div>
          </div>
        </section>

        {/* Max Output Tokens */}
        <section>
          <h2 className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-widest text-fg-faint">
            <Hash className="h-3.5 w-3.5" /> Max Output Tokens
          </h2>
          <select
            value={prefs.maxTokens}
            onChange={e => setPrefs(p => ({ ...p, maxTokens: parseInt(e.target.value) }))}
            className="w-full rounded border border-border bg-card px-3 py-2 text-sm text-fg focus:border-amber/40 focus:outline-none"
          >
            {[1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072].map(n => (
              <option key={n} value={n}>{n.toLocaleString()} tokens</option>
            ))}
          </select>
        </section>

        {/* Theme */}
        <section>
          <h2 className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-widest text-fg-faint">
            <Eye className="h-3.5 w-3.5" /> Theme
          </h2>
          <div className="grid grid-cols-3 gap-2">
            {([
              { id: "dark" as const, label: "Dark", icon: Moon },
              { id: "light" as const, label: "Light", icon: Sun },
              { id: "system" as const, label: "System", icon: Monitor },
            ]).map(t => {
              const Icon = t.icon;
              return (
                <button
                  key={t.id}
                  onClick={() => setPrefs(p => ({ ...p, theme: t.id }))}
                  className={`flex flex-col items-center gap-1 rounded border p-2.5 transition-colors ${
                    prefs.theme === t.id ? "border-amber/40 bg-amber/5 text-amber-light" : "border-border bg-card text-fg-dim hover:border-amber/20"
                  }`}
                >
                  <Icon className="h-4 w-4" />
                  <span className="text-xs">{t.label}</span>
                </button>
              );
            })}
          </div>
        </section>

        {/* Display Options */}
        <section>
          <h2 className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-widest text-fg-faint">
            <Code className="h-3.5 w-3.5" /> Display
          </h2>
          <div className="space-y-3 rounded border border-border bg-card p-4">
            <label className="flex items-center justify-between gap-3">
              <div>
                <p className="text-sm text-fg">Markdown Rendering</p>
                <p className="text-xs text-fg-faint">Render bold, italic, lists, tables in responses</p>
              </div>
              <Toggle active={prefs.markdownRender} onChange={v => setPrefs(p => ({ ...p, markdownRender: v }))} />
            </label>
            <label className="flex items-center justify-between gap-3 border-t border-border pt-3">
              <div>
                <p className="text-sm text-fg">Syntax Highlighting</p>
                <p className="text-xs text-fg-faint">Colorize code blocks in responses</p>
              </div>
              <Toggle active={prefs.syntaxHighlight} onChange={v => setPrefs(p => ({ ...p, syntaxHighlight: v }))} />
            </label>
            <label className="flex items-center justify-between gap-3 border-t border-border pt-3">
              <div>
                <p className="text-sm text-fg">Auto-Approve Tools</p>
                <p className="text-xs text-fg-faint">Skip permission prompts for tool calls</p>
              </div>
              <Toggle active={prefs.autoApprove} onChange={v => setPrefs(p => ({ ...p, autoApprove: v }))} />
            </label>
          </div>
        </section>

        {/* Hooks / Plugins */}
        <section>
          <h2 className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-widest text-fg-faint">
            <Bot className="h-3.5 w-3.5" /> Hooks & Plugins
          </h2>
          <div className="space-y-3 rounded border border-border bg-card p-4">
            <div>
              <label className="mb-1 block text-xs text-fg-faint">System Prompt Extension</label>
              <textarea
                value={prefs.hooks?.systemPromptExtension || ""}
                onChange={e => setPrefs(p => ({ ...p, hooks: { ...(p.hooks || {}), systemPromptExtension: e.target.value } }))}
                className="w-full rounded border border-border bg-bg px-3 py-2 text-sm text-fg focus:border-amber/40 focus:outline-none resize-none font-mono"
                rows={3}
                placeholder="Additional instructions appended to the system prompt..."
              />
            </div>
            <div className="border-t border-border pt-3">
              <label className="mb-1 block text-xs text-fg-faint">Custom Instructions</label>
              <textarea
                value={prefs.hooks?.customInstructions || ""}
                onChange={e => setPrefs(p => ({ ...p, hooks: { ...(p.hooks || {}), customInstructions: e.target.value } }))}
                className="w-full rounded border border-border bg-bg px-3 py-2 text-sm text-fg focus:border-amber/40 focus:outline-none resize-none font-mono"
                rows={4}
                placeholder="Custom behavior rules, coding style preferences, or project-specific context..."
              />
            </div>
          </div>
        </section>

        {/* Actions */}
        <div className="flex gap-3 pb-8">
          <button
            onClick={handleReset}
            className="flex items-center gap-2 rounded border border-border px-4 py-2 text-xs text-fg-dim hover:text-fg hover:border-amber/30 transition-colors"
          >
            <RotateCcw className="h-3.5 w-3.5" />
            Reset
          </button>
          <button
            onClick={handleSave}
            disabled={saving}
            className="flex flex-1 items-center justify-center gap-2 rounded bg-amber px-4 py-2 text-xs font-semibold text-bg hover:bg-amber-light disabled:opacity-40 transition-colors"
          >
            <Save className="h-3.5 w-3.5" />
            {saving ? "Saving…" : saved ? "Saved ✓" : "Save Settings"}
          </button>
        </div>
        {saveError && (
          <div className="pb-4 text-xs text-rose font-mono">{saveError}</div>
        )}
      </div>
    </div>
  );
}

// ── Toggle Switch ────────────────────────────────────────────────────────

function Toggle({ active, onChange }: { active: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      onClick={() => onChange(!active)}
      className={`relative h-5 w-9 shrink-0 rounded-full transition-colors ${
        active ? "bg-amber" : "bg-border"
      }`}
    >
      <span
        className={`absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-bg transition-transform ${
          active ? "translate-x-4" : "translate-x-0"
        }`}
      />
    </button>
  );
}
