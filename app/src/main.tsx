import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'

// `npm run dev` in a plain browser: sample data instead of the Rust backend.
if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
  const { installDevMock } = await import('./devmock')
  installDevMock()
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
