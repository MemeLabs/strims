// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ErrorInfo, ReactNode } from "react";

import { errorText } from "./recorder";

interface Props {
  onError: (text: string) => void;
  children: ReactNode;
}

interface State {
  error?: unknown;
  componentStack?: string;
}

/** Shows a story's render error and component stack in its place. */
export default class StoryErrorBoundary extends React.Component<Props, State> {
  state: State = {};

  static getDerivedStateFromError(error: unknown): State {
    return { error };
  }

  componentDidCatch(error: unknown, info: ErrorInfo): void {
    this.setState({ componentStack: info.componentStack });
    this.props.onError(`${errorText(error)}\n\nComponent stack:${info.componentStack ?? ""}`);
  }

  render(): ReactNode {
    if (this.state.error === undefined) {
      return this.props.children;
    }
    return (
      <div className="storybook_crash">
        <h3>The story crashed</h3>
        <pre className="storybook_crash__error">{errorText(this.state.error)}</pre>
        {this.state.componentStack && (
          <pre className="storybook_crash__stack">{this.state.componentStack.trim()}</pre>
        )}
      </div>
    );
  }
}
