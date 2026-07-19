import './App.css'
import { Dashboard } from './components/Dashboard'
import { useDashboard } from './hooks/useDashboard'

function App() {
  const state = useDashboard()
  return <Dashboard {...state} />
}

export default App
