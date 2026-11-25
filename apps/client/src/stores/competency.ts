import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { RoleJobCount, SkillCategory } from "@/lib/bindings";
import { ipc, IpcError, type NormalizedCompetencyMap } from "@/lib/ipc";

export const useCompetencyStore = defineStore("competency", () => {
  const availableRoles = ref<RoleJobCount[]>([]);
  const selectedRole = ref<string | null>(null);
  const mapData = ref<NormalizedCompetencyMap | null>(null);
  const categoryFilter = ref<SkillCategory | "all">("all");
  const gapOnly = ref(false);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const filteredSkills = computed(() => {
    if (!mapData.value) return [];
    return mapData.value.skills.filter((s) => {
      if (categoryFilter.value !== "all" && s.category !== categoryFilter.value) return false;
      if (gapOnly.value && s.is_owned_by_user) return false;
      return true;
    });
  });

  const gapCount = computed(() => {
    if (!mapData.value) return 0;
    return mapData.value.skills.filter((s) => !s.is_owned_by_user).length;
  });

  async function loadRolesAndMap(userId: string) {
    loading.value = true;
    error.value = null;
    try {
      const roles = await ipc.listAvailableRoles();
      availableRoles.value = roles;
      if (roles.length === 0) {
        selectedRole.value = null;
        mapData.value = null;
        return;
      }
      const target =
        selectedRole.value && roles.some((r) => r.normalized_role === selectedRole.value)
          ? selectedRole.value
          : roles[0].normalized_role;
      selectedRole.value = target;
      mapData.value = await ipc.getCompetencyMap(userId, target);
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memuat peta kompetensi.";
    } finally {
      loading.value = false;
    }
  }

  async function selectRole(userId: string, role: string) {
    selectedRole.value = role;
    loading.value = true;
    error.value = null;
    try {
      mapData.value = await ipc.getCompetencyMap(userId, role);
    } catch (e) {
      error.value = e instanceof IpcError ? e.message : "Gagal memuat kompetensi untuk peran ini.";
    } finally {
      loading.value = false;
    }
  }

  return {
    availableRoles,
    selectedRole,
    mapData,
    categoryFilter,
    gapOnly,
    filteredSkills,
    gapCount,
    loading,
    error,
    loadRolesAndMap,
    selectRole,
  };
});
