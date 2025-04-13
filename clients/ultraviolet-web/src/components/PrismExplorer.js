import React, { useState, useEffect } from 'react';
import { 
  Box, 
  Card, 
  CardContent, 
  CircularProgress, 
  Collapse, 
  Divider, 
  Paper, 
  Tab, 
  Tabs, 
  Typography 
} from '@mui/material';
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
        
        // Auto-select the first frequency if available
        if (spectrumData && spectrumData.wavelengths && spectrumData.wavelengths.length > 0) {
          setSelectedFrequency(spectrumData.wavelengths[0].frequency);
        } else {
          setSelectedFrequency(null);
        }
        
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
    } catch (err) {
      setFormError(`Error: ${err.message}`);
    } finally {
      setFormSubmitting(false);
    }
  };
  
  // Handle frequency tab change
  const handleFrequencyChange = (event, newFrequency) => {
    setSelectedFrequency(newFrequency);
    setResponse(null);
    setFormError(null);
  };
  
  // Get the current wavelength definition
  const getCurrentWavelength = () => {
    if (!spectrum || !selectedFrequency) return null;
    return spectrum.wavelengths.find(w => w.frequency === selectedFrequency);
  };
  
  if (!connectionManager || !connectionManager.isConnected()) {
    return (
      <Box className="prism-explorer not-connected" sx={{ p: 3 }}>
        <Typography variant="body1">Not connected to server</Typography>
        <Typography variant="body2">Please check your connection</Typography>
      </Box>
    );
  }
  
  if (!prismId) {
    return (
      <Box className="prism-explorer empty" sx={{ p: 3 }}>
        <Typography variant="body1">Select a prism to explore</Typography>
      </Box>
    );
  }
  
  if (loading) {
    return (
      <Box className="prism-explorer loading" sx={{ p: 3, display: 'flex', justifyContent: 'center' }}>
        <CircularProgress />
        <Typography variant="body1" sx={{ ml: 2 }}>Loading {prismId}...</Typography>
      </Box>
    );
  }
  
  if (error) {
    return (
      <Box className="prism-explorer error" sx={{ p: 3 }}>
        <Typography variant="body1" color="error">{error}</Typography>
      </Box>
    );
  }
  
  if (!spectrum) {
    return null;
  }
  
  return (
    <Box className="prism-explorer" sx={{ p: 2, width: '100%' }}>
      <Paper sx={{ mb: 3, p: 2 }}>
        <Typography variant="h5" gutterBottom>{spectrum.name} Prism</Typography>
        <Typography variant="body2" color="text.secondary" paragraph>
          {spectrum.description}
        </Typography>
        
        {spectrum.tags && spectrum.tags.length > 0 && (
          <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1, mb: 2 }}>
            {spectrum.tags.map(tag => (
              <Box 
                key={tag} 
                sx={{ 
                  bgcolor: 'primary.dark', 
                  color: 'primary.contrastText',
                  px: 1, 
                  py: 0.5, 
                  borderRadius: 1,
                  fontSize: '0.75rem'
                }}
              >
                {tag}
              </Box>
            ))}
          </Box>
        )}
      </Paper>
      
      {/* Frequency tabs */}
      <Tabs
        value={selectedFrequency}
        onChange={handleFrequencyChange}
        variant="scrollable"
        scrollButtons="auto"
        sx={{ 
          borderBottom: 1, 
          borderColor: 'divider',
          mb: 3
        }}
      >
        {spectrum.wavelengths.map(wavelength => (
          <Tab 
            key={wavelength.frequency}
            label={wavelength.frequency}
            value={wavelength.frequency}
            sx={{ textTransform: 'none' }}
          />
        ))}
      </Tabs>
      
      {/* Selected frequency content */}
      {selectedFrequency && (
        <Box>
          {/* Form card */}
          <Card sx={{ mb: 3 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                {getCurrentWavelength()?.description || selectedFrequency}
              </Typography>
              
              <FormGenerator
                spectrum={spectrum}
                frequency={selectedFrequency}
                onSubmit={handleFormSubmit}
                loading={formSubmitting}
                error={formError}
              />
            </CardContent>
          </Card>
          
          {/* Response area */}
          <Collapse in={!!response}>
            {response && (
              <Card>
                <CardContent>
                  <Typography variant="h6" gutterBottom>Response</Typography>
                  <Paper sx={{ p: 2, maxHeight: '500px', overflow: 'auto' }}>
                    <ResponseRenderer 
                      data={response} 
                      schema={getCurrentWavelength()?.output}
                    />
                  </Paper>
                </CardContent>
              </Card>
            )}
          </Collapse>
        </Box>
      )}
    </Box>
  );
}

export default PrismExplorer;
