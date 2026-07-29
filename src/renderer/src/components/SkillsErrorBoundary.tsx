import {
  Component,
  type ErrorInfo,
  type ReactElement,
  type ReactNode,
} from "react";
import { RefreshCw } from "lucide-react";

export class SkillsErrorBoundary extends Component<
  { children: ReactNode; onRefresh: () => void },
  { hasError: boolean }
> {
  state = { hasError: false };

  static getDerivedStateFromError(): { hasError: boolean } {
    return { hasError: true };
  }

  componentDidCatch(error: unknown, info: ErrorInfo): void {
    console.error("Skills surface render failed", error, info);
  }

  private recover = (): void => {
    this.setState({ hasError: false });
    this.props.onRefresh();
  };

  render(): ReactElement {
    if (this.state.hasError) {
      return (
        <div className="empty-state skills-error-state" role="alert">
          <p className="eyebrow">Skills view unavailable</p>
          <h2>Pronto could not render this skills snapshot.</h2>
          <p>
            The local index is still private and intact. Refresh skills to
            re-read it and recover the view.
          </p>
          <button
            className="button button-secondary"
            type="button"
            onClick={this.recover}
          >
            <RefreshCw size={15} />
            Refresh skills
          </button>
        </div>
      );
    }
    return <>{this.props.children}</>;
  }
}
