declare module "date-and-time" {
  const date: {
    format(dateObj: Date, formatString: string | string[], utc?: boolean): string;
  };
  export default date;
}
