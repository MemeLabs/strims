export const formatNumberWithScale = (n: number, scale: number, unit: string): string =>
  `${(Math.round(n / (scale / 10)) / 10).toLocaleString()}${unit}`;

export const formatNumber = (n: number) => {
  const scales: [string, number][] = [
    ["B", 1000000000],
    ["M", 1000000],
    ["K", 1000],
  ];
  for (const [unit, scale] of scales) {
    if (n >= scale) {
      return formatNumberWithScale(n, scale, unit);
    }
  }
  return n.toLocaleString();
};
