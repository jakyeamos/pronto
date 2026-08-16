import { AlertTriangle, ShieldCheck, X } from "lucide-react";
import type { ReactElement } from "react";

export function FeedbackBanners({
  error,
  notice,
  onDismissError,
  onDismissNotice,
}: {
  error: string | null;
  notice: string | null;
  onDismissError: () => void;
  onDismissNotice: () => void;
}): ReactElement {
  return (
    <>
      {error && (
        <div className="error-banner" role="alert">
          <AlertTriangle size={16} />
          <span>{error}</span>
          <button
            type="button"
            onClick={onDismissError}
            aria-label="Dismiss error"
          >
            <X size={14} />
          </button>
        </div>
      )}
      {notice && (
        <div className="success-banner" role="status">
          <ShieldCheck size={16} />
          <span>{notice}</span>
          <button
            type="button"
            onClick={onDismissNotice}
            aria-label="Dismiss notice"
          >
            <X size={14} />
          </button>
        </div>
      )}
    </>
  );
}
