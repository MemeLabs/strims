// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

class Layout {
  private maxWidth = 0;
  private margin = 0;
  private widths: number[] = [];
  private rows: number[] = [];
  private rowWidths: number[] = [];

  clone() {
    const l = new Layout();
    l.maxWidth = this.maxWidth;
    l.margin = this.margin;
    l.widths = this.widths;
    l.rows = this.rows.slice();
    l.rowWidths = this.rowWidths.slice();
    return l;
  }

  error(): [number[], number] {
    const meanWidth = this.rowWidths.reduce((s, w) => s + w, 0) / this.rowWidths.length;
    const errs = this.rowWidths.map((w) => w - meanWidth);
    const rmse = Math.sqrt(errs.reduce((s, e) => s + e * e, 0));
    return [errs, rmse];
  }

  size() {
    return this.rows.length;
  }

  rotateLeft(i: number) {
    const w = this.widths[this.rows[i - 1]];
    this.rowWidths[i] -= w;
    this.rowWidths[i - 1] += w;
    this.rows[i - 1]++;
  }

  rotateRight(i: number) {
    const w = this.widths[this.rows[i] - 1];
    this.rowWidths[i] -= w;
    if (i + 1 === this.rows.length) {
      this.rows.push(this.rows[i + 1]);
      this.rowWidths.push(-this.margin);
    }
    this.rowWidths[i + 1] += w;
    this.rows[i]--;
  }

  bubbleUp(i: number) {
    this.rotateLeft(i);
    for (let j = i - 1; j > 1 && this.rowWidths[j] > this.maxWidth; j--) {
      this.rotateLeft(j);
    }
  }

  bubbleDown(i: number) {
    this.rotateRight(i);
    for (let j = i + 1; j < this.rows.length && this.rowWidths[j] > this.maxWidth; j++) {
      this.rotateRight(j);
    }
  }

  private static create(widths: number[], maxWidth: number, margin: number) {
    const l = new Layout();
    l.maxWidth = maxWidth;
    l.margin = margin;
    l.widths = widths.map((w) => w + margin);

    l.rows.push(0);
    l.rowWidths.push(-margin);
    for (let i = 0, row = 0; i < l.widths.length; i++) {
      if (l.rowWidths[row] + l.widths[i] > maxWidth) {
        l.rows.push(i);
        l.rowWidths.push(-margin);
        row++;
      }
      l.rows[row] = i + 1;
      l.rowWidths[row] += l.widths[i];
    }
    return l;
  }

  private static getErrRankedIndex(errs: number[], rank: number) {
    const indices: number[] = [];
    for (let i = 0; i < errs.length; i++) indices.push(i);
    indices.sort((a, b) => Math.abs(errs[b]) - Math.abs(errs[a]));
    return indices[rank];
  }

  static solve(this: void, widths: number[], maxWidth: number, margin = 0) {
    let cur = Layout.create(widths, maxWidth, margin);
    let next = cur.clone();

    let [errs, err] = cur.error();
    for (let rank = 0; rank < cur.size(); ) {
      const i = Layout.getErrRankedIndex(errs, rank);
      if (errs[i] < 0) {
        if (i + 1 < cur.size() && (i === 0 || errs[i - 1] < errs[i + 1])) {
          next.bubbleUp(i + 1);
        } else {
          next.bubbleDown(i - 1);
        }
      } else {
        if (i + 1 === cur.size() || (i > 0 && errs[i - 1] < errs[i + 1])) {
          next.bubbleUp(i);
        } else {
          next.bubbleDown(i);
        }
      }

      const [nextErrs, nextErr] = next.error();
      if (nextErr < err) {
        cur = next.clone();
        errs = nextErrs;
        err = nextErr;
        rank = 0;
      } else {
        next = cur.clone();
        rank++;
      }
    }

    return cur.rows;
  }
}

export default Layout.solve;
