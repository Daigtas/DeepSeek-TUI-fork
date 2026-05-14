// Stub Input
export function Input({ value, onChange, placeholder, type, className }: any) {
  return (
    <input
      type={type || "text"}
      value={value}
      onChange={onChange}
      placeholder={placeholder}
      className={`w-full rounded border border-border bg-bg px-3 py-2 text-sm text-fg focus:border-amber/40 focus:outline-none ${className || ""}`}
    />
  );
}
