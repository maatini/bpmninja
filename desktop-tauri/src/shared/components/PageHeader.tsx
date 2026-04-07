import { type ReactNode } from 'react';

interface PageHeaderProps {
  title: string;
  icon?: ReactNode;
  subtitle?: string;
  actions?: ReactNode;
}

export function PageHeader({ title, icon, subtitle, actions }: PageHeaderProps) {
  return (
    <div className="flex items-center justify-between px-6 py-4 border-b bg-background">
      <h2 className="text-2xl font-bold tracking-tight flex items-center gap-2">
        {icon}
        {title}
      </h2>
      <div className="flex items-center gap-4">
        {subtitle && (
          <span className="text-xs text-muted-foreground bg-muted px-2 py-1 rounded-md">
            {subtitle}
          </span>
        )}
        {actions}
      </div>
    </div>
  );
}
