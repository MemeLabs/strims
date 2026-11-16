// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * How near the page's left edge a touch starts that the guard cancels: the
 * outer 34 px of the network nav, or of the swapped drawer's 50 px strip,
 * where Safari's back swipe begins. Neither the band nor whether a cancelled
 * touchstart stops the gesture is measured on a device.
 */
export const BACK_SWIPE_EDGE_PX = 34;

/**
 * How far a touch may move and still be a tap. SwipablePanel's drag opens
 * a panel once it has moved past 10 px.
 */
export const TAP_SLOP_PX = 10;

// Whether the page has an edge back swipe to guard against: an iOS
// browser's tab (Safari's, or another's, all WebKit), on `ios`, not
// `standalone` (a home screen app, which has none). Android has no in-page
// back swipe, nor has a desktop: there the guard would only cost the band's
// scrolling, long presses and text selection.
export const hasEdgeBackSwipe = (ios: boolean, standalone: boolean): boolean => ios && !standalone;

/** A touch, as a TouchEvent's touch lists give it. */
export interface TouchPoint {
  identifier: number;
  clientX: number;
  clientY: number;
}

/**
 * The guard against Safari's back swipe (where hasEdgeBackSwipe), which
 * starts from the screen's left edge, where the network nav sits, or
 * swapped on a phone, the drawer's strip, whose rightward swipe opens the
 * drawer. A touch that starts within BACK_SWIPE_EDGE_PX of the edge is
 * cancelled (its touchstart's default prevented), which also takes the
 * browser's click for a tap there, and the page's scrolling, long press
 * and text selection from it. The guard gives a tap its click back: a touch alone on
 * the screen that ends within TAP_SLOP_PX of where it started and never
 * moved further. A drag gets none, as it gets none from the browser, so a
 * panel's swipe from a button (the drawer's arrow) doesn't end in a click
 * on it, which would toggle the panel back. A touch on a field (an input,
 * or the open drawer's composer, which reaches the edge) is left to the
 * browser, which focuses it and puts its caret where it's tapped, as no
 * click does.
 */
export class BackSwipeGuard {
  // The guarded touch as it started, while it is a tap.
  private tap: TouchPoint | null = null;

  // A touch started (`touch`, one of the `touching` now on the screen, on a
  // field or not): true when its start is to be cancelled. Any touch but a
  // lone guarded one ends the tap.
  start(touch: TouchPoint, touching: number, field = false): boolean {
    const guarded = touch.clientX < BACK_SWIPE_EDGE_PX && !field;
    this.tap = guarded && touching === 1 ? copy(touch) : null;
    return guarded;
  }

  // Touches moved (a touchmove's changedTouches).
  move(touches: ArrayLike<TouchPoint>): void {
    const t = this.find(touches);
    if (t && !this.near(t)) this.tap = null;
  }

  // Touches ended (a touchend's changedTouches): the guarded touch, where
  // it ended a tap, to click at; else null.
  end<T extends TouchPoint>(touches: ArrayLike<T>): T | null {
    const t = this.find(touches);
    if (!t) return null;
    const tapped = this.near(t);
    this.tap = null;
    return tapped ? t : null;
  }

  // The touches were cancelled (a touchcancel): no click.
  cancel(): void {
    this.tap = null;
  }

  private find<T extends TouchPoint>(touches: ArrayLike<T>): T | undefined {
    if (!this.tap) return undefined;
    return Array.from(touches).find((t) => t.identifier === this.tap.identifier);
  }

  private near(t: TouchPoint): boolean {
    return Math.hypot(t.clientX - this.tap.clientX, t.clientY - this.tap.clientY) <= TAP_SLOP_PX;
  }
}

const copy = ({ identifier, clientX, clientY }: TouchPoint): TouchPoint => ({
  identifier,
  clientX,
  clientY,
});
