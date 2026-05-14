// Stub molecules — to be replaced with shadcn-style component library
import { Package } from "lucide-react";

export function PageHeader({ icon: Icon, title, subtitle }: { icon?: any; title: string; subtitle: string }) {
  return (
    <div className="text-center">
      {Icon && <Icon className="h-8 w-8 mx-auto mb-2 text-amber" />}
      <h1 className="text-xl font-bold text-fg">{title}</h1>
      <p className="mt-1 text-sm text-fg-dim">{subtitle}</p>
    </div>
  );
}
