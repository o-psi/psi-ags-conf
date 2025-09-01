import app from "ags/gtk4/app"
import style from "./style.scss"
import Bar from "./widget/Bar"
import Gdk from "gi://Gdk"

app.start({
  css: style,
  instanceName: "ags",
  requestHandler(request: string, res: (response: any) => void) {
    console.log("App: Received request:", request)
    res("ok")
  },
  main() {
    console.log("App: Starting main function")
    // Get all monitors directly
    const display = Gdk.Display.get_default()
    console.log("App: Got display:", display)
    const monitors = display?.get_monitors()
    console.log("App: Got monitors:", monitors)
    
    if (monitors) {
      console.log("App: Number of monitors:", monitors.get_n_items())
      for (let i = 0; i < monitors.get_n_items(); i++) {
        const monitor = monitors.get_item(i) as Gdk.Monitor
        console.log(`App: Creating bar for monitor ${i}:`, monitor.connector)
        try {
          Bar({ gdkmonitor: monitor })
          console.log(`App: Successfully created bar for monitor ${i}`)
        } catch (error) {
          console.error(`App: Error creating bar for monitor ${i}:`, error)
        }
      }
    } else {
      console.log("App: No monitors found")
    }
  },
})