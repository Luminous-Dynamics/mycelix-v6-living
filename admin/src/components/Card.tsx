import { ReactNode } from 'react';

interface CardProps {
  title?: string;
  children: ReactNode;
  className?: string;
  action?: ReactNode;
}

export function Card({ title, children, className = '', action }: CardProps) {
  return (
    <div className={`bg-gray-800 rounded-lg border border-gray-700 ${className}`}>
      {(title || action) && (
        <div className="flex items-center justify-between px-4 py-3 border-b border-gray-700">
          {title && <h3 className="text-lg font-medium text-white">{title}</h3>}
          {action}
        </div>
      )}
      <div className="p-4">{children}</div>
    </div>
  );
}

interface StatCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon?: ReactNode;
  trend?: 'up' | 'down' | 'neutral';
  trendValue?: string;
}

export function StatCard({
  title,
  value,
  subtitle,
  icon,
  trend,
  trendValue,
}: StatCardProps) {
  const trendColors = {
    up: 'text-green-400',
    down: 'text-red-400',
    neutral: 'text-gray-400',
  };

  return (
    <div className="bg-gray-800 rounded-lg border border-gray-700 p-4">
      <div className="flex items-start justify-between">
        <div>
          <p className="text-sm text-gray-400">{title}</p>
          <p className="text-2xl font-bold text-white mt-1">{value}</p>
          {subtitle && <p className="text-xs text-gray-500 mt-1">{subtitle}</p>}
          {trend && trendValue && (
            <p className={`text-xs mt-1 ${trendColors[trend]}`}>
              {trend === 'up' ? '\u2191' : trend === 'down' ? '\u2193' : '\u2192'}{' '}
              {trendValue}
            </p>
          )}
        </div>
        {icon && (
          <div className="p-2 bg-gray-700 rounded-lg text-mycelix-400">
            {icon}
          </div>
        )}
      </div>
    </div>
  );
}
