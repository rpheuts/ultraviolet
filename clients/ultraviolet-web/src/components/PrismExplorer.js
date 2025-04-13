import React, { useState, useEffect } from 'react';
import { Box, Divider, Paper, Typography, Tab, Tabs } from '@mui/material';
import FormGenerator from './forms/FormGenerator';
import ResponseRenderer from './renderers/ResponseRenderer';

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
  const [activeTab, setActiveTab] = useState(0);
  const [formSubmitting, setFormSubmitting] = useState(false);
  const [formError, setFormError] = useState(null);
  const [response, setResponse] = useState(null);
  
  // Load spectrum when prism changes
  useEffect(() => {
    if (!prismId || !connectionManager || !connectionManager.isConnected()) {
      setSpectrum(null);
      setSelectedFrequency(null);
      setResponse(null);
      return;
    }
    
    setLoading(true);
    prismDiscovery.getSpectrum(prismId)
      .then(spectrumData => {
        setSpectrum(spectrumData);
        setError(null);
        setSelectedFrequency(null);
        setResponse(null);
      })
      .catch(err => {
        setError(`Failed to load spectrum: ${err.message}`);
        setSpectrum(null);
      })
      .finally(() => {
        setLoading(false);
      });
  }, [prismId, prismDiscovery, connectionManager]); // eslint-disable-line react-hooks/exhaustive-deps
  
  // Handle form submission
  const handleFormSubmit = async (formData) => {
    if (!connectionManager || !prismId || !selectedFrequency) {
      return;
    }
    
    setFormSubmitting(true);
    setFormError(null);
    setResponse(null);
    
    try {
      const result = await connectionManager.sendWavefront(
        prismId,
        selectedFrequency,
        formData
      );
      
      setResponse(result);
      setActiveTab(1); // Switch to response tab
    } catch (err) {
      setFormError(`Error: ${err.message}`);
    } finally {
      setFormSubmitting(false);
    }
  };
  
  // Handle tab change
  const handleTabChange = (event, newValue) => {
    setActiveTab(newValue);
  };
  
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
          </li>
        ))}
      </ul>
      
      {selectedFrequency && (
        <Box sx={{ mt: 4 }}>
          <Paper>
            <Tabs 
              value={activeTab} 
              onChange={handleTabChange}
              variant="fullWidth"
            >
              <Tab label="Form" />
              <Tab label="Response" disabled={!response} />
              <Tab label="Schema" />
            </Tabs>
            
            <Box sx={{ p: 3 }}>
              {activeTab === 0 && (
                <FormGenerator
                  spectrum={spectrum}
                  frequency={selectedFrequency}
                  onSubmit={handleFormSubmit}
                  loading={formSubmitting}
                  error={formError}
                />
              )}
              
              {activeTab === 1 && response && (
                <Box>
                  <Typography variant="h6" gutterBottom>Response</Typography>
                  <Paper sx={{ p: 2, maxHeight: '500px', overflow: 'auto' }}>
                    <ResponseRenderer 
                      data={response} 
                      schema={spectrum.wavelengths.find(w => w.frequency === selectedFrequency)?.output}
                    />
                  </Paper>
                </Box>
              )}
              
              {activeTab === 2 && (
                <Box>
                  <Typography variant="h6" gutterBottom>Input Schema</Typography>
                  <pre style={{ 
                    backgroundColor: '#f5f5f5', 
                    padding: '1rem',
                    borderRadius: '4px',
                    overflow: 'auto'
                  }}>
                    {JSON.stringify(
                      spectrum.wavelengths.find(w => w.frequency === selectedFrequency)?.input, 
                      null, 
                      2
                    )}
                  </pre>
                  
                  <Divider sx={{ my: 3 }} />
                  
                  <Typography variant="h6" gutterBottom>Output Schema</Typography>
                  <pre style={{ 
                    backgroundColor: '#f5f5f5', 
                    padding: '1rem',
                    borderRadius: '4px',
                    overflow: 'auto'
                  }}>
                    {JSON.stringify(
                      spectrum.wavelengths.find(w => w.frequency === selectedFrequency)?.output, 
                      null, 
                      2
                    )}
                  </pre>
                </Box>
              )}
            </Box>
          </Paper>
        </Box>
      )}
      
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
