// Stub — to be replaced with shadcn-style component library

export function Card({ children, className, onClick }: any) {
  return <div onClick={onClick} className={`rounded border border-border bg-card ${className || ""}`}>{children}</div>;
}
export function CardContent({ children, className }: any) {
  return <div className={`p-4 ${className || ""}`}>{children}</div>;
}
export function CardHeader({ children, className }: any) {
  return <div className={`border-b border-border px-4 py-3 ${className || ""}`}>{children}</div>;
}
export function CardTitle({ children, className }: any) {
  return <h3 className={`text-sm font-semibold text-fg ${className || ""}`}>{children}</h3>;
}
