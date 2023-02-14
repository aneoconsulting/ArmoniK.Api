export default defineNuxtConfig({
  app: {
    baseURL: process.env.NODE_ENV === "production" ? "/ArmoniK.Api/" : "",
  },

  extends: "@aneoconsultingfr/armonik-docs-theme",
});
