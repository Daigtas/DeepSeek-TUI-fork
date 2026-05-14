"use client";

import { useState, type FormEvent } from "react";
import { signUp } from "@/lib/auth/client";
import { Mail, Lock, User, UserPlus, AlertCircle } from "lucide-react";
import Link from "next/link";

export default function RegisterPage() {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  function validate(): string | null {
    if (!name.trim()) return "Name is required.";
    if (name.trim().length < 2) return "Name must be at least 2 characters.";
    if (!email.trim()) return "Email is required.";
    if (!password) return "Password is required.";
    if (password.length < 8) return "Password must be at least 8 characters.";
    if (password !== confirmPassword) return "Passwords do not match.";
    return null;
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    const validationError = validate();
    if (validationError) { setError(validationError); return; }
    setLoading(true);
    try {
      const result = await signUp.email({ name: name.trim(), email: email.trim(), password, callbackURL: "/" });
      if (result?.error) setError(result.error.message || "Registration failed.");
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
          <h1 className="text-xl font-bold text-fg">Create account</h1>
          <p className="mt-1 text-sm text-fg-dim">Get started with DeepSeek TUI</p>
        </div>

        {error && (
          <div className="mb-6 flex items-start gap-3 border border-rose/20 bg-rose/5 p-4">
            <AlertCircle className="mt-0.5 h-4 w-4 shrink-0 text-rose" />
            <p className="text-sm text-rose">{error}</p>
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="name" className="mb-1.5 block text-xs text-fg-dim">Name</label>
            <div className="relative">
              <User className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-faint" />
              <input id="name" type="text" autoComplete="name" placeholder="Your name" value={name} onChange={(e) => setName(e.target.value)} className="input-field pl-11" disabled={loading} autoFocus />
            </div>
          </div>

          <div>
            <label htmlFor="email" className="mb-1.5 block text-xs text-fg-dim">Email</label>
            <div className="relative">
              <Mail className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-faint" />
              <input id="email" type="email" autoComplete="email" placeholder="you@example.com" value={email} onChange={(e) => setEmail(e.target.value)} className="input-field pl-11" disabled={loading} />
            </div>
          </div>

          <div>
            <label htmlFor="password" className="mb-1.5 block text-xs text-fg-dim">Password</label>
            <div className="relative">
              <Lock className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-faint" />
              <input id="password" type="password" autoComplete="new-password" placeholder="At least 8 characters" value={password} onChange={(e) => setPassword(e.target.value)} className="input-field pl-11" disabled={loading} />
            </div>
            {password.length > 0 && password.length < 8 && <p className="mt-1 text-xs text-rose">Minimum 8 characters required</p>}
          </div>

          <div>
            <label htmlFor="confirm-password" className="mb-1.5 block text-xs text-fg-dim">Confirm password</label>
            <div className="relative">
              <Lock className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-fg-faint" />
              <input id="confirm-password" type="password" autoComplete="new-password" placeholder="Repeat your password" value={confirmPassword} onChange={(e) => setConfirmPassword(e.target.value)} className={`${confirmPassword && confirmPassword !== password ? "input-field-error" : "input-field"} pl-11`} disabled={loading} />
            </div>
            {confirmPassword && confirmPassword !== password && <p className="mt-1 text-xs text-rose">Passwords do not match</p>}
          </div>

          <button type="submit" disabled={loading} className="btn-primary flex items-center justify-center gap-2">
            {loading ? <span className="h-4 w-4 animate-spin border-2 border-bg/30 border-t-bg" /> : <UserPlus className="h-4 w-4" />}
            {loading ? "Creating account…" : "Create account"}
          </button>
        </form>

        <p className="mt-6 text-center text-xs text-fg-dim">
          Already have an account?{" "}
          <Link href="/login" className="text-amber hover:text-amber-light underline underline-offset-4">Sign in</Link>
        </p>
      </div>
    </div>
  );
}
