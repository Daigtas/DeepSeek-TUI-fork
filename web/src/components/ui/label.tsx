// Stub Label
export function Label({ children, htmlFor, className }: any) {
  return <label htmlFor={htmlFor} className={`text-xs text-fg-faint ${className || ""}`}>{children}</label>;
}
