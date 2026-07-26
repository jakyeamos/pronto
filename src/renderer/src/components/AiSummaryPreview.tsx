import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { Eye, Save } from "lucide-react";
import type { AiPayloadPreview } from "../types";
import { formatTime, StatusPill } from "./ConsolePrimitives";

function previewTone(status: string): string {
  if (status === "Payload ready for user inspection") return "mint";
  if (
    status === "AI disabled by repository policy" ||
    status === "No committed changes in selected range"
  ) {
    return "slate";
  }
  return "amber";
}

export function AiSummaryPreview({
  permission,
  onSavePermission,
  onPreview,
}: {
  permission: string;
  onSavePermission: (permission: string) => Promise<void>;
  onPreview: () => Promise<AiPayloadPreview>;
}): ReactElement {
  const [selectedPermission, setSelectedPermission] = useState(permission);
  const [preview, setPreview] = useState<AiPayloadPreview | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [isPreviewing, setIsPreviewing] = useState(false);

  useEffect(() => {
    setSelectedPermission(permission);
    setPreview(null);
  }, [permission]);

  const savePermission = async (): Promise<void> => {
    setIsSaving(true);
    try {
      await onSavePermission(selectedPermission);
    } finally {
      setIsSaving(false);
    }
  };

  const previewPayload = async (): Promise<void> => {
    setIsPreviewing(true);
    try {
      setPreview(await onPreview());
    } finally {
      setIsPreviewing(false);
    }
  };

  return (
    <div className="ai-preview">
      <div className="ai-preview-controls">
        <label className="field-label">
          Repository AI permission
          <select
            className="text-input"
            value={selectedPermission}
            onChange={(event) => setSelectedPermission(event.target.value)}
          >
            <option value="Disabled">Disabled</option>
            <option value="Commit metadata only">Commit metadata only</option>
            <option value="Committed diff allowed">
              Committed diff allowed
            </option>
          </select>
        </label>
        <div className="ai-preview-actions">
          <button
            className="button button-secondary"
            type="button"
            onClick={() => void savePermission()}
            disabled={isSaving || selectedPermission === permission}
          >
            <Save size={14} />
            {isSaving ? "Saving…" : "Save permission"}
          </button>
          <button
            className="button button-primary"
            type="button"
            onClick={() => void previewPayload()}
            disabled={isPreviewing}
          >
            <Eye size={14} />
            {isPreviewing ? "Building preview…" : "Preview payload"}
          </button>
        </div>
      </div>
      <p className="field-help">
        AI is summary-only and disabled by default. This action builds a local
        preview; it never sends a request.
      </p>
      {preview && (
        <div className="ai-preview-result">
          <div className="ai-preview-summary">
            <div>
              <span>Status</span>
              <strong>{preview.status}</strong>
            </div>
            <StatusPill tone={previewTone(preview.status)}>
              {preview.request_performed ? "Request made" : "No request made"}
            </StatusPill>
            <div>
              <span>Payload</span>
              <strong>{preview.payload_bytes.toLocaleString()} bytes</strong>
            </div>
            <div>
              <span>Provider</span>
              <strong>{preview.provider}</strong>
            </div>
          </div>
          {preview.reasons.length > 0 && (
            <ul className="preparation-reasons">
              {preview.reasons.map((reason) => (
                <li key={reason}>{reason}</li>
              ))}
            </ul>
          )}
          <div className="ai-preview-categories">
            {preview.categories.map((category) => (
              <div key={category.category}>
                <span>{category.category}</span>
                <strong>
                  {category.included
                    ? category.byte_count.toLocaleString() + " bytes"
                    : "Excluded by permission"}
                </strong>
                <small>{category.item_count} item(s)</small>
              </div>
            ))}
          </div>
          <div className="ai-preview-references">
            <span>Source references</span>
            {preview.source_references.length === 0 ? (
              <strong>No committed references selected</strong>
            ) : (
              preview.source_references.map((reference) => (
                <small key={reference.sha}>
                  {reference.sha.slice(0, 7)} · {reference.subject} ·{" "}
                  {reference.category} · {formatTime(reference.committed_at)}
                </small>
              ))
            )}
          </div>
          {preview.payload_text && (
            <details className="ai-preview-payload">
              <summary>Inspect exact payload</summary>
              <pre>{preview.payload_text}</pre>
            </details>
          )}
          <small className="ai-preview-generated">
            Local trace generated {formatTime(preview.generated_at)} ·
            uncommitted included: {preview.uncommitted_included ? "yes" : "no"}
          </small>
        </div>
      )}
    </div>
  );
}
