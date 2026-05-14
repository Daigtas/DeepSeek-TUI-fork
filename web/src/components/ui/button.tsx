// Stub — to be replaced with shadcn-style component library
export function Button({ children, size, variant, onClick, disabled, className }: any) {
  return (
    <button onClick={onClick} disabled={disabled} className={`inline-flex items-center justify-center rounded border border-border bg-card px-3 py-1.5 text-xs font-medium text-fg hover:border-amber/30 disabled:opacity-40 transition-colors ${className || ""}`}>
      {children}
    </button>
  );
}
