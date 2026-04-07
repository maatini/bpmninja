import { type LucideIcon } from 'lucide-react';

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description: string;
}

export function EmptyState({ icon: Icon, title, description }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-20 text-center">
      <Icon className="h-16 w-16 text-muted-foreground/30 mb-4" />
      <h3 className="text-lg font-semibold text-muted-foreground">{title}</h3>
      <p className="text-sm text-muted-foreground/70 mt-1 max-w-sm">
        {description}
      </p>
    </div>
  );
}
