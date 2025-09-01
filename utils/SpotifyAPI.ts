import { execAsync } from "ags/process"

export interface SpotifyTrack {
  id: string
  name: string
  artists: { name: string }[]
  album: {
    name: string
    images: { url: string }[]
  }
  duration_ms: number
  uri: string
}

export interface SpotifyPlaylist {
  id: string
  name: string
  description: string
  images: { url: string }[]
  tracks: { total: number }
  uri: string
}

export interface SpotifyAPIResponse<T> {
  success: boolean
  data?: T
  error?: string
}

/**
 * Spotify Web API wrapper using curl commands
 * Note: Requires Spotify Premium account and proper authentication setup
 */
export class SpotifyAPI {
  private accessToken: string | null = null
  private refreshToken: string | null = null
  private clientId: string | null = null
  private clientSecret: string | null = null

  constructor() {
    this.loadTokensFromStorage()
  }

  /**
   * Load stored authentication tokens
   */
  private loadTokensFromStorage() {
    try {
      // Try to load from a simple config file
      // In a real implementation, this would be more secure
      const configPath = `${process.env.HOME}/.config/ags/.spotify-auth`
      // For now, we'll implement without persistent storage
      console.log("SpotifyAPI: No stored tokens found, manual auth required")
    } catch (error) {
      console.log("SpotifyAPI: No stored authentication found")
    }
  }

  /**
   * Check if we have a valid access token
   */
  isAuthenticated(): boolean {
    return this.accessToken !== null
  }

  /**
   * Make authenticated API request using curl
   */
  private async makeRequest(endpoint: string, method: 'GET' | 'POST' | 'PUT' | 'DELETE' = 'GET', body?: any): Promise<any> {
    if (!this.accessToken) {
      throw new Error('Not authenticated with Spotify API')
    }

    const url = `https://api.spotify.com/v1${endpoint}`
    const headers = [
      `Authorization: Bearer ${this.accessToken}`,
      'Content-Type: application/json'
    ]

    let curlCommand = `curl -s -X ${method}`
    headers.forEach(header => {
      curlCommand += ` -H "${header}"`
    })

    if (body && (method === 'POST' || method === 'PUT')) {
      curlCommand += ` -d '${JSON.stringify(body)}'`
    }

    curlCommand += ` "${url}"`

    try {
      const response = await execAsync(curlCommand)
      return JSON.parse(response)
    } catch (error) {
      console.error('SpotifyAPI: Request failed:', error)
      throw error
    }
  }

  /**
   * Get current user's playlists
   */
  async getUserPlaylists(): Promise<SpotifyAPIResponse<SpotifyPlaylist[]>> {
    try {
      const response = await this.makeRequest('/me/playlists?limit=50')
      return {
        success: true,
        data: response.items
      }
    } catch (error) {
      return {
        success: false,
        error: `Failed to fetch playlists: ${error}`
      }
    }
  }

  /**
   * Search for tracks, albums, artists, playlists
   */
  async search(query: string, types: string[] = ['track'], limit: number = 20): Promise<SpotifyAPIResponse<any>> {
    try {
      const encodedQuery = encodeURIComponent(query)
      const typeParam = types.join(',')
      const response = await this.makeRequest(`/search?q=${encodedQuery}&type=${typeParam}&limit=${limit}`)
      return {
        success: true,
        data: response
      }
    } catch (error) {
      return {
        success: false,
        error: `Search failed: ${error}`
      }
    }
  }

  /**
   * Get the user's current playback queue
   */
  async getQueue(): Promise<SpotifyAPIResponse<{ currently_playing: SpotifyTrack, queue: SpotifyTrack[] }>> {
    try {
      const response = await this.makeRequest('/me/player/queue')
      return {
        success: true,
        data: response
      }
    } catch (error) {
      return {
        success: false,
        error: `Failed to fetch queue: ${error}`
      }
    }
  }

  /**
   * Add a track to the queue
   */
  async addToQueue(uri: string): Promise<SpotifyAPIResponse<void>> {
    try {
      await this.makeRequest(`/me/player/queue?uri=${encodeURIComponent(uri)}`, 'POST')
      return { success: true }
    } catch (error) {
      return {
        success: false,
        error: `Failed to add to queue: ${error}`
      }
    }
  }

  /**
   * Get user's saved tracks (liked songs)
   */
  async getSavedTracks(limit: number = 20, offset: number = 0): Promise<SpotifyAPIResponse<SpotifyTrack[]>> {
    try {
      const response = await this.makeRequest(`/me/tracks?limit=${limit}&offset=${offset}`)
      return {
        success: true,
        data: response.items.map((item: any) => item.track)
      }
    } catch (error) {
      return {
        success: false,
        error: `Failed to fetch saved tracks: ${error}`
      }
    }
  }

  /**
   * Get tracks from a playlist
   */
  async getPlaylistTracks(playlistId: string, limit: number = 50, offset: number = 0): Promise<SpotifyAPIResponse<SpotifyTrack[]>> {
    try {
      const response = await this.makeRequest(`/playlists/${playlistId}/tracks?limit=${limit}&offset=${offset}`)
      return {
        success: true,
        data: response.items.map((item: any) => item.track)
      }
    } catch (error) {
      return {
        success: false,
        error: `Failed to fetch playlist tracks: ${error}`
      }
    }
  }

  /**
   * Play a specific track or playlist
   */
  async play(contextUri?: string, trackUris?: string[], position?: number): Promise<SpotifyAPIResponse<void>> {
    try {
      const body: any = {}
      
      if (contextUri) {
        body.context_uri = contextUri
      }
      
      if (trackUris) {
        body.uris = trackUris
      }
      
      if (position !== undefined) {
        body.offset = { position }
      }

      await this.makeRequest('/me/player/play', 'PUT', Object.keys(body).length > 0 ? body : undefined)
      return { success: true }
    } catch (error) {
      return {
        success: false,
        error: `Failed to play: ${error}`
      }
    }
  }

  /**
   * Skip to next track in queue
   */
  async skipNext(): Promise<SpotifyAPIResponse<void>> {
    try {
      await this.makeRequest('/me/player/next', 'POST')
      return { success: true }
    } catch (error) {
      return {
        success: false,
        error: `Failed to skip: ${error}`
      }
    }
  }

  /**
   * Skip to previous track
   */
  async skipPrevious(): Promise<SpotifyAPIResponse<void>> {
    try {
      await this.makeRequest('/me/player/previous', 'POST')
      return { success: true }
    } catch (error) {
      return {
        success: false,
        error: `Failed to skip previous: ${error}`
      }
    }
  }

  /**
   * Set authentication token manually (for initial setup)
   */
  setAccessToken(token: string) {
    this.accessToken = token
    console.log("SpotifyAPI: Access token set")
  }

  /**
   * Get authentication status info
   */
  getAuthInfo(): { authenticated: boolean, hasToken: boolean } {
    return {
      authenticated: this.isAuthenticated(),
      hasToken: this.accessToken !== null
    }
  }
}

// Export singleton instance
export const spotifyAPI = new SpotifyAPI()