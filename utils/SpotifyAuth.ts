import { execAsync } from "ags/process"
import GLib from "gi://GLib"

/**
 * Spotify OAuth Authentication Helper
 * Implements the Implicit Grant Flow for client-side authentication
 */
export class SpotifyAuth {
  private configPath = `${GLib.get_user_config_dir()}/ags/.spotify-config`
  private tokenPath = `${GLib.get_user_config_dir()}/ags/.spotify-token`
  private clientId: string | null = null
  private redirectUri: string = "https://example.com/callback"
  
  constructor() {
    this.loadConfig()
  }

  /**
   * Load configuration from file
   */
  private loadConfig() {
    try {
      // In a real implementation, we'd read the config file
      // For now, we'll use environment variables or manual setup
      console.log("SpotifyAuth: Load your credentials in .spotify-config")
    } catch (error) {
      console.error("SpotifyAuth: Failed to load config:", error)
    }
  }

  /**
   * Generate the Spotify authorization URL
   */
  getAuthUrl(clientId: string): string {
    const scopes = [
      'user-read-playback-state',
      'user-modify-playback-state',
      'user-read-currently-playing',
      'playlist-read-private',
      'playlist-read-collaborative',
      'user-library-read',
      'user-read-recently-played',
      'user-top-read'
    ].join(' ')

    const params = new URLSearchParams({
      client_id: clientId,
      response_type: 'token',
      redirect_uri: this.redirectUri,
      scope: scopes,
      show_dialog: 'true'
    })

    return `https://accounts.spotify.com/authorize?${params.toString()}`
  }

  /**
   * Open the authorization URL in the browser
   */
  async authorize(clientId: string) {
    const authUrl = this.getAuthUrl(clientId)
    console.log(`
🎵 Spotify Authorization Steps:

1. Opening your browser to authorize Spotify...
2. Click "Agree" to authorize the app
3. You'll be redirected to example.com with the token in the URL
4. Copy the entire URL from your browser
5. The URL will look like: https://example.com/callback#access_token=YOUR_TOKEN&...
6. Extract the token between 'access_token=' and '&'
7. Use the setToken() method with your token

Authorization URL: ${authUrl}
    `)
    
    try {
      await execAsync(`xdg-open "${authUrl}"`)
    } catch (error) {
      console.error("Failed to open browser:", error)
      console.log("Please open this URL manually:", authUrl)
    }
  }

  /**
   * Parse the redirect URL to extract the access token
   */
  parseRedirectUrl(url: string): string | null {
    try {
      // Extract the fragment part after #
      const fragment = url.split('#')[1]
      if (!fragment) return null

      // Parse the fragment parameters
      const params = new URLSearchParams(fragment)
      const accessToken = params.get('access_token')
      const expiresIn = params.get('expires_in')

      if (accessToken) {
        console.log(`Token extracted! Expires in ${expiresIn} seconds`)
        return accessToken
      }
    } catch (error) {
      console.error("Failed to parse redirect URL:", error)
    }
    return null
  }

  /**
   * Save token to file
   */
  async saveToken(token: string) {
    try {
      const tokenData = {
        access_token: token,
        timestamp: Date.now(),
        expires_in: 3600 // Default 1 hour
      }
      
      // In production, encrypt this!
      await execAsync(`echo '${JSON.stringify(tokenData)}' > ${this.tokenPath}`)
      console.log("Token saved successfully")
      return true
    } catch (error) {
      console.error("Failed to save token:", error)
      return false
    }
  }

  /**
   * Load saved token
   */
  async loadToken(): Promise<string | null> {
    try {
      const data = await execAsync(`cat ${this.tokenPath}`)
      const tokenData = JSON.parse(data)
      
      // Check if token is expired (simple check)
      const age = Date.now() - tokenData.timestamp
      if (age > tokenData.expires_in * 1000) {
        console.log("Token expired, need to re-authenticate")
        return null
      }
      
      return tokenData.access_token
    } catch (error) {
      console.log("No saved token found")
      return null
    }
  }

  /**
   * Complete authentication flow helper
   */
  async authenticate(clientId: string): Promise<void> {
    console.log(`
╔════════════════════════════════════════════════════════════╗
║           Spotify Web API Authentication Setup             ║
╠════════════════════════════════════════════════════════════╣
║                                                            ║
║  1. First, get your Client ID from:                       ║
║     https://developer.spotify.com/dashboard               ║
║                                                            ║
║  2. Make sure your app has this redirect URI:             ║
║     https://example.com/callback                          ║
║                                                            ║
║  3. Run this authentication flow                          ║
║                                                            ║
╚════════════════════════════════════════════════════════════╝
    `)
    
    await this.authorize(clientId)
  }
}

// Export singleton instance
export const spotifyAuth = new SpotifyAuth()

// Helper function for manual token setting
export function setSpotifyToken(token: string) {
  spotifyAuth.saveToken(token)
  console.log("Token set! Restart AGS to use Spotify features.")
}