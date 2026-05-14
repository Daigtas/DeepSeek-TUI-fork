// Stub Textarea
export function Textarea({ value, onChange, placeholder, className, rows }: any) {
  return (
    <textarea
      value={value}
      onChange={onChange}
      placeholder={placeholder}
      rows={rows}
      className={`w-full rounded border border-border bg-bg px-3 py-2 text-sm text-fg focus:border-amber/40 focus:outline-none resize-none ${className || ""}`}
    />
  );
}
