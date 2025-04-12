/**
 * PrismDiscovery service for discovering and inspecting prisms
 * Uses the system:discovery prism
 */
class PrismDiscovery {
  constructor(connectionManager) {
    this.connectionManager = connectionManager;
    this.prismCache = new Map();
    this.spectrumCache = new Map();
  }

  /**
   * List all available prisms in the system
   * @returns {Promise<Array>} Array of prism info objects
   */
  async listPrisms() {
    try {
      const prisms = await this.connectionManager.sendWavefront(
        'system:discovery',
        'list',
        {}
      );
      
      // Cache the prisms
      prisms.forEach(prism => {
        const key = `${prism.namespace}:${prism.name}`;
        this.prismCache.set(key, prism);
      });
      
      return prisms;
    } catch (error) {
      console.error('Failed to list prisms:', error);
      throw error;
    }
  }

  /**
   * Get the spectrum for a specific prism
   * @param {string} prismId - Prism ID in namespace:name format
   * @returns {Promise<Object>} Spectrum definition
   */
  async getSpectrum(prismId) {
    // Check cache first
    if (this.spectrumCache.has(prismId)) {
      return this.spectrumCache.get(prismId);
    }
    
    try {
      const [spectrum] = await this.connectionManager.sendWavefront(
        'system:discovery',
        'describe',
        { prismId }
      );
      
      // Cache the spectrum
      this.spectrumCache.set(prismId, spectrum);
      
      return spectrum;
    } catch (error) {
      console.error(`Failed to get spectrum for ${prismId}:`, error);
      throw error;
    }
  }

  /**
   * Group prisms by namespace
   * @param {Array} prisms - Array of prism info objects
   * @returns {Object} Object with namespace keys and arrays of prisms as values
   */
  groupByNamespace(prisms) {
    return prisms.reduce((groups, prism) => {
      const { namespace } = prism;
      if (!groups[namespace]) {
        groups[namespace] = [];
      }
      groups[namespace].push(prism);
      return groups;
    }, {});
  }

  /**
   * Clear the cache
   */
  clearCache() {
    this.prismCache.clear();
    this.spectrumCache.clear();
  }
}

export default PrismDiscovery;
