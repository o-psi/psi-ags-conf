import Gtk from "gi://Gtk?version=4.0"
import GLib from "gi://GLib"
import { exec } from "ags/process"

let graphCounter = 0

export function SvgGraph({ 
  history, 
  maxValue = 100, 
  color = "#89b4fa",
  width = 250,
  height = 60,
  strokeWidth = 2
}: { 
  history: number[], 
  maxValue?: number, 
  color?: string,
  width?: number,
  height?: number,
  strokeWidth?: number
}) {
  // Ensure we have some data
  const dataToPlot = history.length > 0 ? history : [0]
  
  // Use the Rust graph renderer
  const tmpPath = `/tmp/ags-graph-${graphCounter++}.svg`
  
  const graphData = {
    data: dataToPlot,
    max_value: maxValue,
    color: color,
    width: width,
    height: height,
    output_path: tmpPath
  }
  
  try {
    // Write JSON to temp file to avoid shell escaping issues
    const jsonPath = `/tmp/ags-graph-${graphCounter}.json`
    GLib.file_set_contents(jsonPath, JSON.stringify(graphData))
    
    // Call the Rust binary with JSON file
    const [success, stdout, stderr] = GLib.spawn_command_line_sync(
      `/home/psi/.config/ags/graph-renderer/target/release/graph-renderer < ${jsonPath}`
    )
    
    if (!success) {
      console.error("Graph renderer failed:", stderr)
    }
    
    // Return the image widget pointing to the generated SVG
    return (
      <image 
        file={tmpPath}
        widthRequest={width}
        heightRequest={height}
        cssClasses={["graph-image"]}
      />
    )
  } catch (error) {
    console.error("Failed to generate graph:", error)
    // Fallback to empty box
    return (
      <box 
        widthRequest={width}
        heightRequest={height}
        cssClasses={["graph-error"]}
      />
    )
  }
}