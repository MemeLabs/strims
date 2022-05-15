// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

export type Task = () => void;

export default class WorkQueue {
  private busy: boolean = false;
  private tasks: Task[];

  public constructor() {
    this.tasks = [];
  }

  public reset(): void {
    this.tasks = [];
    this.busy = false;
  }

  public insert(task: Task): void {
    if (this.busy) {
      this.tasks.push(task);
    } else {
      this.busy = true;
      task();
    }
  }

  public runNext(): void {
    this.busy = this.tasks.length !== 0;
    if (this.busy) {
      this.tasks.shift()();
    }
  }

  public pause(): void {
    this.busy = true;
  }
}
