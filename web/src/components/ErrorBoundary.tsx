"use client";

import { Component, type ReactNode } from "react";
import { AlertTriangle, RefreshCw } from "lucide-react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback;

      return (
        <div className="flex min-h-screen items-center justify-center bg-bg px-4">
          <div className="card max-w-md p-8 text-center animate-fade-in">
            <AlertTriangle className="mx-auto h-8 w-8 text-rose mb-3" />
            <h2 className="text-lg font-bold text-fg mb-2">Something went wrong</h2>
            <p className="text-sm text-fg-dim mb-1">
              {this.state.error?.message || "An unexpected error occurred"}
            </p>
            {this.state.error?.message && this.state.error.message.length > 100 && (
              <details className="mt-2 text-left">
                <summary className="text-xs text-fg-faint cursor-pointer hover:text-amber">Stack trace</summary>
                <pre className="mt-1 max-h-32 overflow-y-auto text-[10px] text-fg-faint bg-alt border border-border p-2 whitespace-pre-wrap">
                  {this.state.error?.stack || "No stack trace available"}
                </pre>
              </details>
            )}
            <button
              onClick={this.handleReset}
              className="mt-4 inline-flex items-center gap-2 rounded bg-amber px-4 py-2 text-xs font-semibold text-bg hover:bg-amber-light transition-colors"
            >
              <RefreshCw className="h-3.5 w-3.5" />
              Try again
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
