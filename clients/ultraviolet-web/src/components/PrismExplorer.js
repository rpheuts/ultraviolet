import React, { useState, useEffect } from 'react';

/**
 * PrismExplorer component displays details about a selected prism
 * @param {Object} props - Component props
 * @param {string} props.prismId - ID of the selected prism
 * @param {Object} props.prismDiscovery - PrismDiscovery service
 * @param {Object} props.connectionManager - ConnectionManager service
 */
function PrismExplorer({ prismId, prismDiscovery, connectionManager }) {
  const [spectrum, setSpectrum] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [selectedFrequency, setSelectedFrequency] = useState(null);
  
  // Load spectrum when prism changes
  useEffect(() => {
    if (!prismId || !connectionManager || !connectionManager.isConnected()) {
      setSpectrum(null);
      setSelectedFrequency(null);
      return;
    }
    
    setLoading(true);
    prismDiscovery.getSpectrum(prismId)
      .then(spectrumData => {
        setSpectrum(spectrumData);
        setError(null);
        setSelectedFrequency(null);
      })
      .catch(err => {
        setError(`Failed to load spectrum: ${err.message}`);
        setSpectrum(null);
      })
      .finally(() => {
        setLoading(false);
      });
  }, [prismId, prismDiscovery, connectionManager]); // eslint-disable-line react-hooks/exhaustive-deps
  
  if (!connectionManager || !connectionManager.isConnected()) {
    return (
      <div className="prism-explorer not-connected">
        <p>Not connected to server</p>
        <p>Please check your connection</p>
      </div>
    );
  }
  
  if (!prismId) {
    return (
      <div className="prism-explorer empty">
        <p>Select a prism to explore</p>
      </div>
    );
  }
  
  if (loading) {
    return (
      <div className="prism-explorer loading">
        <p>Loading {prismId}...</p>
      </div>
    );
  }
  
  if (error) {
    return (
      <div className="prism-explorer error">
        <p>{error}</p>
      </div>
    );
  }
  
  if (!spectrum) {
    return null;
  }
  
  return (
    <div className="prism-explorer">
      <h2>{spectrum.name} Prism</h2>
      <p className="description">{spectrum.description}</p>
      
      {spectrum.tags && spectrum.tags.length > 0 && (
        <div className="tags">
          <h3>Tags</h3>
          <div className="tag-list">
            {spectrum.tags.map(tag => (
              <span key={tag} className="tag">{tag}</span>
            ))}
          </div>
        </div>
      )}
      
      <h3>Available Frequencies</h3>
      <ul className="frequency-list">
        {spectrum.wavelengths.map(wavelength => (
          <li 
            key={wavelength.frequency} 
            className={`frequency-item ${selectedFrequency === wavelength.frequency ? 'selected' : ''}`}
            onClick={() => setSelectedFrequency(wavelength.frequency)}
          >
            <h4>{wavelength.frequency}</h4>
            <p>{wavelength.description}</p>
            
            {selectedFrequency === wavelength.frequency && (
              <div className="frequency-details">
                <div className="input-schema">
                  <h5>Input Schema</h5>
                  <pre>{JSON.stringify(wavelength.input, null, 2)}</pre>
                </div>
                <div className="output-schema">
                  <h5>Output Schema</h5>
                  <pre>{JSON.stringify(wavelength.output, null, 2)}</pre>
                </div>
              </div>
            )}
          </li>
        ))}
      </ul>
      
      {spectrum.refractions && spectrum.refractions.length > 0 && (
        <div className="refractions">
          <h3>Refractions</h3>
          <ul className="refraction-list">
            {spectrum.refractions.map(refraction => (
              <li key={refraction.name} className="refraction-item">
                <h4>{refraction.name}</h4>
                <p>Target: {refraction.target}</p>
                <p>Frequency: {refraction.frequency}</p>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

export default PrismExplorer;
