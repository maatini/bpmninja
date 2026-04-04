import { useState, useEffect } from 'react'
import { deployDefinition, startInstance } from './lib/tauri'
import { Modeler } from './Modeler'
import { Instances } from './Instances'
import { DeployedProcesses } from './DeployedProcesses'
import { Settings } from './Settings'
import { Monitoring } from './Monitoring'
import { PendingTasks } from './PendingTasks'
import { MessageDialog } from './MessageDialog'
import { PenTool, Database, ListTodo, Layers, BarChart2, Settings as SettingsIcon, Mail, AlertTriangle } from 'lucide-react'
import { useToast } from './ToastContext'
import { IncidentsView } from './IncidentsView'

function App() {
  const toast = useToast()
  useEffect(() => {
    const saved = localStorage.getItem('theme')
    if (saved && saved !== 'auto') {
      document.documentElement.setAttribute('data-theme', saved)
    }
  }, [])

  const [activeTab, setActiveTab] = useState('definitions')
  const [selectedInstanceId, setSelectedInstanceId] = useState<string | null>(null)
  const [viewXml, setViewXml] = useState<string | null>(null)
  const [showMessageDialog, setShowMessageDialog] = useState(false)


  const handleDeploy = async (xml: string) => {
    try {
      const id = await deployDefinition(xml, 'modeler-process')
      toast.success("Deployed! Definition: " + id.substring(0, 8))
    } catch (e) {
      toast.error("Deploy failed: " + e)
    }
  }

  const handleStart = async (xml: string, variables: Record<string, unknown>): Promise<string> => {
    // Auto-deploy the current modeler state
    const newDefId = await deployDefinition(xml, 'modeler-process')

    // Start instance with the freshly deployed definition
    const id = await startInstance(newDefId, variables)

    // Navigate to the new instance
    setSelectedInstanceId(id)
    setActiveTab('instances')

    return id
  }

  // Called when user clicks "View in Modeler" on a deployed definition
  const handleViewDefinition = (xml: string) => {
    setViewXml(xml)
    setActiveTab('modeler')
  }

  // Called when user clicks "New Diagram" in the Modeler
  const handleNewDiagram = () => {
    setViewXml(null)
  }

  // Called when the user opens a local BPMN file via "Open File".
  const handleOpenFile = () => {
    setViewXml(null)
  }

  return (
    <div className="app-container">
      <div className="sidebar">
        <div className="sidebar-header">Mini BPM</div>
        <div className={`nav-item ${activeTab === 'modeler' ? 'active' : ''}`} onClick={() => setActiveTab('modeler')} style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <PenTool size={18} /> BPMN Modeler
        </div>
        <div className={`nav-item ${activeTab === 'definitions' ? 'active' : ''}`} onClick={() => setActiveTab('definitions')} style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Database size={18} /> Deployed Processes
        </div>
        <div className={`nav-item ${activeTab === 'tasks' ? 'active' : ''}`} onClick={() => setActiveTab('tasks')} style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <ListTodo size={18} /> Pending Tasks
        </div>
        <div className={`nav-item ${activeTab === 'incidents' ? 'active' : ''}`} onClick={() => setActiveTab('incidents')} style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <AlertTriangle size={18} /> Incidents
        </div>
        <div className={`nav-item ${activeTab === 'instances' ? 'active' : ''}`} onClick={() => { setSelectedInstanceId(null); setActiveTab('instances'); }} style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Layers size={18} /> Instances
        </div>
        <div className={`nav-item ${activeTab === 'monitoring' ? 'active' : ''}`} onClick={() => setActiveTab('monitoring')} style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <BarChart2 size={18} /> Monitoring
        </div>
        <div className={`nav-item ${activeTab === 'settings' ? 'active' : ''}`} onClick={() => setActiveTab('settings')} style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <SettingsIcon size={18} /> Settings
        </div>

        <div className="nav-item" onClick={() => setShowMessageDialog(true)} style={{ marginTop: 'auto', display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Mail size={18} /> Send Message
        </div>

        <div className="sidebar-footer">
          <span className="backend-badge backend-nats">
            ● Thin Client
          </span>
        </div>
      </div>
      
      <div className="main-content">
        {activeTab === 'modeler' && (
          <Modeler onDeploy={handleDeploy} onStart={handleStart} onNewDiagram={handleNewDiagram} onOpenFile={handleOpenFile} initialXml={viewXml} />
        )}

        {activeTab === 'definitions' && (
          <DeployedProcesses 
            onView={handleViewDefinition} 
            onViewInstance={(id) => { setSelectedInstanceId(id); setActiveTab('instances'); }}
          />
        )}

        {activeTab === 'tasks' && (
          <PendingTasks />
        )}

        {activeTab === 'incidents' && (
          <IncidentsView onViewInstance={(id) => { setSelectedInstanceId(id); setActiveTab('instances'); }} />
        )}

        {activeTab === 'instances' && (
          <Instances 
            selectedInstanceId={selectedInstanceId} 
            onClearSelection={() => setSelectedInstanceId(null)} 
          />
        )}

        {activeTab === 'monitoring' && (
          <Monitoring />
        )}

        {activeTab === 'settings' && (
          <Settings />
        )}
      </div>
      <MessageDialog open={showMessageDialog} onClose={() => setShowMessageDialog(false)} />
    </div>
  )
}

export default App

