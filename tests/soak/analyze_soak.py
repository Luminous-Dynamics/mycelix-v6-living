#!/usr/bin/env python3
"""
Soak Test Analysis Tool for Mycelix WebSocket Server

Analyzes soak test results and generates reports with visualizations.

Usage:
    python analyze_soak.py soak_results/metrics_*.csv
    python analyze_soak.py --output report.html soak_results/metrics_*.csv
"""

import argparse
import csv
import json
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import List, Optional, Tuple


@dataclass
class MetricSample:
    """A single metric sample from the soak test."""
    timestamp: str
    elapsed_seconds: int
    rss_kb: int
    rss_mb: int
    vsz_kb: int
    open_fds: int
    threads: int
    cpu_percent: float
    active_connections: int
    messages_received: int


@dataclass
class AnalysisResult:
    """Results of soak test analysis."""
    duration_seconds: int
    sample_count: int

    # Memory analysis
    memory_min_mb: int
    memory_max_mb: int
    memory_avg_mb: float
    memory_std_mb: float
    memory_growth_kb: int
    memory_growth_percent: float
    memory_trend_slope: float  # KB per hour

    # File descriptor analysis
    fd_initial: int
    fd_final: int
    fd_max: int
    fd_growth: int

    # Thread analysis
    thread_min: int
    thread_max: int
    thread_avg: float

    # Connection analysis
    connection_max: int
    messages_total: int

    # Leak detection
    potential_memory_leak: bool
    potential_fd_leak: bool
    leak_confidence: str  # "low", "medium", "high"

    # Overall result
    passed: bool
    issues: List[str]


