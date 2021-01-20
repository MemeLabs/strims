export type Task = () => void;

export default class WorkQueue {
  private busy: boolean = false;
  private tasks: Task[];

  public constructor() {
    this.tasks = [];
  }

  public reset() {
    this.tasks = [];
    this.busy = false;
  }

  public insert(task: Task) {
    if (this.busy) {
      this.tasks.push(task);
    } else {
      this.busy = true;
      task();
    }
  }

  public runNext() {
    this.busy = this.tasks.length !== 0;
    if (this.busy) {
      this.tasks.shift()();
    }
  }

  public pause() {
    this.busy = true;
  }
}
