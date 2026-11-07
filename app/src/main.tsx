import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'

// `npm run dev` in a plain browser: sample data instead of the Rust backend.
if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
  const { installDevMock } = await import('./devmock')
  installDevMock()
}

// Focus rings only while the keyboard is in use: WebKitGTK also shows them after a click.
const root = document.documentElement
window.addEventListener('keydown', (e) => {
  if (e.key === 'Tab' || e.key.startsWith('Arrow')) root.dataset.keys = ''
}, true)
window.addEventListener('pointerdown', () => delete root.dataset.keys, true)

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
