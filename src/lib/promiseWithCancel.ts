// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

const promiseWithCancel = (): [Promise<never>, () => void] => {
  let cancel: () => void;
  const promise = new Promise<never>((_, reject) => {
    cancel = reject;
  });
  return [promise, cancel];
};

export default promiseWithCancel;
