// Stub Select
export function Select({ value, onChange, children, className }: any) {
  return (
    <select
      value={value}
      onChange={onChange}
      className={`rounded border border-border bg-bg px-3 py-2 text-sm text-fg focus:border-amber/40 focus:outline-none ${className || ""}`}
    >
      {children}
    </select>
  );
}
export function SelectTrigger({ children, className }: any) {
  return <div className={className}>{children}</div>;
}
export function SelectValue({ placeholder }: any) {
  return <span>{placeholder}</span>;
}
export function SelectContent({ children }: any) {
  return <>{children}</>;
}
export function SelectItem({ value, children }: any) {
  return <option value={value}>{children}</option>;
}
