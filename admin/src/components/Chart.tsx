import {
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';

interface DataPoint {
  name: string;
  [key: string]: string | number;
}

interface LineChartProps {
  data: DataPoint[];
  lines: {
    key: string;
    color: string;
    name?: string;
  }[];
  height?: number;
}

export function SimpleLineChart({ data, lines, height = 300 }: LineChartProps) {
  return (
    <ResponsiveContainer width="100%" height={height}>
      <LineChart data={data}>
        <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
        <XAxis
          dataKey="name"
          stroke="#9CA3AF"
          fontSize={12}
          tickLine={false}
        />
        <YAxis stroke="#9CA3AF" fontSize={12} tickLine={false} />
        <Tooltip
          contentStyle={{
            backgroundColor: '#1F2937',
            border: '1px solid #374151',
            borderRadius: '8px',
          }}
          labelStyle={{ color: '#F9FAFB' }}
        />
        <Legend />
        {lines.map((line) => (
          <Line
            key={line.key}
            type="monotone"
            dataKey={line.key}
            name={line.name || line.key}
            stroke={line.color}
            strokeWidth={2}
            dot={false}
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  );
}

interface BarChartProps {
  data: DataPoint[];
  bars: {
    key: string;
    color: string;
    name?: string;
  }[];
  height?: number;
}

export function SimpleBarChart({ data, bars, height = 300 }: BarChartProps) {
  return (
    <ResponsiveContainer width="100%" height={height}>
      <BarChart data={data}>
        <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
        <XAxis
          dataKey="name"
          stroke="#9CA3AF"
          fontSize={12}
          tickLine={false}
        />
        <YAxis stroke="#9CA3AF" fontSize={12} tickLine={false} />
        <Tooltip
          contentStyle={{
            backgroundColor: '#1F2937',
            border: '1px solid #374151',
            borderRadius: '8px',
          }}
          labelStyle={{ color: '#F9FAFB' }}
        />
        <Legend />
        {bars.map((bar) => (
          <Bar
            key={bar.key}
            dataKey={bar.key}
            name={bar.name || bar.key}
            fill={bar.color}
            radius={[4, 4, 0, 0]}
          />
        ))}
      </BarChart>
    </ResponsiveContainer>
  );
}
