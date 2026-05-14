"use client";

import { useState, useEffect, useCallback } from "react";
import { useSession } from "@/lib/auth/client";
import { useRouter } from "next/navigation";
import type { UserProfile } from "@/lib/types";
import {
  User, Mail, Calendar, Edit3, Check, X,
  ChevronLeft, Save, Camera,
} from "lucide-react";

export default function ProfilePageContent() {
  const { data: session, isPending } = useSession();
  const router = useRouter();
  const [profile, setProfile] = useState<UserProfile | null>(null);
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState("");
  const [bio, setBio] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (!session?.user) return;
    fetch("/api/profile")
      .then(r => r.json())
      .then(d => {
        if (d.profile) {
          setProfile(d.profile);
          setName(d.profile.name || "");
          setBio(d.profile.bio || "");
        }
      })
      .catch((err) => console.warn("[Profile] failed to load profile:", err));
  }, [session]);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      const res = await fetch("/api/profile", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: name.trim(), bio: bio.trim() }),
      });
      if (!res.ok) {
        const d = await res.json();
        throw new Error(d.error || "Save failed");
      }
      const d = await res.json();
      setProfile(d.profile);
      setEditing(false);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Save failed");
    } finally { setSaving(false); }
  }, [name, bio]);

  const handleCancel = useCallback(() => {
    setName(profile?.name || "");
    setBio(profile?.bio || "");
    setEditing(false);
    setError(null);
  }, [profile]);

  useEffect(() => {
    if (!isPending && !session) {
      router.push("/login");
    }
  }, [isPending, session, router]);

  if (isPending) {
    return <div className="flex h-screen items-center justify-center bg-bg"><div className="animate-pulse text-fg-faint text-sm">Loading…</div></div>;
  }

  if (!session) {
    return <div className="flex h-screen items-center justify-center bg-bg"><div className="animate-pulse text-fg-faint text-sm">Redirecting…</div></div>;
  }

  return (
    <div className="flex min-h-screen flex-col bg-bg text-fg">
      <header className="flex items-center gap-3 border-b border-border px-4 py-3">
        <button onClick={() => router.push("/")} className="text-fg-faint hover:text-fg transition-colors">
          <ChevronLeft className="h-5 w-5" />
        </button>
        <User className="h-4 w-4 text-amber" />
        <h1 className="text-sm font-bold text-amber-light">Profile</h1>
      </header>

      <div className="flex-1 overflow-y-auto px-4 py-6 max-w-lg mx-auto w-full space-y-6">
        <div className="flex flex-col items-center gap-3">
          <div className="relative">
            <div className="flex h-20 w-20 items-center justify-center rounded-full bg-amber/10 border-2 border-amber/30 text-2xl font-bold text-amber">
              {profile?.name?.[0]?.toUpperCase() || "?"}
            </div>
            <button className="absolute -bottom-1 -right-1 flex h-7 w-7 items-center justify-center rounded-full bg-card border border-border text-fg-faint hover:text-amber hover:border-amber/30 transition-colors">
              <Camera className="h-3.5 w-3.5" />
            </button>
          </div>
          {!editing ? (
            <>
              <h2 className="text-lg font-semibold text-fg">{profile?.name}</h2>
              {profile?.bio && <p className="text-sm text-fg-dim text-center max-w-xs">{profile.bio}</p>}
            </>
          ) : null}
        </div>

        {editing ? (
          <div className="space-y-4 rounded border border-border bg-card p-4">
            {error && <div className="border border-rose/20 bg-rose/5 px-3 py-2 text-xs text-rose">{error}</div>}
            <div>
              <label className="mb-1 block text-xs text-fg-faint">Name</label>
              <input value={name} onChange={e => setName(e.target.value)} className="w-full rounded border border-border bg-bg px-3 py-2 text-sm text-fg focus:border-amber/40 focus:outline-none" placeholder="Your name" maxLength={100} />
            </div>
            <div>
              <label className="mb-1 block text-xs text-fg-faint">Bio</label>
              <textarea value={bio} onChange={e => setBio(e.target.value)} className="w-full rounded border border-border bg-bg px-3 py-2 text-sm text-fg focus:border-amber/40 focus:outline-none resize-none" rows={3} placeholder="Tell us about yourself…" maxLength={500} />
              <p className="mt-1 text-right text-[10px] text-fg-faint">{bio.length}/500</p>
            </div>
            <div className="flex gap-2">
              <button onClick={handleCancel} className="flex items-center gap-1.5 rounded border border-border px-3 py-1.5 text-xs text-fg-dim hover:text-fg transition-colors">
                <X className="h-3.5 w-3.5" />Cancel
              </button>
              <button onClick={handleSave} disabled={saving || !name.trim()} className="flex flex-1 items-center justify-center gap-1.5 rounded bg-amber px-3 py-1.5 text-xs font-semibold text-bg hover:bg-amber-light disabled:opacity-40 transition-colors">
                <Save className="h-3.5 w-3.5" />{saving ? "Saving…" : "Save"}
              </button>
            </div>
          </div>
        ) : (
          <div className="space-y-3 rounded border border-border bg-card p-4">
            <div className="flex items-center gap-3">
              <Mail className="h-3.5 w-3.5 shrink-0 text-fg-faint" />
              <span className="text-sm text-fg">{profile?.email}</span>
            </div>
            <div className="flex items-center gap-3">
              <Calendar className="h-3.5 w-3.5 shrink-0 text-fg-faint" />
              <span className="text-xs text-fg-faint">
                Joined {profile?.createdAt ? new Date(profile.createdAt).toLocaleDateString() : "—"}
              </span>
            </div>
            <div className="pt-2">
              <button onClick={() => setEditing(true)} className="flex items-center gap-1.5 rounded border border-border px-3 py-1.5 text-xs text-fg-dim hover:text-amber hover:border-amber/30 transition-colors">
                <Edit3 className="h-3.5 w-3.5" />Edit Profile
              </button>
            </div>
          </div>
        )}

        {saved && (
          <div className="flex items-center justify-center gap-1.5 text-xs text-green">
            <Check className="h-3.5 w-3.5" />Profile updated
          </div>
        )}
      </div>
    </div>
  );
}
