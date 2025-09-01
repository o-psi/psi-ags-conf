import Gtk from "gi://Gtk?version=4.0"
import GLib from "gi://GLib"
import { createPoll } from "ags/time"
import { readFile } from "ags/file"

interface NetInfo {
  download: number
  upload: number
}

function readSharedStats() {
  try {
    const latest = readFile("/tmp/ags-stats/latest.json")
    return JSON.parse(latest)
  } catch {
    return null
  }
}

export function CpuWidget() {
  const cpuInfo = createPoll<number>(
    0,
    1000,
    () => {
      const shared = readSharedStats()
      if (shared) {
        return shared.cpu_usage
      }
      return 0
    }
  )
  
  const openGraphWindow = (event: any) => {
    const [x, y] = event.get_root_coords ? event.get_root_coords() : [0, 0]
    
    const config = {
      title: "CPU Usage",
      color: "#89dceb",
      max_value: 100,
      width: 400,
      height: 150,
      data_source: "cpu",
      position_x: Math.floor(x),
      position_y: Math.floor(y),
      multi_chart: false
    }
    try {
      const jsonStr = JSON.stringify(config).replace(/'/g, "\'")
      const cmd = `/home/psi/.config/ags/graph-window/target/release/graph-window '${jsonStr}'`
      GLib.spawn_command_line_async(cmd)
    } catch (error) {
      console.error("Failed to spawn graph window:", error)
    }
  }

  const openMultiCoreWindow = (event: any) => {
    const [x, y] = event.get_root_coords ? event.get_root_coords() : [0, 0]
    
    const config = {
      title: "CPU Cores & IO Wait",
      color: "#89dceb",
      max_value: 100,
      width: 600,
      height: 400,
      data_source: "cpu",
      position_x: Math.floor(x),
      position_y: Math.floor(y),
      multi_chart: true
    }
    try {
      const jsonStr = JSON.stringify(config).replace(/'/g, "\'")
      const cmd = `/home/psi/.config/ags/graph-window/target/release/graph-window '${jsonStr}'`
      GLib.spawn_command_line_async(cmd)
    } catch (error) {
      console.error("Failed to spawn multi-core graph window:", error)
    }
  }
  
  return (
    <box spacing={2}>
      <button 
        cssClasses={["cpu-widget"]}
        onClicked={openGraphWindow}
      >
        <box spacing={4}>
          <label label="󰻠" cssClasses={["icon"]} />
          <label label={cpuInfo((usage) => `${usage.toFixed(0)}%`)} />
        </box>
      </button>
      <button 
        cssClasses={["cpu-cores-widget"]}
        onClicked={openMultiCoreWindow}
      >
        <label label="󰘚" cssClasses={["icon"]} />
      </button>
    </box>
  )
}

export function MemoryWidget() {
  const memInfo = createPoll<number>(
    0,
    1000,
    () => {
      const shared = readSharedStats()
      if (shared && shared.memory) {
        return shared.memory.used_percentage
      }
      return 0
    }
  )
  
  const openGraphWindow = (event: any) => {
    const [x, y] = event.get_root_coords ? event.get_root_coords() : [0, 0]
    
    const config = {
      title: "Advanced Memory Usage",
      color: "#cba6f7",
      max_value: 100,
      width: 500,
      height: 250,
      data_source: "memory",
      advanced: true,
      position_x: Math.floor(x),
      position_y: Math.floor(y)
    }
    try {
      const jsonStr = JSON.stringify(config).replace(/'/g, "\'")
      const cmd = `/home/psi/.config/ags/graph-window/target/release/graph-window '${jsonStr}'`
      GLib.spawn_command_line_async(cmd)
    } catch (error) {
      console.error("Failed to spawn graph window:", error)
    }
  }
  
  return (
    <button 
      cssClasses={["memory-widget"]}
      onClicked={openGraphWindow}
    >
      <box spacing={4}>
        <label label="󰍛" cssClasses={["icon"]} />
        <label label={memInfo((percentage) => `${percentage.toFixed(0)}%`)} />
      </box>
    </button>
  )
}

export function NetworkWidget() {
  const netInfo = createPoll<NetInfo>(
    { download: 0, upload: 0 },
    1000,
    () => {
      const shared = readSharedStats()
      if (shared) {
        return {
          download: shared.network_download,
          upload: shared.network_upload
        }
      }
      return { download: 0, upload: 0 }
    }
  )
  
  function formatSpeed(kbps: number): string {
    if (kbps > 1024) {
      return `${(kbps / 1024).toFixed(1)} MB/s`
    }
    return `${kbps.toFixed(0)} KB/s`
  }
  
  const openGraphWindow = (event: any) => {
    const [x, y] = event.get_root_coords ? event.get_root_coords() : [0, 0]
    
    const config = {
      title: "Network Activity",
      color: "#89dceb",
      color2: "#f38ba8",
      max_value: 1024, // Default max value
      width: 400,
      height: 150,
      data_source: "network",
      position_x: Math.floor(x),
      position_y: Math.floor(y)
    }
    try {
      const jsonStr = JSON.stringify(config).replace(/'/g, "\'")
      const cmd = `/home/psi/.config/ags/graph-window/target/release/graph-window '${jsonStr}'`
      GLib.spawn_command_line_async(cmd)
    } catch (error) {
      console.error("Failed to spawn graph window:", error)
    }
  }
  
  return (
    <button
      cssClasses={["network-widget"]}
      onClicked={openGraphWindow}
    >
      <box spacing={8}>
        <box spacing={2}>
          <label label="󰇚" cssClasses={["icon"]} />
          <label label={netInfo((info) => formatSpeed(info.download))} />
        </box>
        <box spacing={2}>
          <label label="󰕒" cssClasses={["icon"]} />
          <label label={netInfo((info) => formatSpeed(info.upload))} />
        </box>
      </box>
    </button>
  )
}

export function SystemStatsWidget() {
  return (
    <box cssClasses={["system-stats-widget"]} spacing={12}>
      <CpuWidget />
      <MemoryWidget />
      <NetworkWidget />
    </box>
  )
}
