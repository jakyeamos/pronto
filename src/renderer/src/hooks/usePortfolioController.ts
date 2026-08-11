import { useCallback, useEffect, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import * as api from "../api";
import type {
  AnalyticsSnapshot,
  CreatePapercutInput,
  PapercutBacklog,
  PapercutStatus,
  MultiplierProposalStatus,
  PortfolioSnapshot,
  PromotionDecision,
  PromotionInbox,
  RemediationActionStatus,
  SkillsSnapshot,
} from "../types";

type LoadSnapshot = (
  operation: () => Promise<PortfolioSnapshot>,
) => Promise<void>;

function messageFromCaught(caught: unknown, fallback: string): string {
  if (caught instanceof Error && caught.message) return caught.message;
  if (typeof caught === "string" && caught.trim()) return caught;
  if (
    typeof caught === "object" &&
    caught !== null &&
    "message" in caught &&
    typeof caught.message === "string" &&
    caught.message.trim()
  ) {
    return caught.message;
  }
  return fallback;
}

async function loadAnalytics(
  setAnalytics: Dispatch<SetStateAction<AnalyticsSnapshot>>,
  setError: Dispatch<SetStateAction<string | null>>,
): Promise<void> {
  try {
    setAnalytics(await api.getAnalytics());
  } catch (caught) {
    setAnalytics(api.emptyAnalytics);
    setError(
      caught instanceof Error
        ? caught.message
        : "Pronto could not load local analytics history.",
    );
  }
}

async function registerPickedRoot(loadSnapshot: LoadSnapshot): Promise<void> {
  const root = await api.pickRoot();
  if (!root) return;
  await loadSnapshot(() => api.registerRoot(root));
}

export function usePortfolioController() {
  const [snapshot, setSnapshot] = useState<PortfolioSnapshot>(
    api.emptySnapshot,
  );
  const [analytics, setAnalytics] = useState<AnalyticsSnapshot>(
    api.emptyAnalytics,
  );
  const [skills, setSkills] = useState<SkillsSnapshot>(api.emptySkills);
  const [promotionInbox, setPromotionInbox] = useState<PromotionInbox>(
    api.emptyPromotionInbox,
  );
  const [papercutBacklog, setPapercutBacklog] = useState<PapercutBacklog>(
    api.emptyPapercutBacklog,
  );
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const loadSnapshot = useCallback(
    async (operation: () => Promise<PortfolioSnapshot>): Promise<void> => {
      setIsRefreshing(true);
      setError(null);
      setNotice(null);
      try {
        setSnapshot(await operation());
        await loadAnalytics(setAnalytics, setError);
      } catch (caught) {
        setError(
          messageFromCaught(caught, "Pronto could not update the portfolio."),
        );
      } finally {
        setIsRefreshing(false);
      }
    },
    [],
  );

  useEffect(() => {
    void loadSnapshot(api.getSnapshot);
    void api
      .getSkills()
      .then(setSkills)
      .catch(() => setSkills(api.emptySkills));
    void api
      .getPromotionInbox()
      .then(setPromotionInbox)
      .catch(() => setPromotionInbox(api.emptyPromotionInbox));
    void api
      .getPapercutBacklog()
      .then(setPapercutBacklog)
      .catch(() => setPapercutBacklog(api.emptyPapercutBacklog));
  }, [loadSnapshot]);

  const handleRefreshSkills = useCallback(async (): Promise<void> => {
    setIsRefreshing(true);
    setError(null);
    try {
      setSkills(await api.refreshSkills());
    } catch (caught) {
      setError(
        caught instanceof Error
          ? caught.message
          : "Pronto could not refresh the skills corpus.",
      );
    } finally {
      setIsRefreshing(false);
    }
  }, []);

  const handleRefreshPromotionInbox = useCallback(async (): Promise<void> => {
    setIsRefreshing(true);
    setError(null);
    setNotice(null);
    try {
      setPromotionInbox(await api.refreshPromotionInbox());
    } catch (caught) {
      setError(
        messageFromCaught(
          caught,
          "Pronto could not refresh the promotion inbox.",
        ),
      );
    } finally {
      setIsRefreshing(false);
    }
  }, []);

  const handlePromotionDecision = useCallback(
    async (
      candidateId: string,
      decision: PromotionDecision,
      reason?: string,
    ): Promise<void> => {
      setIsRefreshing(true);
      setError(null);
      setNotice(null);
      try {
        const updated = await api.decidePromotion(
          candidateId,
          decision,
          reason,
        );
        setPromotionInbox(updated);
        const admission = updated.jas_admission;
        if (
          decision === "public" ||
          decision === "private" ||
          decision === "both"
        ) {
          if (
            admission?.status === "JAS_APPLIED" ||
            admission?.status === "JAS_ALREADY_APPLIED"
          ) {
            setNotice(
              admission.receipt_status === "blocked"
                ? "Decision recorded and JAS changed, but AWL could not persist the admission receipt."
                : admission.status === "JAS_ALREADY_APPLIED"
                  ? "Decision recorded. JAS was already in the requested state."
                  : "Decision recorded and JAS admission/install completed.",
            );
          } else {
            setNotice(
              `Decision recorded in AWL; JAS apply is blocked: ${
                admission?.message ??
                admission?.reason ??
                "the candidate needs a valid JAS projection"
              }`,
            );
          }
        } else {
          setNotice("Decision recorded in AWL.");
        }
      } catch (caught) {
        setError(
          messageFromCaught(
            caught,
            "Pronto could not record the promotion decision.",
          ),
        );
      } finally {
        setIsRefreshing(false);
      }
    },
    [],
  );

  const handleRefreshPapercutBacklog = useCallback(async (): Promise<void> => {
    setIsRefreshing(true);
    setError(null);
    setNotice(null);
    try {
      setPapercutBacklog(await api.refreshPapercutBacklog());
    } catch (caught) {
      setError(
        messageFromCaught(
          caught,
          "Pronto could not refresh the papercut backlog.",
        ),
      );
    } finally {
      setIsRefreshing(false);
    }
  }, []);

  const handleCreatePapercut = useCallback(
    async (input: CreatePapercutInput): Promise<void> => {
      setIsRefreshing(true);
      setError(null);
      setNotice(null);
      try {
        setPapercutBacklog(await api.createPapercut(input));
        setNotice("Papercut captured in the design audit backlog.");
      } catch (caught) {
        setError(
          messageFromCaught(caught, "Pronto could not capture that papercut."),
        );
      } finally {
        setIsRefreshing(false);
      }
    },
    [],
  );

  const handlePapercutStatus = useCallback(
    async (papercutId: string, status: PapercutStatus): Promise<void> => {
      setIsRefreshing(true);
      setError(null);
      setNotice(null);
      try {
        setPapercutBacklog(await api.setPapercutStatus(papercutId, status));
        setNotice("Papercut status updated.");
      } catch (caught) {
        setError(
          messageFromCaught(caught, "Pronto could not update that papercut."),
        );
      } finally {
        setIsRefreshing(false);
      }
    },
    [],
  );

  const handleMultiplierProposalStatus = useCallback(
    async (
      proposalId: string,
      status: MultiplierProposalStatus,
    ): Promise<void> => {
      setIsRefreshing(true);
      setError(null);
      setNotice(null);
      try {
        await api.setMultiplierProposalStatus(proposalId, status);
        setPapercutBacklog(await api.refreshPapercutBacklog());
        setNotice(
          "Multiplier proposal review recorded. No implementation was started.",
        );
      } catch (caught) {
        setError(
          messageFromCaught(
            caught,
            "Pronto could not update that multiplier proposal.",
          ),
        );
      } finally {
        setIsRefreshing(false);
      }
    },
    [],
  );

  const handleOpenSkillSource = useCallback(
    async (path: string): Promise<void> => {
      try {
        await api.openSkillSource(path);
      } catch (caught) {
        setError(
          caught instanceof Error
            ? caught.message
            : "Pronto could not open that skill source.",
        );
      }
    },
    [],
  );

  const handleAddRoot = useCallback(async (): Promise<void> => {
    try {
      await registerPickedRoot(loadSnapshot);
    } catch (caught) {
      setError(
        caught instanceof Error
          ? caught.message
          : "Pronto could not register that root.",
      );
    }
  }, [loadSnapshot]);

  const handleSaveRoot = useCallback(
    async (
      rootId: string,
      ignorePatterns: string[],
      refreshPolicy: string,
      backgroundMonitoring: boolean,
    ): Promise<void> => {
      await loadSnapshot(() =>
        api.updateRootSettings(
          rootId,
          ignorePatterns,
          refreshPolicy,
          backgroundMonitoring,
        ),
      );
    },
    [loadSnapshot],
  );

  const handleSaveRetention = useCallback(
    async (retentionDays: number): Promise<void> => {
      await loadSnapshot(() => api.setRetentionDays(retentionDays));
    },
    [loadSnapshot],
  );

  const handleSaveProduct = useCallback(
    async (
      productId: string | null,
      name: string,
      repositoryIds: string[],
      releaseMode: string,
    ): Promise<void> => {
      await loadSnapshot(() =>
        api.upsertProduct(productId, name, repositoryIds, releaseMode),
      );
    },
    [loadSnapshot],
  );

  const handleDeleteProduct = useCallback(
    async (productId: string): Promise<void> => {
      await loadSnapshot(() => api.deleteProduct(productId));
    },
    [loadSnapshot],
  );

  const handleSaveGroup = useCallback(
    async (
      groupId: string | null,
      name: string,
      repositoryIds: string[],
    ): Promise<void> => {
      await loadSnapshot(() => api.upsertGroup(groupId, name, repositoryIds));
    },
    [loadSnapshot],
  );

  const handleDeleteGroup = useCallback(
    async (groupId: string): Promise<void> => {
      await loadSnapshot(() => api.deleteGroup(groupId));
    },
    [loadSnapshot],
  );

  const handleRefreshGithub = useCallback(async (): Promise<void> => {
    await loadSnapshot(api.refreshGithub);
  }, [loadSnapshot]);

  const handleRefreshRemediation = useCallback(async (): Promise<void> => {
    await loadSnapshot(api.refreshRemediation);
  }, [loadSnapshot]);

  const handleExportRemediation = useCallback(async (): Promise<void> => {
    setError(null);
    try {
      const result = await api.exportRemediation();
      setNotice(`Remediation plans exported to ${result.output_path}.`);
    } catch (caught) {
      setError(
        caught instanceof Error
          ? caught.message
          : "Pronto could not export the remediation plans.",
      );
    }
  }, []);

  const handleRemediationStatus = useCallback(
    async (
      actionId: string,
      status: RemediationActionStatus,
    ): Promise<void> => {
      await loadSnapshot(() =>
        api.setRemediationActionStatus(actionId, status),
      );
    },
    [loadSnapshot],
  );

  const handleConfirmRefresh = useCallback(async (): Promise<void> => {
    await loadSnapshot(api.refresh);
  }, [loadSnapshot]);

  const handleOpenQualityReport = useCallback(
    async (reportPath: string): Promise<void> => {
      await loadSnapshot(() => api.openQualityReport(reportPath));
    },
    [loadSnapshot],
  );

  return {
    snapshot,
    setSnapshot,
    analytics,
    skills,
    promotionInbox,
    papercutBacklog,
    isRefreshing,
    error,
    setError,
    notice,
    setNotice,
    loadSnapshot,
    handleRefreshSkills,
    handleRefreshPromotionInbox,
    handlePromotionDecision,
    handleRefreshPapercutBacklog,
    handleCreatePapercut,
    handlePapercutStatus,
    handleMultiplierProposalStatus,
    handleOpenSkillSource,
    handleAddRoot,
    handleSaveRoot,
    handleSaveRetention,
    handleSaveProduct,
    handleDeleteProduct,
    handleSaveGroup,
    handleDeleteGroup,
    handleRefreshGithub,
    handleRefreshRemediation,
    handleExportRemediation,
    handleRemediationStatus,
    handleConfirmRefresh,
    handleOpenQualityReport,
  };
}
