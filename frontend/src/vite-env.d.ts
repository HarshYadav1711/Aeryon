/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_AERYON_API_URL: string
  readonly VITE_AERYON_WS_URL: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
