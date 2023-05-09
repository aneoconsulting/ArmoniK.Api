const baseURL = process.env.NODE_ENV === 'production' ? '/ArmoniK.Api/' : '/'

export default defineNuxtConfig({
  app: {
    baseURL,
    head: {
      link: [
        {
          rel: 'icon',
          type: 'image/ico',
          href: `${baseURL}favicon.ico`
        }
      ]
    }
  },

  extends: '@aneoconsultingfr/armonik-docs-theme',

  runtimeConfig: {
    public: {
      siteName: 'ArmoniK.Api',
      siteDescription: 'API for ArmoniK'
    }
  }
})
