import React from 'react';

/**
 * Navigation component displays a list of prisms grouped by namespace
 * @param {Object} props - Component props
 * @param {Array} props.prisms - Array of prism info objects
 * @param {boolean} props.loading - Whether prisms are being loaded
 * @param {string} props.selectedPrism - Currently selected prism ID
 * @param {function} props.onSelectPrism - Function to call when a prism is selected
 * @param {boolean} props.connected - Whether the connection is established
 */
function Navigation({ prisms, loading, selectedPrism, onSelectPrism, connected = true }) {
  // Group prisms by namespace
  const groupedPrisms = prisms.reduce((groups, prism) => {
    const { namespace } = prism;
    if (!groups[namespace]) {
      groups[namespace] = [];
    }
    groups[namespace].push(prism);
    return groups;
  }, {});
  
  return (
    <div className="navigation">
      <h2>Prisms</h2>
      
      {!connected ? (
        <div className="not-connected">
          <p>Not connected to server</p>
          <p>Please check your connection</p>
        </div>
      ) : loading ? (
        <div className="loading">Loading prisms...</div>
      ) : (
        <ul className="namespace-list">
          {Object.entries(groupedPrisms).map(([namespace, prismList]) => (
            <li key={namespace} className="namespace-item">
              <h3>{namespace}</h3>
              <ul className="prism-list">
                {prismList.map(prism => (
                  <li 
                    key={`${prism.namespace}:${prism.name}`}
                    className={`prism-item ${selectedPrism === `${prism.namespace}:${prism.name}` ? 'selected' : ''}`}
                    onClick={() => onSelectPrism(`${prism.namespace}:${prism.name}`)}
                  >
                    {prism.name}
                  </li>
                ))}
              </ul>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export default Navigation;
