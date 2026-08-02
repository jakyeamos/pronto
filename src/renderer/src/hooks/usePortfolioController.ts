import { useCallback, useEffect, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import * as api from "../api";
import type {
  AnalyticsSnapshot,
  PortfolioSnapshot,
  RemediationActionStatus,
  SkillsSnapshot,
} from "../types";

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

export function usePortfolioController() {
  const [snapshot, setSnapshot] = useState<PortfolioSnapshot>(
    api.emptySnapshot,
  );
  const [analytics, setAnalytics] = useState<AnalyticsSnapshot>(
    api.emptyAnalytics,
  );
  const [skills, setSkills] = useState<SkillsSnapshot>(api.emptySkills);
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
          caught instanceof Error
            ? caught.message
            : "Pronto could not update the portfolio.",
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
      const root = await api.pickRoot();
      if (!root) return;
      await loadSnapshot(() => api.registerRoot(root));
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
    isRefreshing,
    error,
    setError,
    notice,
    setNotice,
    loadSnapshot,
    handleRefreshSkills,
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