def parse_csv(filepath: str) -> List[MetricSample]:
    """Parse a metrics CSV file."""
    samples = []

    with open(filepath, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            try:
                sample = MetricSample(
                    timestamp=row['timestamp'],
                    elapsed_seconds=int(row['elapsed_seconds']),
                    rss_kb=int(row['rss_kb']),
                    rss_mb=int(row['rss_mb']),
                    vsz_kb=int(row['vsz_kb']),
                    open_fds=int(row['open_fds']),
                    threads=int(row['threads']),
                    cpu_percent=float(row.get('cpu_percent', 0)),
                    active_connections=int(row.get('active_connections', 0)),
                    messages_received=int(row.get('messages_received', 0)),
                )
                samples.append(sample)
            except (KeyError, ValueError) as e:
                print(f"Warning: Skipping malformed row: {e}", file=sys.stderr)

    return samples


def calculate_linear_regression(x: List[float], y: List[float]) -> Tuple[float, float]:
    """Calculate simple linear regression, returns (slope, intercept)."""
    n = len(x)
    if n == 0:
        return 0.0, 0.0

    sum_x = sum(x)
    sum_y = sum(y)
    sum_xy = sum(xi * yi for xi, yi in zip(x, y))
    sum_xx = sum(xi * xi for xi in x)

    denom = n * sum_xx - sum_x * sum_x
    if denom == 0:
        return 0.0, sum_y / n if n > 0 else 0.0

    slope = (n * sum_xy - sum_x * sum_y) / denom
    intercept = (sum_y - slope * sum_x) / n

    return slope, intercept


def analyze_samples(samples: List[MetricSample]) -> AnalysisResult:
    """Analyze soak test samples for issues."""
    if not samples:
        raise ValueError("No samples to analyze")

    duration = samples[-1].elapsed_seconds if samples else 0

    # Memory analysis
    rss_values = [s.rss_kb for s in samples]
    memory_min = min(rss_values)
    memory_max = max(rss_values)
    memory_avg = sum(rss_values) / len(rss_values)
    memory_std = (sum((x - memory_avg) ** 2 for x in rss_values) / len(rss_values)) ** 0.5

    # Calculate memory growth trend
    elapsed = [s.elapsed_seconds for s in samples]
    slope, _ = calculate_linear_regression(elapsed, rss_values)
    memory_trend_slope = slope * 3600  # Convert to KB per hour

    # First and last 10% comparison
    n = len(samples)
    first_n = max(1, n // 10)
    last_n = max(1, n // 10)

    first_avg = sum(s.rss_kb for s in samples[:first_n]) / first_n
    last_avg = sum(s.rss_kb for s in samples[-last_n:]) / last_n
    memory_growth = int(last_avg - first_avg)
    memory_growth_percent = (memory_growth / first_avg * 100) if first_avg > 0 else 0

    # File descriptor analysis
    fd_values = [s.open_fds for s in samples]
    fd_initial = samples[0].open_fds
    fd_final = samples[-1].open_fds
    fd_max = max(fd_values)
    fd_growth = fd_final - fd_initial

    # Thread analysis
    thread_values = [s.threads for s in samples]
    thread_min = min(thread_values)
    thread_max = max(thread_values)
    thread_avg = sum(thread_values) / len(thread_values)

    # Connection analysis
    connection_max = max(s.active_connections for s in samples)
    messages_total = samples[-1].messages_received

    # Leak detection
    issues = []

    # Memory leak detection
    potential_memory_leak = False
    if memory_growth_percent > 10:
        potential_memory_leak = True
        issues.append(f"Memory grew by {memory_growth_percent:.1f}% during the test")

    if memory_trend_slope > 100:  # More than 100KB/hour growth
        if not potential_memory_leak:
            potential_memory_leak = True
        issues.append(f"Memory trend shows {memory_trend_slope:.1f} KB/hour growth")

    # FD leak detection
    potential_fd_leak = False
    if fd_growth > 10 and fd_growth > fd_initial * 0.1:
        potential_fd_leak = True
        issues.append(f"File descriptors grew by {fd_growth} during the test")

    # Determine confidence
    if potential_memory_leak and memory_growth_percent > 50:
        leak_confidence = "high"
    elif potential_memory_leak or potential_fd_leak:
        leak_confidence = "medium"
    else:
        leak_confidence = "low"

    # Overall result
    passed = not potential_memory_leak and not potential_fd_leak

    return AnalysisResult(
        duration_seconds=duration,
        sample_count=len(samples),
        memory_min_mb=memory_min // 1024,
        memory_max_mb=memory_max // 1024,
        memory_avg_mb=memory_avg / 1024,
        memory_std_mb=memory_std / 1024,
        memory_growth_kb=memory_growth,
        memory_growth_percent=memory_growth_percent,
        memory_trend_slope=memory_trend_slope,
        fd_initial=fd_initial,
        fd_final=fd_final,
        fd_max=fd_max,
        fd_growth=fd_growth,
        thread_min=thread_min,
        thread_max=thread_max,
        thread_avg=thread_avg,
        connection_max=connection_max,
        messages_total=messages_total,
        potential_memory_leak=potential_memory_leak,
        potential_fd_leak=potential_fd_leak,
        leak_confidence=leak_confidence,
        passed=passed,
        issues=issues,
    )


def generate_text_report(result: AnalysisResult) -> str:
    """Generate a text report of the analysis."""
    hours = result.duration_seconds / 3600

    lines = [
        "=" * 60,
        "Soak Test Analysis Report",
        "=" * 60,
        "",
        f"Duration: {hours:.2f} hours ({result.sample_count} samples)",
        "",
        "Memory (RSS):",
        f"  Min:          {result.memory_min_mb} MB",
        f"  Max:          {result.memory_max_mb} MB",
        f"  Average:      {result.memory_avg_mb:.1f} MB",
        f"  Std Dev:      {result.memory_std_mb:.1f} MB",
        f"  Growth:       {result.memory_growth_kb} KB ({result.memory_growth_percent:.1f}%)",
        f"  Trend:        {result.memory_trend_slope:.1f} KB/hour",
        "",
        "File Descriptors:",
        f"  Initial:      {result.fd_initial}",
        f"  Final:        {result.fd_final}",
        f"  Max:          {result.fd_max}",
        f"  Growth:       {result.fd_growth}",
        "",
        "Threads:",
        f"  Min:          {result.thread_min}",
        f"  Max:          {result.thread_max}",
        f"  Average:      {result.thread_avg:.1f}",
        "",
        "Connections:",
        f"  Max Active:   {result.connection_max}",
        f"  Total Msgs:   {result.messages_total}",
        "",
        "-" * 60,
        "Leak Detection:",
        f"  Memory Leak:  {'DETECTED' if result.potential_memory_leak else 'None detected'}",
        f"  FD Leak:      {'DETECTED' if result.potential_fd_leak else 'None detected'}",
        f"  Confidence:   {result.leak_confidence}",
        "",
    ]

    if result.issues:
        lines.append("Issues Found:")
        for issue in result.issues:
            lines.append(f"  - {issue}")
        lines.append("")

    lines.extend([
        "-" * 60,
        f"Result: {'PASSED' if result.passed else 'FAILED'}",
        "=" * 60,
    ])

    return "\n".join(lines)


def generate_html_report(result: AnalysisResult, samples: List[MetricSample]) -> str:
    """Generate an HTML report with charts."""
    hours = result.duration_seconds / 3600

    # Prepare data for charts
    timestamps = [s.elapsed_seconds / 3600 for s in samples]
    memory_data = [s.rss_mb for s in samples]
    fd_data = [s.open_fds for s in samples]
    thread_data = [s.threads for s in samples]

    html = f"""<!DOCTYPE html>
<html>
<head>
    <title>Soak Test Report</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: #333; color: white; padding: 20px; margin-bottom: 20px; }}
        .metrics {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; }}
        .metric-card {{ background: #f5f5f5; padding: 15px; border-radius: 5px; }}
        .metric-value {{ font-size: 24px; font-weight: bold; }}
        .metric-label {{ color: #666; }}
        .chart {{ margin: 20px 0; }}
        .passed {{ color: green; }}
        .failed {{ color: red; }}
        .warning {{ color: orange; }}
        .issues {{ background: #fff3cd; padding: 15px; border-radius: 5px; margin: 20px 0; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Soak Test Analysis Report</h1>
        <p>Duration: {hours:.2f} hours | Samples: {result.sample_count}</p>
        <h2 class="{'passed' if result.passed else 'failed'}">
            {'PASSED' if result.passed else 'FAILED'}
        </h2>
    </div>

    <div class="metrics">
        <div class="metric-card">
            <div class="metric-value">{result.memory_avg_mb:.1f} MB</div>
            <div class="metric-label">Average Memory</div>
        </div>
        <div class="metric-card">
            <div class="metric-value">{result.memory_max_mb} MB</div>
            <div class="metric-label">Peak Memory</div>
        </div>
        <div class="metric-card">
            <div class="metric-value">{result.memory_growth_percent:.1f}%</div>
            <div class="metric-label">Memory Growth</div>
        </div>
        <div class="metric-card">
            <div class="metric-value">{result.fd_final}</div>
            <div class="metric-label">Final FDs</div>
        </div>
        <div class="metric-card">
            <div class="metric-value">{result.connection_max}</div>
            <div class="metric-label">Max Connections</div>
        </div>
        <div class="metric-card">
            <div class="metric-value">{result.messages_total:,}</div>
            <div class="metric-label">Total Messages</div>
        </div>
    </div>

    {"".join(f'<div class="issues"><strong>Issue:</strong> {issue}</div>' for issue in result.issues) if result.issues else ''}

    <div class="chart" id="memory-chart"></div>
    <div class="chart" id="fd-chart"></div>
    <div class="chart" id="thread-chart"></div>

    <script>
        // Memory chart
        Plotly.newPlot('memory-chart', [{{
            x: {timestamps},
            y: {memory_data},
            type: 'scatter',
            name: 'RSS Memory (MB)',
            line: {{ color: 'blue' }}
        }}], {{
            title: 'Memory Usage Over Time',
            xaxis: {{ title: 'Time (hours)' }},
            yaxis: {{ title: 'Memory (MB)' }}
        }});

        // FD chart
        Plotly.newPlot('fd-chart', [{{
            x: {timestamps},
            y: {fd_data},
            type: 'scatter',
            name: 'Open FDs',
            line: {{ color: 'green' }}
        }}], {{
            title: 'File Descriptors Over Time',
            xaxis: {{ title: 'Time (hours)' }},
            yaxis: {{ title: 'Open FDs' }}
        }});

        // Thread chart
        Plotly.newPlot('thread-chart', [{{
            x: {timestamps},
            y: {thread_data},
            type: 'scatter',
            name: 'Threads',
            line: {{ color: 'orange' }}
        }}], {{
            title: 'Thread Count Over Time',
            xaxis: {{ title: 'Time (hours)' }},
            yaxis: {{ title: 'Threads' }}
        }});
    </script>
</body>
</html>
"""
    return html


def main():
    parser = argparse.ArgumentParser(description="Analyze soak test results")
    parser.add_argument("input", help="Input CSV file with metrics")
    parser.add_argument("--output", "-o", help="Output file (HTML if .html, otherwise text)")
    parser.add_argument("--json", "-j", action="store_true", help="Output JSON summary")
    args = parser.parse_args()

    # Parse samples
    samples = parse_csv(args.input)
    if not samples:
        print("Error: No valid samples found in input file", file=sys.stderr)
        sys.exit(1)

    # Analyze
    result = analyze_samples(samples)

    # Output
    if args.json:
        output = json.dumps({
            "duration_seconds": result.duration_seconds,
            "sample_count": result.sample_count,
            "memory": {
                "min_mb": result.memory_min_mb,
                "max_mb": result.memory_max_mb,
                "avg_mb": result.memory_avg_mb,
                "growth_percent": result.memory_growth_percent,
                "trend_kb_per_hour": result.memory_trend_slope,
            },
            "file_descriptors": {
                "initial": result.fd_initial,
                "final": result.fd_final,
                "growth": result.fd_growth,
            },
            "leak_detection": {
                "memory_leak": result.potential_memory_leak,
                "fd_leak": result.potential_fd_leak,
                "confidence": result.leak_confidence,
            },
            "passed": result.passed,
            "issues": result.issues,
        }, indent=2)
    elif args.output and args.output.endswith('.html'):
        output = generate_html_report(result, samples)
    else:
        output = generate_text_report(result)

    if args.output:
        Path(args.output).write_text(output)
        print(f"Report written to {args.output}")
    else:
        print(output)

    # Exit with appropriate code
    sys.exit(0 if result.passed else 1)


if __name__ == "__main__":
    main()
