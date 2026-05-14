"use client";

import { useState, type FormEvent } from "react";
import { signIn } from "@/lib/auth/client";
import { Mail, Lock, LogIn, AlertCircle } from "lucide-react";
import Link from "next/link";

export default function LoginPage() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    if (!email.trim() || !password) { setError("Please fill in both fields."); return; }
    setLoading(true);
    try {
      const result = await signIn.email({ email: email.trim(), password, callbackURL: "/" });
      if (result?.error) setError(result.error.message || "Invalid credentials.");
    } catch {
      setError("Network error. Please try again.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-bg px-4">
      <div className="card w-full max-w-md p-8 animate-fade-in">
        <div className="mb-8 text-center">
          <span className="inline-block text-2xl font-mono text-amber mb-2">❯_</span>
          <h1 className="text-xl font-bold text-fg">Welcome back</h1>
          <p className="mt-1 text-sm text-fg-dim">Sign in to your account</p>
        </div>

        {error && (
          <div className="mb-6 flex items-start gap-3 border border-rose/20 bg-rose/5 p-4">
            <AlertCircle className="mt-0.5 h-4 w-4 shrink-0 text-rose" />
            <p className="text-sm text-rose">{error}</p>
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="email" className="mb-1.5 block text-xs text-fg-dim">Email</label>
            <div className="relative">
              <Mail className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-faint" />
              <input id="email" type="email" autoComplete="email" placeholder="you@example.com" value={email} onChange={(e) => setEmail(e.target.value)} className="input-field pl-11" disabled={loading} autoFocus />
            </div>
          </div>

          <div>
            <label htmlFor="password" className="mb-1.5 block text-xs text-fg-dim">Password</label>
            <div className="relative">
              <Lock className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-faint" />
              <input id="password" type="password" autoComplete="current-password" placeholder="Enter your password" value={password} onChange={(e) => setPassword(e.target.value)} className="input-field pl-11" disabled={loading} />
            </div>
          </div>

          <button type="submit" disabled={loading} className="btn-primary flex items-center justify-center gap-2">
            {loading ? (
              <span className="h-4 w-4 animate-spin border-2 border-bg/30 border-t-bg" />
            ) : (
              <LogIn className="h-4 w-4" />
            )}
            {loading ? "Signing in…" : "Sign in"}
          </button>
        </form>

        <p className="mt-6 text-center text-xs text-fg-dim">
          Don&apos;t have an account?{" "}
          <Link href="/register" className="text-amber hover:text-amber-light underline underline-offset-4">
            Create account
          </Link>
        </p>
      </div>
    </div>
  );
}
