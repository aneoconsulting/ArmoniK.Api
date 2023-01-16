export default defineNuxtConfig({
  app: {
    baseURL: process.env.NODE_ENV === "production" ? "/ArmoniK.Api/" : "",
  },

  extends: "@nuxt-themes/docus",

  studio: { enabled: false },

  content: {
    markdown: {
      toc: { depth: 1, searchDepth: 2 },
    },
  },
});
