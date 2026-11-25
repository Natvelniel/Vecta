import { defineStore } from "pinia";
import { ref } from "vue";
import type {
  RoadmapDetail_Serialize,
  StepStatus,
  StepUpdateResult_Serialize,
} from "@/lib/bindings";
import { ipc, IpcError, type NormalizedRoadmapSummary } from "@/lib/ipc";

export const useRoadmapStore = defineStore("roadmap", () => {
  const roadmaps = ref<NormalizedRoadmapSummary[]>([]);
  const activeRoadmapId = ref<string | null>(null);
  const activeDetail = ref<RoadmapDetail_Serialize | null>(null);
  const loading = ref(false);
  const generating = ref(false);
  const error = ref<string | null>(null);

  async function loadRoadmaps(userId: string) {
    loading.value = true;
    error.value = null;
    try {
      const list = await ipc.listRoadmaps(userId);
      roadmaps.value = list;
      if (list.length === 0) {
        activeRoadmapId.value = null;
        activeDetail.value = null;
        return;
      }
      const targetId =
        activeRoadmapId.value && list.some((r) => r.id === activeRoadmapId.value)
          ? activeRoadmapId.value
          : list[0].id;
      await selectRoadmap(targetId);
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memuat daftar roadmap.";
    } finally {
      loading.value = false;
    }
  }

  async function selectRoadmap(roadmapId: string) {
    activeRoadmapId.value = roadmapId;
    loading.value = true;
    try {
      activeDetail.value = await ipc.getRoadmapDetail(roadmapId);
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memuat detail roadmap.";
    } finally {
      loading.value = false;
    }
  }

  async function generateRoadmap(
    userId: string,
    targetRole: string,
    sourceJobId: string | null = null,
    hoursPerWeek = 8,
  ): Promise<boolean> {
    generating.value = true;
    error.value = null;
    try {
      const detail = await ipc.generateLearningRoadmap(
        userId,
        targetRole,
        sourceJobId,
        hoursPerWeek,
      );
      activeRoadmapId.value = detail.id;
      activeDetail.value = detail;
      roadmaps.value = await ipc.listRoadmaps(userId);
      return true;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal menyusun roadmap belajar AI.";
      return false;
    } finally {
      generating.value = false;
    }
  }

  async function setStepStatus(
    userId: string,
    stepId: string,
    nextStatus: StepStatus,
  ): Promise<StepUpdateResult_Serialize | null> {
    try {
      const result = await ipc.updateRoadmapStepStatus(stepId, nextStatus);
      if (activeDetail.value) {
        const step = activeDetail.value.steps.find((s) => s.id === stepId);
        if (step) {
          step.status = result.status;
          step.completed_at = result.completed_at;
        }
      }
      roadmaps.value = await ipc.listRoadmaps(userId);
      return result;
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memperbarui progres langkah.";
      return null;
    }
  }

  async function toggleStepDone(
    userId: string,
    stepId: string,
    currentStatus: StepStatus,
  ): Promise<StepUpdateResult_Serialize | null> {
    const nextStatus: StepStatus = currentStatus === "done" ? "todo" : "done";
    return setStepStatus(userId, stepId, nextStatus);
  }

  return {
    roadmaps,
    activeRoadmapId,
    activeDetail,
    loading,
    generating,
    error,
    loadRoadmaps,
    selectRoadmap,
    generateRoadmap,
    setStepStatus,
    toggleStepDone,
  };
});
