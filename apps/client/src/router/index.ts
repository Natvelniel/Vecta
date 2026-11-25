import { createRouter, createWebHistory } from "vue-router";
import DashboardView from "@/views/DashboardView.vue";
import OnboardingStoryView from "@/views/OnboardingStoryView.vue";
import JobsFeedView from "@/views/JobsFeedView.vue";
import CompetencyMapView from "@/views/CompetencyMapView.vue";
import CvStudioView from "@/views/CvStudioView.vue";
import RoadmapView from "@/views/RoadmapView.vue";
import SettingsView from "@/views/SettingsView.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/dashboard" },
    { path: "/onboarding", name: "onboarding", component: OnboardingStoryView },
    { path: "/dashboard", name: "dashboard", component: DashboardView },
    { path: "/jobs", name: "jobs", component: JobsFeedView },
    { path: "/cv", name: "cv", component: CvStudioView },
    { path: "/competency", name: "competency", component: CompetencyMapView },
    { path: "/roadmap", name: "roadmap", component: RoadmapView },
    { path: "/settings", name: "settings", component: SettingsView },
    { path: "/:pathMatch(.*)*", redirect: "/dashboard" },
  ],
});
