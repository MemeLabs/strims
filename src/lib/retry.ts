export const retrySync = (fn: () => boolean, delay: number, attempts: number) =>
  setTimeout(() => {
    if (!fn() && attempts > 0) retrySync(fn, delay * 2, attempts - 1);
  }, delay);
