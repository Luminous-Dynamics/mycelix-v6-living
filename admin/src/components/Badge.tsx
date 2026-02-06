import { ReactNode } from 'react';

type BadgeVariant = 'default' | 'success' | 'warning' | 'error' | 'info';

interface BadgeProps {
  children: ReactNode;
  variant?: BadgeVariant;
  className?: string;
}

const variantClasses: Record<BadgeVariant, string> = {
  default: 'bg-gray-700 text-gray-300',
  success: 'bg-green-900/50 text-green-400 border border-green-700',
  warning: 'bg-yellow-900/50 text-yellow-400 border border-yellow-700',
  error: 'bg-red-900/50 text-red-400 border border-red-700',
  info: 'bg-blue-900/50 text-blue-400 border border-blue-700',
};

export function Badge({ children, variant = 'default', className = '' }: BadgeProps) {
  return (
    <span
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${variantClasses[variant]} ${className}`}
    >
      {children}
    </span>
  );
}

// Phase-specific badge component
import { CyclePhase } from '../api/client';

const phaseColors: Record<CyclePhase, { bg: string; text: string; border: string }> = {
  [CyclePhase.Shadow]: { bg: 'bg-purple-900/50', text: 'text-purple-300', border: 'border-purple-700' },
  [CyclePhase.Composting]: { bg: 'bg-amber-900/50', text: 'text-amber-300', border: 'border-amber-700' },
  [CyclePhase.Liminal]: { bg: 'bg-indigo-900/50', text: 'text-indigo-300', border: 'border-indigo-700' },
  [CyclePhase.NegativeCapability]: { bg: 'bg-slate-900/50', text: 'text-slate-300', border: 'border-slate-700' },
  [CyclePhase.Eros]: { bg: 'bg-pink-900/50', text: 'text-pink-300', border: 'border-pink-700' },
  [CyclePhase.CoCreation]: { bg: 'bg-emerald-900/50', text: 'text-emerald-300', border: 'border-emerald-700' },
  [CyclePhase.Beauty]: { bg: 'bg-rose-900/50', text: 'text-rose-300', border: 'border-rose-700' },
  [CyclePhase.EmergentPersonhood]: { bg: 'bg-cyan-900/50', text: 'text-cyan-300', border: 'border-cyan-700' },
  [CyclePhase.Kenosis]: { bg: 'bg-gray-900/50', text: 'text-gray-300', border: 'border-gray-600' },
};

interface PhaseBadgeProps {
  phase: CyclePhase;
  className?: string;
}

export function PhaseBadge({ phase, className = '' }: PhaseBadgeProps) {
  const colors = phaseColors[phase];
  return (
    <span
      className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${colors.bg} ${colors.text} ${colors.border} ${className}`}
    >
      {phase}
    </span>
  );
}
