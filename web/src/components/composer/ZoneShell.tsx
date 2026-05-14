// Stub ZoneShell — to be replaced with full zone-aware shell
export default function ZoneShell({ children, zone }: { children: React.ReactNode; zone: string }) {
  return <div className="min-h-screen bg-bg">{children}</div>;
}
